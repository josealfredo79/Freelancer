pub mod contract;
pub mod errors;
pub mod events;
pub mod storage;
pub mod types;

pub use errors::Error;
use contract::{cancel, deposit, dispute, initialize, query_escrow, release, resolve, approve_milestone};
use soroban_sdk::{contract, contractimpl, Address, Env, Vec};

#[contract]
pub struct FreelancerEscrow;

#[contractimpl]
impl FreelancerEscrow {
    pub fn initialize(
        env: Env,
        empresa: Address,
        freelancer: Address,
        arbitro: Address,
        token: Address,
        hitos: Vec<types::Hito>,
    ) -> Result<(), Error> {
        initialize(env, empresa, freelancer, arbitro, token, hitos)
    }

    pub fn deposit(env: Env, monto: i128) -> Result<(), Error> {
        deposit(env, monto)
    }

    pub fn approve_milestone(env: Env, hito_id: u32) -> Result<(), Error> {
        approve_milestone(env, hito_id)
    }

    pub fn release(env: Env, hito_id: u32) -> Result<(), Error> {
        release(env, hito_id)
    }

    pub fn dispute(env: Env) -> Result<(), Error> {
        dispute(env)
    }

    pub fn resolve(env: Env, monto_freelancer: i128, monto_empresa: i128) -> Result<(), Error> {
        resolve(env, monto_freelancer, monto_empresa)
    }

    pub fn cancel(env: Env) -> Result<(), Error> {
        cancel(env)
    }

    pub fn query_escrow(env: Env) -> Result<types::Escrow, Error> {
        query_escrow(env)
    }
}