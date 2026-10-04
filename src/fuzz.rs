//! Deterministic fuzz / property coverage for conversion, BPS (fee/rate),
//! and overflow invariants (issue #74).
//!
//! Uses a fixed-seed LCG so every CI run replays the same inputs. On failure
//! reports retain the seed, iteration, complete raw inputs, and bounded
//! reductions of pure-math counterexamples. Stateful failures retain raw inputs
//! and observations without replaying a new ledger environment.

#![cfg(test)]

extern crate std;

use crate::error::Error;
use crate::math::{
    convert_to_assets, convert_to_shares, mul_div, price_per_share, share_fraction_bps,
};
use crate::types::{BPS_DENOMINATOR, PRICE_SCALE};

/// Canonical seed for the #74 fuzz lane. Keep stable across CI runs.
pub const FUZZ_SEED: u64 = 0x0059_5646_3734_2026; // "YVF74" + year marker
const FUZZ_ITERS: usize = 2_048;

/// Numerical Recipes LCG — tiny, deterministic, no_std-friendly via `u64`.
struct SeedRng {
    state: u64,
    seed: u64,
}

impl SeedRng {
    fn new(seed: u64) -> Self {
        Self { state: seed, seed }
    }

    fn seed(&self) -> u64 {
        self.seed
    }

    fn next_u64(&mut self) -> u64 {
        // Knuth / Numerical Recipes multiplicative LCG.
        self.state = self.state.wrapping_mul(6364136223846793005).wrapping_add(1);
        self.state
    }

    fn next_u128(&mut self) -> u128 {
        let hi = self.next_u64() as u128;
        let lo = self.next_u64() as u128;
        (hi << 64) | lo
    }

    /// Mix of small, mid-range, and near-`u128::MAX` values so overflow and
    /// dust edges are hit without needing millions of iterations.
    fn gen_amount(&mut self) -> u128 {
        match self.next_u64() % 8 {
            0 => 0,
            1 => 1 + (self.next_u64() % 1_000) as u128,
            2 => 1 + (self.next_u64() % 1_000_000) as u128,
            3 => {
                let bits = 1 + (self.next_u64() % 64) as u32;
                self.next_u128() & ((1u128 << bits) - 1)
            }
            4 => u128::MAX - (self.next_u64() % 1_024) as u128,
            5 => (u64::MAX as u128) - (self.next_u64() % 1_024) as u128,
            6 => self.next_u128() >> (self.next_u64() % 64),
            _ => self.next_u128(),
        }
    }

    fn gen_positive(&mut self) -> u128 {
        let v = self.gen_amount();
        if v == 0 {
            1
        } else {
            v
        }
    }

    fn gen_bps(&mut self) -> u128 {
        match self.next_u64() % 5 {
            0 => 1,
            1 => BPS_DENOMINATOR,
            2 => 100,
            3 => 10_000_000,
            _ => 1 + (self.next_u64() % 100_000) as u128,
        }
    }
}

fn fail_ctx(seed: u64, label: &str, detail: &str) -> std::string::String {
    std::format!("fuzz seed=0x{seed:016X} [{label}]: {detail}")
}

const MAX_REDUCTION_CHECKS: usize = 512;

/// Reduce only a reproduced failure, with a fixed predicate-call budget.
/// The caller binds the original input domain and exact failure class.
fn reduce_counterexample<const N: usize>(
    input: [u128; N],
    mut same_failure: impl FnMut([u128; N]) -> bool,
) -> ([u128; N], usize, bool) {
    let mut checks = 1;
    if !same_failure(input) {
        return (input, checks, false);
    }
    let mut reduced = input;
    for coordinate in 0..N {
        let mut step = reduced[coordinate];
        while step > 0 && reduced[coordinate] > 0 && checks < MAX_REDUCTION_CHECKS {
            let mut candidate = reduced;
            candidate[coordinate] = reduced[coordinate].saturating_sub(step);
            checks += 1;
            if same_failure(candidate) {
                reduced = candidate;
            }
            step /= 2;
        }
    }
    (reduced, checks, true)
}

/// Called only while reporting a failure, never on the passing fuzz path.
/// Reduction is deterministic and bounded; it does not claim a global minimum.
fn fail_case<const N: usize>(
    context: (u64, usize),
    label: &str,
    names: [&str; N],
    input: [u128; N],
    observed: &str,
    same_failure: impl FnMut([u128; N]) -> bool,
) -> std::string::String {
    let (seed, iteration) = context;
    let named_input: std::vec::Vec<_> = names.iter().copied().zip(input).collect();
    let (reduced, checks, reproduced) = reduce_counterexample(input, same_failure);
    let raw = std::format!(
        "fuzz seed=0x{seed:016X} i={iteration} [{label}]: inputs={named_input:?}; observed={observed}"
    );
    if !reproduced {
        return std::format!(
            "{raw}; reduced_inputs=unavailable; original_failure_reproduced=false"
        );
    }
    let named_reduced: std::vec::Vec<_> = names.iter().copied().zip(reduced).collect();
    std::format!(
        "{raw}; reduced_inputs={named_reduced:?}; reduction_checks={checks}/{MAX_REDUCTION_CHECKS}; bounded_reduction=true"
    )
}

/// Preserve an unexpected error variant or an unequal successful value.
/// The expected value is recomputed from the candidate input by the caller.
fn same_mismatch(
    original: Result<u128, Error>,
    replay: Result<u128, Error>,
    expected: u128,
) -> bool {
    match (original, replay) {
        (Err(original_error), Err(replayed_error)) => original_error == replayed_error,
        (Ok(_), Ok(value)) => value != expected,
        _ => false,
    }
}

/// mul_div either returns the floored product/divisor or a documented error.
#[test]
fn fuzz_mul_div_floor_or_safe_reject() {
    let mut rng = SeedRng::new(FUZZ_SEED);
    for i in 0..FUZZ_ITERS {
        let a = rng.gen_amount();
        let b = rng.gen_amount();
        let d = rng.gen_amount();
        match mul_div(a, b, d) {
            Err(Error::DivisionByZero) => {
                assert_eq!(
                    d,
                    0,
                    "{}",
                    fail_case(
                        (rng.seed(), i),
                        "mul_div/div0",
                        ["a", "b", "d"],
                        [a, b, d],
                        "err=DivisionByZero",
                        |[a, b, d]| d != 0 && mul_div(a, b, d) == Err(Error::DivisionByZero)
                    )
                );
            }
            Err(Error::MathOverflow) => {
                assert!(
                    a.checked_mul(b).is_none(),
                    "{}",
                    fail_case(
                        (rng.seed(), i),
                        "mul_div/overflow-false-positive",
                        ["a", "b", "d"],
                        [a, b, d],
                        "err=MathOverflow",
                        |[a, b, d]| a.checked_mul(b).is_some()
                            && mul_div(a, b, d) == Err(Error::MathOverflow)
                    )
                );
            }
            Ok(q) => {
                let product = a.checked_mul(b).unwrap_or_else(|| {
                    panic!(
                        "{}",
                        fail_case(
                            (rng.seed(), i),
                            "mul_div/ok-but-overflow",
                            ["a", "b", "d"],
                            [a, b, d],
                            &std::format!("q={q}"),
                            |[a, b, d]| a.checked_mul(b).is_none() && mul_div(a, b, d).is_ok()
                        )
                    )
                });
                assert_ne!(
                    d,
                    0,
                    "{}",
                    fail_case(
                        (rng.seed(), i),
                        "mul_div/ok-with-zero-denom",
                        ["a", "b", "d"],
                        [a, b, d],
                        &std::format!("q={q} product={product}"),
                        |[a, b, d]| d == 0
                            && a.checked_mul(b).is_some()
                            && mul_div(a, b, d).is_ok()
                    )
                );
                assert_eq!(
                    q,
                    product / d,
                    "{}",
                    fail_case(
                        (rng.seed(), i),
                        "mul_div/floor",
                        ["a", "b", "d"],
                        [a, b, d],
                        &std::format!("q={q} product={product}"),
                        |[a, b, d]| d > 0
                            && matches!(
                                (a.checked_mul(b), mul_div(a, b, d)),
                                (Some(product), Ok(value)) if value != product / d
                            )
                    )
                );
            }
            Err(other) => {
                panic!(
                    "{}",
                    fail_case(
                        (rng.seed(), i),
                        "mul_div/unexpected-err",
                        ["a", "b", "d"],
                        [a, b, d],
                        &std::format!("err={other:?}"),
                        |[a, b, d]| mul_div(a, b, d) == Err(other)
                    )
                );
            }
        }
    }
}
/// Round-trip conservation: converting assets→shares→assets never mints value.
#[test]
fn fuzz_convert_round_trip_conserves_value() {
    let mut rng = SeedRng::new(FUZZ_SEED ^ 0xC011_C011);
    for i in 0..FUZZ_ITERS {
        let total_shares = rng.gen_positive();
        let total_assets = rng.gen_positive();
        let assets = rng.gen_amount();

        let shares = match convert_to_shares(assets, total_shares, total_assets) {
            Ok(s) => s,
            Err(Error::MathOverflow) => continue,
            Err(e) => panic!(
                "{}",
                fail_case(
                    (rng.seed(), i),
                    "shares/unexpected",
                    ["assets", "total_shares", "total_assets"],
                    [assets, total_shares, total_assets],
                    &std::format!("err={e:?}"),
                    |[assets, ts, ta]| ts > 0
                        && ta > 0
                        && convert_to_shares(assets, ts, ta) == Err(e)
                )
            ),
        };

        let back = match convert_to_assets(shares, total_shares, total_assets) {
            Ok(a) => a,
            Err(Error::MathOverflow) => continue,
            Err(e) => panic!(
                "{}",
                fail_case(
                    (rng.seed(), i),
                    "assets/unexpected",
                    ["assets", "total_shares", "total_assets"],
                    [assets, total_shares, total_assets],
                    &std::format!("shares={shares} err={e:?}"),
                    |[assets, ts, ta]| ts > 0
                        && ta > 0
                        && matches!(convert_to_shares(assets, ts, ta),
                            Ok(shares) if convert_to_assets(shares, ts, ta) == Err(e))
                )
            ),
        };

        assert!(
            back <= assets,
            "{}",
            fail_case(
                (rng.seed(), i),
                "conservation/assets-roundtrip",
                ["assets", "total_shares", "total_assets"],
                [assets, total_shares, total_assets],
                &std::format!("shares={shares} back={back}"),
                |[assets, ts, ta]| ts > 0
                    && ta > 0
                    && matches!(convert_to_shares(assets, ts, ta)
                        .and_then(|shares| convert_to_assets(shares, ts, ta)),
                        Ok(back) if back > assets)
            )
        );
    }
}
/// Symmetric conservation: shares→assets→shares never inflates share count.
#[test]
fn fuzz_convert_shares_round_trip_conserves() {
    let mut rng = SeedRng::new(FUZZ_SEED ^ 0x5441_5245);
    for i in 0..FUZZ_ITERS {
        let total_shares = rng.gen_positive();
        let total_assets = rng.gen_positive();
        let shares = rng.gen_amount();

        let assets = match convert_to_assets(shares, total_shares, total_assets) {
            Ok(a) => a,
            Err(Error::MathOverflow) => continue,
            Err(e) => panic!(
                "{}",
                fail_case(
                    (rng.seed(), i),
                    "assets/unexpected",
                    ["shares", "total_shares", "total_assets"],
                    [shares, total_shares, total_assets],
                    &std::format!("err={e:?}"),
                    |[shares, ts, ta]| ts > 0
                        && ta > 0
                        && convert_to_assets(shares, ts, ta) == Err(e)
                )
            ),
        };

        let back = match convert_to_shares(assets, total_shares, total_assets) {
            Ok(s) => s,
            Err(Error::MathOverflow) => continue,
            Err(e) => panic!(
                "{}",
                fail_case(
                    (rng.seed(), i),
                    "shares/unexpected",
                    ["shares", "total_shares", "total_assets"],
                    [shares, total_shares, total_assets],
                    &std::format!("assets={assets} err={e:?}"),
                    |[shares, ts, ta]| ts > 0
                        && ta > 0
                        && matches!(convert_to_assets(shares, ts, ta),
                            Ok(assets) if convert_to_shares(assets, ts, ta) == Err(e))
                )
            ),
        };

        assert!(
            back <= shares,
            "{}",
            fail_case(
                (rng.seed(), i),
                "conservation/shares-roundtrip",
                ["shares", "total_shares", "total_assets"],
                [shares, total_shares, total_assets],
                &std::format!("assets={assets} back={back}"),
                |[shares, ts, ta]| ts > 0
                    && ta > 0
                    && matches!(convert_to_assets(shares, ts, ta)
                        .and_then(|assets| convert_to_shares(assets, ts, ta)),
                        Ok(back) if back > shares)
            )
        );
    }
}
/// Monotonicity: more assets (same vault state) never mint fewer shares.
#[test]
fn fuzz_convert_to_shares_monotonic() {
    let mut rng = SeedRng::new(FUZZ_SEED ^ 0x4D4F_4E4F);
    for i in 0..FUZZ_ITERS {
        let total_shares = rng.gen_positive();
        let total_assets = rng.gen_positive();
        let a1 = rng.gen_amount();
        let a2 = rng.gen_amount();
        let (lo, hi) = if a1 <= a2 { (a1, a2) } else { (a2, a1) };

        let s_lo = match convert_to_shares(lo, total_shares, total_assets) {
            Ok(s) => s,
            Err(Error::MathOverflow) => continue,
            Err(e) => panic!(
                "{}",
                fail_case(
                    (rng.seed(), i),
                    "mono/lo",
                    ["lo_assets", "hi_assets", "total_shares", "total_assets"],
                    [lo, hi, total_shares, total_assets],
                    &std::format!("err={e:?}"),
                    |[lo, hi, ts, ta]| lo <= hi
                        && ts > 0
                        && ta > 0
                        && convert_to_shares(lo, ts, ta) == Err(e)
                )
            ),
        };
        let s_hi = match convert_to_shares(hi, total_shares, total_assets) {
            Ok(s) => s,
            Err(Error::MathOverflow) => continue,
            Err(e) => panic!(
                "{}",
                fail_case(
                    (rng.seed(), i),
                    "mono/hi",
                    ["lo_assets", "hi_assets", "total_shares", "total_assets"],
                    [lo, hi, total_shares, total_assets],
                    &std::format!("s_lo={s_lo} err={e:?}"),
                    |[lo, hi, ts, ta]| lo <= hi
                        && ts > 0
                        && ta > 0
                        && convert_to_shares(lo, ts, ta).is_ok()
                        && convert_to_shares(hi, ts, ta) == Err(e)
                )
            ),
        };

        assert!(
            s_lo <= s_hi,
            "{}",
            fail_case(
                (rng.seed(), i),
                "monotonicity/shares",
                ["lo_assets", "hi_assets", "total_shares", "total_assets"],
                [lo, hi, total_shares, total_assets],
                &std::format!("s_lo={s_lo} s_hi={s_hi}"),
                |[lo, hi, ts, ta]| lo <= hi
                    && ts > 0
                    && ta > 0
                    && matches!((convert_to_shares(lo, ts, ta), convert_to_shares(hi, ts, ta)),
                        (Ok(low), Ok(high)) if low > high)
            )
        );
    }
}
/// Monotonicity for redeem: more shares never redeem fewer assets.
#[test]
fn fuzz_convert_to_assets_monotonic() {
    let mut rng = SeedRng::new(FUZZ_SEED ^ 0xA55E_A55E);
    for i in 0..FUZZ_ITERS {
        let total_shares = rng.gen_positive();
        let total_assets = rng.gen_positive();
        let s1 = rng.gen_amount();
        let s2 = rng.gen_amount();
        let (lo, hi) = if s1 <= s2 { (s1, s2) } else { (s2, s1) };

        let a_lo = match convert_to_assets(lo, total_shares, total_assets) {
            Ok(a) => a,
            Err(Error::MathOverflow) => continue,
            Err(e) => panic!(
                "{}",
                fail_case(
                    (rng.seed(), i),
                    "mono/lo",
                    ["lo_shares", "hi_shares", "total_shares", "total_assets"],
                    [lo, hi, total_shares, total_assets],
                    &std::format!("err={e:?}"),
                    |[lo, hi, ts, ta]| lo <= hi
                        && ts > 0
                        && ta > 0
                        && convert_to_assets(lo, ts, ta) == Err(e)
                )
            ),
        };
        let a_hi = match convert_to_assets(hi, total_shares, total_assets) {
            Ok(a) => a,
            Err(Error::MathOverflow) => continue,
            Err(e) => panic!(
                "{}",
                fail_case(
                    (rng.seed(), i),
                    "mono/hi",
                    ["lo_shares", "hi_shares", "total_shares", "total_assets"],
                    [lo, hi, total_shares, total_assets],
                    &std::format!("a_lo={a_lo} err={e:?}"),
                    |[lo, hi, ts, ta]| lo <= hi
                        && ts > 0
                        && ta > 0
                        && convert_to_assets(lo, ts, ta).is_ok()
                        && convert_to_assets(hi, ts, ta) == Err(e)
                )
            ),
        };

        assert!(
            a_lo <= a_hi,
            "{}",
            fail_case(
                (rng.seed(), i),
                "monotonicity/assets",
                ["lo_shares", "hi_shares", "total_shares", "total_assets"],
                [lo, hi, total_shares, total_assets],
                &std::format!("a_lo={a_lo} a_hi={a_hi}"),
                |[lo, hi, ts, ta]| lo <= hi
                    && ts > 0
                    && ta > 0
                    && matches!((convert_to_assets(lo, ts, ta), convert_to_assets(hi, ts, ta)),
                        (Ok(low), Ok(high)) if low > high)
            )
        );
    }
}
/// BPS / "fee" path: a holder of ≤ total shares never reports > `bps` (100%).
#[test]
fn fuzz_share_fraction_bps_bounded() {
    let mut rng = SeedRng::new(FUZZ_SEED ^ 0xFEE5_FEE5);
    for i in 0..FUZZ_ITERS {
        let total_shares = rng.gen_positive();
        let shares = rng.gen_amount() % (total_shares.saturating_add(1)); // shares ∈ [0, total]
        let bps = rng.gen_bps();

        let frac = match share_fraction_bps(shares, total_shares, bps) {
            Ok(f) => f,
            Err(Error::MathOverflow) => continue,
            Err(e) => panic!(
                "{}",
                fail_case(
                    (rng.seed(), i),
                    "bps/unexpected",
                    ["shares", "total_shares", "bps"],
                    [shares, total_shares, bps],
                    &std::format!("err={e:?}"),
                    |[shares, ts, bps]| ts > 0
                        && shares <= ts
                        && bps > 0
                        && share_fraction_bps(shares, ts, bps) == Err(e)
                )
            ),
        };

        assert!(
            frac <= bps,
            "{}",
            fail_case(
                (rng.seed(), i),
                "bounded-fee/bps",
                ["shares", "total_shares", "bps"],
                [shares, total_shares, bps],
                &std::format!("frac={frac}"),
                |[shares, ts, bps]| ts > 0
                    && shares <= ts
                    && bps > 0
                    && matches!(share_fraction_bps(shares, ts, bps), Ok(frac) if frac > bps)
            )
        );
    }
}
/// Rate / price path: higher total_assets (fixed shares) never lowers pps.
#[test]
fn fuzz_price_per_share_monotonic_in_assets() {
    let mut rng = SeedRng::new(FUZZ_SEED ^ 0x5241_5445);
    for i in 0..FUZZ_ITERS {
        let total_shares = rng.gen_positive();
        let a1 = rng.gen_amount();
        let a2 = rng.gen_amount();
        let (lo, hi) = if a1 <= a2 { (a1, a2) } else { (a2, a1) };

        let p_lo = match price_per_share(total_shares, lo, PRICE_SCALE) {
            Ok(p) => p,
            Err(Error::MathOverflow) => continue,
            Err(e) => panic!(
                "{}",
                fail_case(
                    (rng.seed(), i),
                    "pps/lo",
                    ["lo_assets", "hi_assets", "total_shares"],
                    [lo, hi, total_shares],
                    &std::format!("scale={PRICE_SCALE} err={e:?}"),
                    |[lo, hi, ts]| lo <= hi
                        && ts > 0
                        && price_per_share(ts, lo, PRICE_SCALE) == Err(e)
                )
            ),
        };
        let p_hi = match price_per_share(total_shares, hi, PRICE_SCALE) {
            Ok(p) => p,
            Err(Error::MathOverflow) => continue,
            Err(e) => panic!(
                "{}",
                fail_case(
                    (rng.seed(), i),
                    "pps/hi",
                    ["lo_assets", "hi_assets", "total_shares"],
                    [lo, hi, total_shares],
                    &std::format!("p_lo={p_lo} scale={PRICE_SCALE} err={e:?}"),
                    |[lo, hi, ts]| lo <= hi
                        && ts > 0
                        && price_per_share(ts, lo, PRICE_SCALE).is_ok()
                        && price_per_share(ts, hi, PRICE_SCALE) == Err(e)
                )
            ),
        };

        assert!(
            p_lo <= p_hi,
            "{}",
            fail_case(
                (rng.seed(), i),
                "rate/pps-monotonic",
                ["lo_assets", "hi_assets", "total_shares"],
                [lo, hi, total_shares],
                &std::format!("p_lo={p_lo} p_hi={p_hi} scale={PRICE_SCALE}"),
                |[lo, hi, ts]| lo <= hi
                    && ts > 0
                    && matches!((price_per_share(ts, lo, PRICE_SCALE),
                        price_per_share(ts, hi, PRICE_SCALE)),
                        (Ok(low), Ok(high)) if low > high)
            )
        );
    }
}
/// Empty-vault bootstrap: first depositor gets 1:1 shares; redeem of zero
/// total shares yields zero assets.
#[test]
fn fuzz_empty_vault_bootstrap_and_zero_totals() {
    let mut rng = SeedRng::new(FUZZ_SEED ^ 0xE7F7_E7F7);
    for i in 0..256 {
        let assets = rng.gen_amount();
        let empty_shares = convert_to_shares(assets, 0, 0);
        assert_eq!(
            empty_shares,
            Ok(assets),
            "{}",
            fail_case(
                (rng.seed(), i),
                "empty/shares",
                ["assets"],
                [assets],
                &std::format!("total_shares=0 total_assets=0 result={empty_shares:?}"),
                |[assets]| same_mismatch(empty_shares, convert_to_shares(assets, 0, 0), assets)
            )
        );
        let bootstrap_assets = rng.gen_positive();
        let zero_ts_shares = convert_to_shares(assets, 0, bootstrap_assets);
        assert_eq!(
            zero_ts_shares,
            Ok(assets),
            "{}",
            fail_case(
                (rng.seed(), i),
                "empty/shares-zero-ts",
                ["assets", "total_assets"],
                [assets, bootstrap_assets],
                &std::format!("total_shares=0 result={zero_ts_shares:?}"),
                |[assets, ta]| ta > 0
                    && same_mismatch(zero_ts_shares, convert_to_shares(assets, 0, ta), assets)
            )
        );
        let shares = rng.gen_amount();
        let redeem_assets = rng.gen_amount();
        let redeemed = convert_to_assets(shares, 0, redeem_assets);
        assert_eq!(
            redeemed,
            Ok(0),
            "{}",
            fail_case(
                (rng.seed(), i),
                "empty/assets",
                ["shares", "total_assets"],
                [shares, redeem_assets],
                &std::format!("total_shares=0 result={redeemed:?}"),
                |[shares, ta]| same_mismatch(redeemed, convert_to_assets(shares, 0, ta), 0)
            )
        );
        let fraction = share_fraction_bps(shares, 0, BPS_DENOMINATOR);
        assert_eq!(
            fraction,
            Ok(0),
            "{}",
            fail_case(
                (rng.seed(), i),
                "empty/bps",
                ["shares"],
                [shares],
                &std::format!("total_shares=0 bps={BPS_DENOMINATOR} result={fraction:?}"),
                |[shares]| same_mismatch(
                    fraction,
                    share_fraction_bps(shares, 0, BPS_DENOMINATOR),
                    0
                )
            )
        );
        let price_assets = rng.gen_amount();
        let price = price_per_share(0, price_assets, PRICE_SCALE);
        assert_eq!(
            price,
            Ok(PRICE_SCALE),
            "{}",
            fail_case(
                (rng.seed(), i),
                "empty/pps",
                ["total_assets"],
                [price_assets],
                &std::format!("total_shares=0 scale={PRICE_SCALE} result={price:?}"),
                |[ta]| same_mismatch(price, price_per_share(0, ta, PRICE_SCALE), PRICE_SCALE)
            )
        );
    }
}
/// Regression fixtures: known overflow / floor / bound edges pinned by seed.
#[test]
fn fuzz_regression_fixtures_overflow_and_bounds() {
    // Intermediate product overflows u128.
    assert_eq!(
        mul_div(u128::MAX, 2, 1),
        Err(Error::MathOverflow),
        "{}",
        fail_ctx(
            FUZZ_SEED,
            "fixture/mul_div-overflow",
            &std::format!("i=0 a={} b=2 d=1", u128::MAX)
        )
    );
    assert_eq!(
        mul_div(1, 1, 0),
        Err(Error::DivisionByZero),
        "{}",
        fail_ctx(FUZZ_SEED, "fixture/mul_div-div0", "i=1 a=1 b=1 d=0")
    );
    // Flooring: 1*1/2 == 0.
    assert_eq!(
        mul_div(1, 1, 2),
        Ok(0),
        "{}",
        fail_ctx(FUZZ_SEED, "fixture/mul_div-floor-dust", "i=2 a=1 b=1 d=2")
    );
    // Full ownership reports exactly bps.
    assert_eq!(
        share_fraction_bps(1_000, 1_000, BPS_DENOMINATOR),
        Ok(BPS_DENOMINATOR),
        "{}",
        fail_ctx(
            FUZZ_SEED,
            "fixture/full-ownership",
            &std::format!("i=3 shares=1000 total_shares=1000 bps={BPS_DENOMINATOR}")
        )
    );
    // Partial ownership never exceeds bps.
    assert_eq!(
        share_fraction_bps(1, 3, BPS_DENOMINATOR),
        Ok(3333),
        "{}",
        fail_ctx(
            FUZZ_SEED,
            "fixture/partial-ownership",
            &std::format!("i=4 shares=1 total_shares=3 bps={BPS_DENOMINATOR}")
        )
    );
    // convert overflow path (assets * total_shares overflows).
    assert_eq!(
        convert_to_shares(u128::MAX, u128::MAX, 1),
        Err(Error::MathOverflow),
        "{}",
        fail_ctx(
            FUZZ_SEED,
            "fixture/convert-to-shares-overflow",
            &std::format!(
                "i=5 assets={} total_shares={} total_assets=1",
                u128::MAX,
                u128::MAX
            )
        )
    );
    assert_eq!(
        convert_to_assets(u128::MAX, 1, u128::MAX),
        Err(Error::MathOverflow),
        "{}",
        fail_ctx(
            FUZZ_SEED,
            "fixture/convert-to-assets-overflow",
            &std::format!(
                "i=6 shares={} total_shares=1 total_assets={}",
                u128::MAX,
                u128::MAX
            )
        )
    );

    // Exercise the reporter/reducer here without another suite or ledger Env.
    let input = [u128::MAX, 2, 1];
    let same_overflow =
        |[a, b, d]: [u128; 3]| d > 0 && mul_div(a, b, d) == Err(Error::MathOverflow);
    let reduced = reduce_counterexample(input, same_overflow);
    assert!(reduced.2 && reduced.0 != input && same_overflow(reduced.0));
    assert!(reduced.1 <= MAX_REDUCTION_CHECKS);
    assert_eq!(reduced, reduce_counterexample(input, same_overflow));
    assert!(!reduce_counterexample([1, 1, 1], same_overflow).2);
    let report = fail_case(
        (FUZZ_SEED, 7),
        "reporter/overflow",
        ["a", "b", "d"],
        input,
        "err=MathOverflow",
        same_overflow,
    );
    assert!(report.contains(&std::format!("seed=0x{FUZZ_SEED:016X} i=7")));
    assert!(report.contains(&std::format!("{}", u128::MAX)));
    assert!(report.contains("inputs=[") && report.contains("reduced_inputs=["));
    assert!(report.contains("\"d\", 1") && report.contains("bounded_reduction=true"));
}
/// End-to-end vault invariant under randomized deposit / yield / withdraw:
/// a full redeem never returns more assets than deposited + accrued yield
/// attributed to the position (vault keeps dust via floor rounding).
#[test]
fn fuzz_vault_deposit_withdraw_conserves_under_yield() {
    use crate::{YieldVault, YieldVaultClient};
    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::token::{StellarAssetClient, TokenClient};
    use soroban_sdk::{Address, Env};

    let mut rng = SeedRng::new(FUZZ_SEED ^ 0xAA11_7001);
    // Keep iteration count small: each pass spins up a Soroban Env and
    // writes a ledger snapshot. Eight seeded cases still hit deposit,
    // yield-funded withdraw, and dust edges without bloating CI artifacts.
    for i in 0..8 {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let issued = env.register_stellar_asset_contract_v2(admin.clone());
        let token_address = issued.address();
        let token = TokenClient::new(&env, &token_address);
        let token_admin = StellarAssetClient::new(&env, &token_address);
        let vault_address = env.register(YieldVault, ());
        let vault = YieldVaultClient::new(&env, &vault_address);
        vault.initialize(&admin, &token_address);

        let user = Address::generate(&env);
        // Bound amounts so token i128 and dust guards stay in range.
        let deposit: u128 = 1_000 + (rng.next_u64() % 5_000_000) as u128;
        let yield_amt: u128 = (rng.next_u64() % 2_000_000) as u128;
        // Captured by the test harness if a native operation panics before an assertion.
        std::eprintln!(
            "fuzz seed=0x{:016X} i={i} [vault/context]: deposit={deposit} yield={yield_amt}",
            rng.seed()
        );
        token_admin.mint(&user, &(deposit as i128));

        let shares = vault.deposit(&user, &deposit);
        assert!(
            shares > 0,
            "{}",
            fail_ctx(
                rng.seed(),
                "vault/zero-shares",
                &std::format!("i={i} deposit={deposit} yield={yield_amt} shares={shares}")
            )
        );

        if yield_amt > 0 {
            // Mock yield only bumps accounting; fund the vault token balance
            // so the eventual withdraw transfer can succeed (same pattern as
            // `test_deposit_yield_withdraw_round_trip`).
            token_admin.mint(&vault_address, &(yield_amt as i128));
            vault.accrue_yield(&yield_amt);
        }

        let before = token.balance(&user) as u128;
        let redeemed = vault.withdraw(&user, &shares);
        let after = token.balance(&user) as u128;

        assert_eq!(
            after,
            before + redeemed,
            "{}",
            fail_ctx(
                rng.seed(), "vault/token-delta",
                &std::format!(
                    "i={i} deposit={deposit} yield={yield_amt} shares={shares} redeemed={redeemed} before={before} after={after}"
                )
            )
        );
        // Solo depositor receives all yield; floor rounding still cannot
        // credit more than deposit + yield.
        assert!(
            redeemed <= deposit.saturating_add(yield_amt),
            "{}",
            fail_ctx(
                rng.seed(),
                "vault/conservation",
                &std::format!(
                    "i={i} deposit={deposit} yield={yield_amt} shares={shares} redeemed={redeemed}"
                )
            )
        );
        let residual_shares = vault.balance_of(&user);
        assert_eq!(
            residual_shares,
            0,
            "{}",
            fail_ctx(
                rng.seed(), "vault/residual-shares",
                &std::format!(
                    "i={i} deposit={deposit} yield={yield_amt} shares={shares} redeemed={redeemed} before={before} after={after} residual_shares={residual_shares}"
                )
            )
        );
    }
}
