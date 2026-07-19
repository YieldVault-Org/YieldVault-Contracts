#!/usr/bin/env bash
# tests/test_verify_wasm_hash.sh — Integration / unit tests for
# scripts/verify_wasm_hash.sh using only bash built-ins plus standard
# coreutils.  No external test framework is required.
#
# Run from the repository root:
#   bash tests/test_verify_wasm_hash.sh
#
# Exit codes:
#   0  All tests passed.
#   1  One or more tests failed.

set -euo pipefail

SCRIPT="scripts/verify_wasm_hash.sh"
PASS=0
FAIL=0

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

RED='\033[0;31m'
GREEN='\033[0;32m'
BOLD='\033[1m'
RESET='\033[0m'

pass() { echo -e "${GREEN}[PASS]${RESET} $1"; ((PASS++)); }
fail() { echo -e "${RED}[FAIL]${RESET} $1"; ((FAIL++)); }

# Run the script and assert the exit code.
# Usage: assert_exit <expected_code> [args...]
assert_exit() {
    local expected="$1"; shift
    local actual
    bash "$SCRIPT" "$@" &>/dev/null
    actual=$?
    if [[ "$actual" -eq "$expected" ]]; then
        pass "exit code $expected for: $*"
    else
        fail "expected exit $expected, got $actual for: $*"
    fi
}

# Run the script and assert stdout/stderr contains a pattern.
# Usage: assert_output_contains <pattern> [args...]
assert_output_contains() {
    local pattern="$1"; shift
    local combined
    combined=$(bash "$SCRIPT" "$@" 2>&1 || true)
    if echo "$combined" | grep -qE "$pattern"; then
        pass "output contains '$pattern' for: $*"
    else
        fail "expected '$pattern' in output for: $*"
        echo "  Actual output: $combined"
    fi
}

# ---------------------------------------------------------------------------
# Temporary fixtures
# ---------------------------------------------------------------------------

TMP_DIR=$(mktemp -d)
trap 'rm -rf "$TMP_DIR"' EXIT

# A predictable fake WASM file (32 bytes of zeros).
FAKE_WASM="$TMP_DIR/fake.wasm"
dd if=/dev/zero bs=32 count=1 of="$FAKE_WASM" 2>/dev/null

FAKE_HASH=$(sha256sum "$FAKE_WASM" 2>/dev/null | awk '{print $1}' \
            || shasum -a 256 "$FAKE_WASM" | awk '{print $1}')

# A second fake WASM that produces a different hash.
FAKE_WASM2="$TMP_DIR/fake2.wasm"
printf '\x01%.0s' {1..32} > "$FAKE_WASM2"

FAKE_HASH2=$(sha256sum "$FAKE_WASM2" 2>/dev/null | awk '{print $1}' \
             || shasum -a 256 "$FAKE_WASM2" | awk '{print $1}')

# Stub stellar CLI that echoes a configurable wasm_hash JSON fragment.
# Placed earlier on PATH so the real binary (if present) is shadowed.
STUB_DIR="$TMP_DIR/bin"
mkdir -p "$STUB_DIR"

make_stellar_stub() {
    local hash="$1"
    cat > "$STUB_DIR/stellar" <<STUB
#!/usr/bin/env bash
echo '{"wasm_hash": "${hash}"}'
STUB
    chmod +x "$STUB_DIR/stellar"
}

export PATH="$STUB_DIR:$PATH"

# ---------------------------------------------------------------------------
# Test: missing CONTRACT_ID → exit 2
# ---------------------------------------------------------------------------
echo ""
echo "${BOLD}=== Pre-condition checks ===${RESET}"

assert_exit 2 # (no args → usage)

# ---------------------------------------------------------------------------
# Test: help flag → exit 2 (usage)
# ---------------------------------------------------------------------------
assert_exit 2 --help

# ---------------------------------------------------------------------------
# Test: missing WASM file → exit 2
# ---------------------------------------------------------------------------
assert_exit 2 CTEST --network testnet --wasm "$TMP_DIR/nonexistent.wasm"
assert_output_contains "not found" CTEST --network testnet --wasm "$TMP_DIR/nonexistent.wasm"

# ---------------------------------------------------------------------------
# Test: unknown option → exit 2
# ---------------------------------------------------------------------------
assert_exit 2 CTEST --unknown-option

# ---------------------------------------------------------------------------
# Test: matching hashes → exit 0
# ---------------------------------------------------------------------------
echo ""
echo "${BOLD}=== Hash comparison ===${RESET}"

# Stub stellar to return the same hash as our fake WASM.
make_stellar_stub "$FAKE_HASH"

assert_exit 0 CTEST --network testnet --wasm "$FAKE_WASM"
assert_output_contains "MATCHES" CTEST --network testnet --wasm "$FAKE_WASM"

# ---------------------------------------------------------------------------
# Test: mismatched hashes → exit 1
# ---------------------------------------------------------------------------

# Stub returns FAKE_HASH but we provide FAKE_WASM2 (different hash).
# Stub still returns FAKE_HASH so local != on-chain.
assert_exit 1 CTEST --network testnet --wasm "$FAKE_WASM2"
assert_output_contains "MISMATCH" CTEST --network testnet --wasm "$FAKE_WASM2"

# ---------------------------------------------------------------------------
# Test: case-insensitive hash comparison
# ---------------------------------------------------------------------------

# Stub returns uppercase hash; local computation returns lowercase — should match.
UPPER_HASH=$(echo "$FAKE_HASH" | tr '[:lower:]' '[:upper:]')
make_stellar_stub "$UPPER_HASH"

assert_exit 0 CTEST --network testnet --wasm "$FAKE_WASM"
assert_output_contains "MATCHES" CTEST --network testnet --wasm "$FAKE_WASM"

# ---------------------------------------------------------------------------
# Test: --optimized flag resolves to .optimized.wasm path (file not present)
# ---------------------------------------------------------------------------
echo ""
echo "${BOLD}=== --optimized flag ===${RESET}"

# Without --wasm, the default path is derived; .optimized.wasm won't exist.
assert_exit 2 CTEST --optimized

# ---------------------------------------------------------------------------
# Test: stellar CLI not on PATH → exit 2
# ---------------------------------------------------------------------------
echo ""
echo "${BOLD}=== Missing stellar CLI ===${RESET}"

# Temporarily remove our stub from PATH
SAVED_PATH="$PATH"
export PATH="${PATH/$STUB_DIR:/}"   # strip the stub directory

assert_exit 2 CTEST --wasm "$FAKE_WASM"
assert_output_contains "stellar.*not found|not found.*stellar" CTEST --wasm "$FAKE_WASM"

export PATH="$SAVED_PATH"

# ---------------------------------------------------------------------------
# Test: stellar CLI fails (non-zero exit, no usable output) → exit 2
# ---------------------------------------------------------------------------
echo ""
echo "${BOLD}=== stellar CLI error handling ===${RESET}"

cat > "$STUB_DIR/stellar" <<'STUB'
#!/usr/bin/env bash
echo "Error: contract not found" >&2
exit 1
STUB
chmod +x "$STUB_DIR/stellar"

# The script must exit 2 (pre-condition) when it cannot parse the hash.
assert_exit 2 CTEST --wasm "$FAKE_WASM"
assert_output_contains "parse|Raw output" CTEST --wasm "$FAKE_WASM"

# ---------------------------------------------------------------------------
# Summary
# ---------------------------------------------------------------------------
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
TOTAL=$((PASS + FAIL))
echo "Results: $PASS passed, $FAIL failed out of $TOTAL tests."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

[[ "$FAIL" -eq 0 ]]
