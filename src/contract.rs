use soroban_sdk::{Address, Env, Vec};

use crate::errors::Error;
use crate::events::{
    emit_cancelled, emit_deposit, emit_dispute_opened, emit_dispute_resolved, emit_initialized,
    emit_milestone_approved, emit_payment_released,
};
use crate::storage::{read_escrow, write_escrow};
use crate::types::{Escrow, Estado, Hito};

fn extend_ttl(env: &Env) {
    let key = soroban_sdk::Symbol::new(env, "escrow");
    let threshold: u32 = 100_000;
    let extend_to: u32 = 200_000;
    env.storage().persistent().extend_ttl(&key, threshold, extend_to);
}

pub fn initialize(
    env: Env,
    empresa: Address,
    freelancer: Address,
    arbitro: Address,
    token: Address,
    hitos: Vec<Hito>,
) -> Result<(), Error> {
    empresa.require_auth();

    if read_escrow(&env).is_ok() {
        return Err(Error::YaInicializado.into());
    }

    if hitos.len() == 0 {
        return Err(Error::HitosVacios.into());
    }

    let mut monto_total: i128 = 0;
    for i in 0..hitos.len() {
        let hito = hitos.get(i).unwrap();
        monto_total = monto_total.checked_add(hito.monto).unwrap();
    }

    let created_at: u64 = env.ledger().timestamp();

    let escrow = Escrow {
        empresa,
        freelancer,
        arbitro,
        token,
        monto_total,
        monto_pagado: 0,
        hitos,
        estado: Estado::Activo,
        created_at,
    };

    write_escrow(&env, &escrow);
    emit_initialized(
        &env,
        escrow.empresa.clone(),
        escrow.freelancer.clone(),
        escrow.arbitro.clone(),
    );

    Ok(())
}

pub fn deposit(env: Env, monto: i128) -> Result<(), Error> {
    let mut escrow = read_escrow(&env)?;

    if escrow.estado != Estado::Activo {
        return Err(Error::EstadoInvalido.into());
    }

    escrow.empresa.require_auth();

    if monto != escrow.monto_total {
        return Err(Error::MontoIncorrecto.into());
    }

    let token_client = soroban_sdk::token::Client::new(&env, &escrow.token);
    token_client.transfer(
        &escrow.empresa,
        &env.current_contract_address(),
        &monto,
    );

    escrow.estado = Estado::Depositado;
    write_escrow(&env, &escrow);

    extend_ttl(&env);
    emit_deposit(&env, monto);

    Ok(())
}

pub fn approve_milestone(env: Env, hito_id: u32) -> Result<(), Error> {
    let mut escrow = read_escrow(&env)?;

    if escrow.estado != Estado::Depositado {
        return Err(Error::NoDepositado.into());
    }

    escrow.empresa.require_auth();

    let mut hito_encontrado = false;
    let mut index: u32 = 0;
    for i in 0..escrow.hitos.len() {
        let hito = escrow.hitos.get(i).unwrap();
        if hito.id == hito_id {
            if hito.aprobado {
                return Err(Error::HitoYaAprobado.into());
            }
            hito_encontrado = true;
            index = i;
            break;
        }
    }

    if !hito_encontrado {
        return Err(Error::HitoNoEncontrado.into());
    }

    let mut hito = escrow.hitos.get(index).unwrap();
    hito.completado = true;
    hito.aprobado = true;
    escrow.hitos.set(index, hito);

    write_escrow(&env, &escrow);

    extend_ttl(&env);
    emit_milestone_approved(&env, hito_id);

    Ok(())
}

pub fn release(env: Env, hito_id: u32) -> Result<(), Error> {
    let mut escrow = read_escrow(&env)?;

    if escrow.estado != Estado::Depositado {
        return Err(Error::NoDepositado.into());
    }

    escrow.empresa.require_auth();

    let mut hito_encontrado = false;
    let mut index: u32 = 0;
    for i in 0..escrow.hitos.len() {
        let hito = escrow.hitos.get(i).unwrap();
        if hito.id == hito_id {
            if !hito.aprobado {
                return Err(Error::HitoNoAprobado.into());
            }
            hito_encontrado = true;
            index = i;
            break;
        }
    }

    if !hito_encontrado {
        return Err(Error::HitoNoEncontrado.into());
    }

    let hito = escrow.hitos.get(index).unwrap();
    let monto_hito = hito.monto;

    let token_client = soroban_sdk::token::Client::new(&env, &escrow.token);
    token_client.transfer(
        &env.current_contract_address(),
        &escrow.freelancer,
        &monto_hito,
    );

    escrow.monto_pagado = escrow.monto_pagado.checked_add(monto_hito).unwrap();

    if escrow.monto_pagado == escrow.monto_total {
        escrow.estado = Estado::Completado;
    }

    write_escrow(&env, &escrow);

    extend_ttl(&env);
    emit_payment_released(&env, hito_id, monto_hito);

    Ok(())
}

pub fn dispute(env: Env) -> Result<(), Error> {
    let escrow = read_escrow(&env)?;

    if escrow.estado != Estado::Depositado {
        return Err(Error::NoDepositado.into());
    }

    let called_empresa = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        escrow.empresa.require_auth();
    }));
    let called_freelancer = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        escrow.freelancer.require_auth();
    }));

    if called_empresa.is_err() && called_freelancer.is_err() {
        return Err(Error::NoAutorizado.into());
    }

    let mut new_escrow = escrow.clone();
    new_escrow.estado = Estado::Disputado;
    write_escrow(&env, &new_escrow);

    emit_dispute_opened(&env, escrow.freelancer);

    Ok(())
}

pub fn resolve(env: Env, monto_freelancer: i128, monto_empresa: i128) -> Result<(), Error> {
    let mut escrow = read_escrow(&env)?;

    if escrow.estado != Estado::Disputado {
        return Err(Error::EstadoInvalido.into());
    }

    escrow.arbitro.require_auth();

    let fundos_disponibles = escrow
        .monto_total
        .checked_sub(escrow.monto_pagado)
        .unwrap();

    if monto_freelancer
        .checked_add(monto_empresa)
        .unwrap()
        > fundos_disponibles
    {
        return Err(Error::FondosInsuficientes.into());
    }

    let token_client = soroban_sdk::token::Client::new(&env, &escrow.token);

    if monto_freelancer > 0 {
        token_client.transfer(
            &env.current_contract_address(),
            &escrow.freelancer,
            &monto_freelancer,
        );
    }

    if monto_empresa > 0 {
        token_client.transfer(
            &env.current_contract_address(),
            &escrow.empresa,
            &monto_empresa,
        );
    }

    escrow.estado = Estado::Completado;
    write_escrow(&env, &escrow);

    emit_dispute_resolved(&env, monto_freelancer, monto_empresa);

    Ok(())
}

pub fn cancel(env: Env) -> Result<(), Error> {
    let mut escrow = read_escrow(&env)?;

    if escrow.estado != Estado::Activo && escrow.estado != Estado::Depositado {
        return Err(Error::EstadoInvalido.into());
    }

    escrow.empresa.require_auth();

    if escrow.estado == Estado::Depositado {
        for i in 0..escrow.hitos.len() {
            let hito = escrow.hitos.get(i).unwrap();
            if hito.aprobado {
                return Err(Error::NoPuedeCancelar.into());
            }
        }

        let fondos = escrow.monto_total;
        let token_client = soroban_sdk::token::Client::new(&env, &escrow.token);
        token_client.transfer(
            &env.current_contract_address(),
            &escrow.empresa,
            &fondos,
        );
    }

    escrow.estado = Estado::Cancelado;
    write_escrow(&env, &escrow);

    emit_cancelled(&env);

    Ok(())
}

pub fn query_escrow(env: Env) -> Result<Escrow, Error> {
    read_escrow(&env)
}