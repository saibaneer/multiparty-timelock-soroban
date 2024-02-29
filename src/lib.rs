#![no_std]

use core::panic;

use soroban_sdk::{contract, contractimpl, contracttype, token, Address, Env, Vec};

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Init,
    Balance,
}

#[derive(Clone)]
#[contracttype]
pub enum TimeBoundKind {
    Before,
    After,
}

#[derive(Clone)]
#[contracttype]
pub struct TimeBound {
    pub kind: TimeBoundKind,
    pub timestamp: u64,
}

#[derive(Clone)]
#[contracttype]
/// Represents a claimable balance that can be distributed among multiple beneficiaries.
pub struct ClaimableBalance {
    pub token: Address,
    pub amount_per_beneficiary: i128,
    pub total_amount: i128,
    pub beneficiaries: Vec<Address>,
    pub claimed_beneficiaries: Vec<Address>,
    pub time_bound: TimeBound,
}

#[contract]
pub struct MultiPartyClaimableBalanceContract;

fn check_time_bound(env: &Env, time_bound: &TimeBound) -> bool {
    let ledger_timestamp = env.ledger().timestamp();

    match time_bound.kind {
        TimeBoundKind::Before => ledger_timestamp <= time_bound.timestamp,
        TimeBoundKind::After => ledger_timestamp >= time_bound.timestamp,
    }
}
fn is_initialized(env: &Env) -> bool {
    env.storage().instance().has(&DataKey::Init)
}

#[contractimpl]
impl MultiPartyClaimableBalanceContract {
    pub fn deposit(
        env: &Env,
        from: Address,
        token: Address,
        amount_per_beneficiary: i128,
        beneficiaries: Vec<Address>,
        timebound: TimeBound,
    ) {
        if amount_per_beneficiary < 0 {
            panic!("amount must be positive");
        }

        if beneficiaries.len() > 10 {
            panic!("too many beneficiaries");
        }

        if is_initialized(&env) {
            panic!("contract has been already initialized");
        }

        from.require_auth();

        let total_amount = &amount_per_beneficiary * beneficiaries.len() as i128;
        let empty_claimed: Vec<Address> = Vec::new(&env);
        token::Client::new(&env, &token).transfer(
            &from,
            &env.current_contract_address(),
            &total_amount,
        );
        env.storage().instance().set(
            &DataKey::Balance,
            &ClaimableBalance {
                token,
                amount_per_beneficiary,
                total_amount,
                beneficiaries,
                claimed_beneficiaries: empty_claimed,
                time_bound: timebound,
            },
        );
        env.storage().instance().set(&DataKey::Init, &true);
    }

    pub fn claim(env: &Env, beneficiary: Address) {
        beneficiary.require_auth();
        let mut claimable_balance: ClaimableBalance =
            env.storage().instance().get(&DataKey::Balance).unwrap();
        
      
        if !claimable_balance.beneficiaries.contains(&beneficiary) {
            panic!("beneficiary not in list");
        }
        if !check_time_bound(&env, &claimable_balance.time_bound) {
            panic!("time bound not satisfied");
        }
        if claimable_balance
            .claimed_beneficiaries
            .contains(&beneficiary)
        {
            panic!("beneficiary already claimed");
        }
        claimable_balance
            .claimed_beneficiaries
            .push_back(beneficiary.clone());
        token::Client::new(&env, &claimable_balance.token).transfer(
            &env.current_contract_address(),
            &beneficiary,
            &claimable_balance.amount_per_beneficiary,
        );

        if &claimable_balance.claimed_beneficiaries.len() == &claimable_balance.beneficiaries.len()
        {
            env.storage().instance().remove(&DataKey::Balance);
        } else {
            env.storage().instance().set(
                &DataKey::Balance,
                &ClaimableBalance {
                    token: claimable_balance.token,
                    amount_per_beneficiary: claimable_balance.amount_per_beneficiary,
                    total_amount: claimable_balance.total_amount
                        - claimable_balance.amount_per_beneficiary,
                    beneficiaries: claimable_balance.beneficiaries,
                    claimed_beneficiaries: claimable_balance.claimed_beneficiaries,
                    time_bound: claimable_balance.time_bound,
                },
            );
        }
        
    }
}

mod test;
