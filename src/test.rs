#![cfg(test)]

extern crate std;

use soroban_sdk::testutils::Address as _;
use soroban_sdk::token::{StellarAssetClient, TokenClient};
use soroban_sdk::{Address, Env};

use crate::{YieldVault, YieldVaultClient};

/// Bundles together the objects needed to exercise the vault in a test.
struct VaultTest<'a> {
    env: Env,
    admin: Address,
    token: TokenClient<'a>,
    token_admin: StellarAssetClient<'a>,
    vault: YieldVaultClient<'a>,
}

impl<'a> VaultTest<'a> {
    /// Sets up an initialized vault backed by a fresh Stellar Asset Contract
    /// token, with all authorizations mocked.
    fn setup() -> Self {
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

        VaultTest {
            env,
            admin,
            token,
            token_admin,
            vault,
        }
    }

    /// Mints `amount` of the underlying token to `user`.
    fn mint(&self, user: &Address, amount: i128) {
        self.token_admin.mint(user, &amount);
    }
}

#[test]
fn test_initialize_sets_admin_and_token() {
    let t = VaultTest::setup();
    assert_eq!(t.vault.get_admin(), t.admin);
    assert_eq!(t.vault.get_token(), t.token.address);
}
