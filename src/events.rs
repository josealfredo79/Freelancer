use soroban_sdk::{Env, Symbol, Address};

pub fn emit_deposit(env: &Env, monto: i128) {
    let topics = (Symbol::new(env, "deposit"),);
    env.events().publish(topics, monto);
}

pub fn emit_milestone_approved(env: &Env, hito_id: u32) {
    let topics = (Symbol::new(env, "milestone_approved"),);
    env.events().publish(topics, hito_id);
}

pub fn emit_payment_released(env: &Env, hito_id: u32, monto: i128) {
    let topics = (Symbol::new(env, "payment_released"),);
    env.events().publish(topics, (hito_id, monto));
}

pub fn emit_dispute_opened(env: &Env, por: Address) {
    let topics = (Symbol::new(env, "dispute_opened"),);
    env.events().publish(topics, por);
}

pub fn emit_dispute_resolved(env: &Env, monto_freelancer: i128, monto_empresa: i128) {
    let topics = (Symbol::new(env, "dispute_resolved"),);
    env.events().publish(topics, (monto_freelancer, monto_empresa));
}

pub fn emit_cancelled(env: &Env) {
    let topics = (Symbol::new(env, "cancelled"),);
    env.events().publish(topics, ());
}

pub fn emit_initialized(env: &Env, empresa: Address, freelancer: Address, arbitro: Address) {
    let topics = (Symbol::new(env, "initialized"),);
    env.events().publish(topics, (empresa, freelancer, arbitro));
}