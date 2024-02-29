#![cfg(test)]
extern crate std;

use core::ops::Add;

use super::*;
use soroban_sdk::testutils::{Address as _, AuthorizedFunction, AuthorizedInvocation, Ledger};
use soroban_sdk::{symbol_short, token, vec, Address, Env, IntoVal};
use token::Client as TokenClient;
use token::StellarAssetClient as TokenAdminClient;

fn create_token_contract<'a>(env: &Env, admin: &Address) -> (TokenClient<'a>, TokenAdminClient<'a>) {
    let contract_address: Address = env.register_stellar_asset_contract(admin.clone());
    (TokenClient::new(env, &contract_address), TokenAdminClient::new(env, &contract_address))
}

fn create_claimable_contract<'a>(env: &Env) -> MultiPartyClaimableBalanceContractClient<'a> {
    MultiPartyClaimableBalanceContractClient::new(env, &env.register_contract(None, MultiPartyClaimableBalanceContract {}))
}


struct ClaimableBalanceTest <'a> {
    env: Env,
    deposit_address: Address,
    claim_address: [Address; 3],
    token: TokenClient<'a>,
    contract: MultiPartyClaimableBalanceContractClient<'a>
}

impl ClaimableBalanceTest <'_>{

    fn setup() -> Self {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().with_mut(|li| {li.timestamp = 12345});
        let deposit_address = Address::generate(&env);
        let claim_addresses = [Address::generate(&env), Address::generate(&env), Address::generate(&env)];
        let token_admin = Address::generate(&env);

        let (token, token_admin_client) = create_token_contract(&env, &token_admin);
        token_admin_client.mint(&deposit_address, &1000);

        let contract = create_claimable_contract(&env);
        ClaimableBalanceTest {
            env,
            deposit_address,
            claim_address: claim_addresses,
            token,
            contract
        }
    }
}

#[test]
fn test_deposit_and_claim() {
    let test = ClaimableBalanceTest::setup();
    test.contract.deposit(
        &test.deposit_address,
        &test.token.address,
        &100,
        &vec![
            &test.env,
            test.claim_address[0].clone(),
            test.claim_address[1].clone(),
        ],
        &TimeBound {
            kind: TimeBoundKind::Before,
            timestamp: 12346,
        },
    );

    assert_eq!(
        test.env.auths(),
        [(
            test.deposit_address.clone(),
            AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    test.contract.address.clone(),
                    symbol_short!("deposit"),
                    (
                        test.deposit_address.clone(),
                        test.token.address.clone(),
                        100_i128,
                        vec![
                            &test.env,
                            test.claim_address[0].clone(),
                            test.claim_address[1].clone()
                        ],
                        TimeBound {
                            kind: TimeBoundKind::Before,
                            timestamp: 12346,
                        },
                    )
                        .into_val(&test.env),
                )),
                sub_invocations: std::vec![AuthorizedInvocation {
                    function: AuthorizedFunction::Contract((
                        test.token.address.clone(),
                        symbol_short!("transfer"),
                        (
                            test.deposit_address.clone(),
                            &test.contract.address,
                            200_i128,
                        )
                            .into_val(&test.env),
                    )),
                    sub_invocations: std::vec![]
                }]
            }
        ),]
    );

    assert_eq!(test.token.balance(&test.deposit_address), 800);
    assert_eq!(test.token.balance(&test.contract.address), 200);
    assert_eq!(test.token.balance(&test.claim_address[1]), 0);

    test.contract.claim(&test.claim_address[1]);
    assert_eq!(
        test.env.auths(),
        [(
            test.claim_address[1].clone(),
            AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    test.contract.address.clone(),
                    symbol_short!("claim"),
                    (test.claim_address[1].clone(),).into_val(&test.env),
                )),
                sub_invocations: std::vec![]
            }
        ),]
    );

    assert_eq!(test.token.balance(&test.deposit_address), 800);
    assert_eq!(test.token.balance(&test.contract.address), 100);
    assert_eq!(test.token.balance(&test.claim_address[1]), 100);
}

#[test]
fn test_deposit_and_double_claim_pass() {
    let test = ClaimableBalanceTest::setup();
    test.contract.deposit(
        &test.deposit_address,
        &test.token.address,
        &100,
        &vec![
            &test.env,
            test.claim_address[0].clone(),
            test.claim_address[1].clone(),
        ],
        &TimeBound {
            kind: TimeBoundKind::Before,
            timestamp: 12346,
        },
    );

    assert_eq!(
        test.env.auths(),
        [(
            test.deposit_address.clone(),
            AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    test.contract.address.clone(),
                    symbol_short!("deposit"),
                    (
                        test.deposit_address.clone(),
                        test.token.address.clone(),
                        100_i128,
                        vec![
                            &test.env,
                            test.claim_address[0].clone(),
                            test.claim_address[1].clone()
                        ],
                        TimeBound {
                            kind: TimeBoundKind::Before,
                            timestamp: 12346,
                        },
                    )
                        .into_val(&test.env),
                )),
                sub_invocations: std::vec![AuthorizedInvocation {
                    function: AuthorizedFunction::Contract((
                        test.token.address.clone(),
                        symbol_short!("transfer"),
                        (
                            test.deposit_address.clone(),
                            &test.contract.address,
                            200_i128,
                        )
                            .into_val(&test.env),
                    )),
                    sub_invocations: std::vec![]
                }]
            }
        ),]
    );

    assert_eq!(test.token.balance(&test.deposit_address), 800);
    assert_eq!(test.token.balance(&test.contract.address), 200);
    assert_eq!(test.token.balance(&test.claim_address[1]), 0);

    test.contract.claim(&test.claim_address[1]);
    assert_eq!(
        test.env.auths(),
        [(
            test.claim_address[1].clone(),
            AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    test.contract.address.clone(),
                    symbol_short!("claim"),
                    (test.claim_address[1].clone(),).into_val(&test.env),
                )),
                sub_invocations: std::vec![]
            }
        ),]
    );

    assert_eq!(test.token.balance(&test.deposit_address), 800);
    assert_eq!(test.token.balance(&test.contract.address), 100);
    assert_eq!(test.token.balance(&test.claim_address[1]), 100);

    test.contract.claim(&test.claim_address[0]);
    assert_eq!(test.token.balance(&test.contract.address), 0);
    assert_eq!(test.token.balance(&test.claim_address[0]), 100);

}


#[test]
#[should_panic(expected = "already initialized")]
fn test_double_deposit_fail() {
    let test = ClaimableBalanceTest::setup();
    test.contract.deposit(
        &test.deposit_address, &test.token.address, &1, &vec![&test.env, test.claim_address[0].clone()], &TimeBound{kind: TimeBoundKind::Before, timestamp: 12346});
    
        test.contract.deposit(
            &test.deposit_address, &test.token.address, &1, &vec![&test.env, test.claim_address[0].clone()], &TimeBound{kind: TimeBoundKind::Before, timestamp: 12346});
}


#[test]
#[should_panic(expected = "beneficiary not in list")]
fn test_rogue_claimant_fail() {
    let test = ClaimableBalanceTest::setup();
    test.contract.deposit(
        &test.deposit_address, &test.token.address, &100, &vec![&test.env, test.claim_address[0].clone()], &TimeBound{kind: TimeBoundKind::Before, timestamp: 12346});

    test.contract.claim(&test.claim_address[2]);
}

#[test]
#[should_panic(expected = "time bound not satisfied")]
fn test_bad_time_fail() {
    let test = ClaimableBalanceTest::setup();
    test.contract.deposit(
        &test.deposit_address, &test.token.address, &100, &vec![&test.env, test.claim_address[0].clone()], &TimeBound{kind: TimeBoundKind::After, timestamp: 12346});

    test.contract.claim(&test.claim_address[0]);
}

#[test]
#[should_panic(expected = "beneficiary already claimed")]
fn test_double_claim_fail() {
    let test = ClaimableBalanceTest::setup();
    test.contract.deposit(
        &test.deposit_address, &test.token.address, &100, &vec![&test.env, test.claim_address[0].clone(), test.claim_address[1].clone()], &TimeBound{kind: TimeBoundKind::Before, timestamp: 12346});

    test.contract.claim(&test.claim_address[0]);
    assert_eq!(test.token.balance(&test.claim_address[0]), 100);
    test.contract.claim(&test.claim_address[0]);
}



#[test]
#[should_panic(expected = "amount must be positive")]
fn test_negative_deposit_fail() {
    let test = ClaimableBalanceTest::setup();
    test.contract.deposit(
        &test.deposit_address, &test.token.address, &-1, &vec![&test.env, test.claim_address[0].clone()], &TimeBound{kind: TimeBoundKind::Before, timestamp: 12346});
}