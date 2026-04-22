use soroban_sdk::{Env, Symbol};

use crate::errors::Error;
use crate::types::Escrow;

const ESCROW_KEY: &str = "escrow";

pub fn read_escrow(env: &Env) -> Result<Escrow, Error> {
    let key = Symbol::new(env, ESCROW_KEY);
    env.storage()
        .persistent()
        .get::<Symbol, Escrow>(&key)
        .ok_or(Error::NoInicializado.into())
}

pub fn write_escrow(env: &Env, escrow: &Escrow) {
    let key = Symbol::new(env, ESCROW_KEY);
    env.storage().persistent().set(&key, escrow);
}