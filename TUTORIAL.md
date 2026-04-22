# Tutorial: Contrato FreelancerEscrow en Stellar Soroban

Este tutorial te guía paso a paso para crear un contrato inteligente de escrow para freelancers en Stellar Soroban.

## Requisitos Previos

- Rust instalado (1.75+)
- Conocimientos básicos de Rust
- Conocimiento de contratos inteligentes en Stellar

---

## Paso 1: Estructura del Proyecto

Crea la siguiente estructura de archivos:

freelancers/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── contract.rs
│   ├── types.rs
│   ├── errors.rs
│   ├── storage.rs
│   └── events.rs
└── tests/
    └── integration.rs
```

---

## Paso 2: Configuración Cargo.toml

Crea el archivo `Cargo.toml`:

```toml
[package]
name = "freelancer_escrow"
version = "0.1.0"
edition = "2021"

[dependencies]
soroban-sdk = { version = "21.0.0", features = ["alloc"] }

[dev-dependencies]
soroban-sdk = { version = "21.0.0", features = ["testutils"] }

[profile.release]
opt-level = "z"
overflow-checks = true
debug = 0
strip = "symbols"
panic = "abort"
codegen-units = 1
lto = true
```

---

## Paso 3: Definir Tipos de Datos (types.rs)

El contrato necesita estructuras para representar el estado del escrow:

```rust
use soroban_sdk::{contracttype, Address, Symbol, Vec};

#[contracttype]
#[derive(Clone, PartialEq, Debug)]
pub enum Estado {
    Activo,      // Inicializado, esperando depósito
    Depositado,  // Fondos recibidos, listo para trabajar
    Completado,  // Todo pagado
    Disputado,   // En disputa
    Cancelado,   // Cancelado
}

#[contracttype]
#[derive(Clone)]
pub struct Hito {
    pub id: u32,
    pub descripcion: Symbol,
    pub monto: i128,
    pub completado: bool,
    pub aprobado: bool,
}

#[contracttype]
#[derive(Clone)]
pub struct Escrow {
    pub empresa: Address,
    pub freelancer: Address,
    pub arbitro: Address,
    pub token: Address,
    pub monto_total: i128,
    pub monto_pagado: i128,
    pub hitos: Vec<Hito>,
    pub estado: Estado,
    pub created_at: u64,
}
```

**Explicación:**
- `Estado`: enum que representa los estados posibles del contrato
- `Hito`: Cada hito (entregable) del proyecto
- `Escrow`: El contrato completo con toda la información

---

## Paso 4: Definir Errores (errors.rs)

Definimos errores con códigos específicos:

```rust
#![allow(non_camel_case_types)]
#![allow(dead_code)]

use soroban_sdk::contracterror;

#[contracterror]
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(u32)]
pub enum Error {
    YaInicializado = 1,    // Ya existe un escrow
    NoInicializado = 2,     // No hay escrow configurado
    HitosVacios = 3,        // No se proporcionaron hitos
    
    EstadoInvalido = 10,     // Estado no permite operación
    NoDepositado = 11,      // Necesita estado Depositado
    
    HitoNoEncontrado = 20,  // Hito no existe
    HitoYaAprobado = 21,     // Ya está aprobado
    HitoYaPagado = 22,      // Ya fue pagado
    HitoNoAprobado = 23,     // No está aprobado
    
    MontoIncorrecto = 30,     // Monto no coincide
    FondosInsuficientes = 31, // No hay suficientes fondos
    
    NoAutorizado = 40,        // No tiene autorización
    NoPuedeCancelar = 50,  // No se puede cancelar
}
```

---

## Paso 5: Storage (storage.rs)

Funciones para persistir el escrow:

```rust
use soroban_sdk::{Env, Symbol};
use crate::errors::Error;
use crate::types::Escrow;

const ESCROW_KEY: &str = "escrow";

// Leer el escrow del storage
pub fn read_escrow(env: &Env) -> Result<Escrow, Error> {
    let key = Symbol::new(env, ESCROW_KEY);
    env.storage()
        .persistent()
        .get::<Symbol, Escrow>(&key)
        .ok_or(Error::NoInicializado.into())
}

// Escribir el escrow en storage
pub fn write_escrow(env: &Env, escrow: &Escrow) {
    let key = Symbol::new(env, ESCROW_KEY);
    env.storage().persistent().set(&key, escrow);
}
```

---

## Paso 6: Eventos (events.rs)

Emite eventos para seguir las operaciones:

```rust
use soroban_sdk::{Env, Symbol, Address};

pub fn emit_initialized(env: &Env, empresa: Address, freelancer: Address, arbitro: Address) {
    let topics = (Symbol::new(env, "initialized"),);
    env.events().publish(topics, (empresa, freelancer, arbitro));
}

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
```

---

## Paso 7: Lógica del Contrato (contract.rs)

Esta es la parte principal con toda la lógica:

```rust
use soroban_sdk::{Address, Env, Vec};
use crate::errors::Error;
use crate::events::*;
use crate::storage::{read_escrow, write_escrow};
use crate::types::{Escrow, Estado, Hito};

// Extender TTL del storage
fn extend_ttl(env: &Env) {
    let key = soroban_sdk::Symbol::new(env, "escrow");
    env.storage().persistent().extend_ttl(&key, 100_000, 200_000);
}

// Inicializar el escrow
pub fn initialize(
    env: Env,
    empresa: Address,
    freelancer: Address,
    arbitro: Address,
    token: Address,
    hitos: Vec<Hito>,
) -> Result<(), Error> {
    empresa.require_auth();  // La empresa debe autorizada

    // Verificar que no exista un escrow previo
    if read_escrow(&env).is_ok() {
        return Err(Error::YaInicializado.into());
    }

    // Verificar que hay hitos
    if hitos.len() == 0 {
        return Err(Error::HitosVacios.into());
    }

    // Calcular monto total
    let mut monto_total: i128 = 0;
    for i in 0..hitos.len() {
        let hito = hitos.get(i).unwrap();
        monto_total = monto_total.checked_add(hito.monto).unwrap();
    }

    // Crear el escrow
    let escrow = Escrow {
        empresa,
        freelancer,
        arbitro,
        token,
        monto_total,
        monto_pagado: 0,
        hitos,
        estado: Estado::Activo,
        created_at: env.ledger().timestamp(),
    };

    write_escrow(&env, &escrow);
    emit_initialized(&env, escrow.empresa.clone(), escrow.freelancer.clone(), escrow.arbitro.clone());

    Ok(())
}

// Depositar fondos
pub fn deposit(env: Env, monto: i128) -> Result<(), Error> {
    let mut escrow = read_escrow(&env)?;

    // Solo puede depositar si está activo
    if escrow.estado != Estado::Activo {
        return Err(Error::EstadoInvalido.into());
    }

    escrow.empresa.require_auth();

    // El monto debe ser exacto
    if monto != escrow.monto_total {
        return Err(Error::MontoIncorrecto.into());
    }

    // Transferir tokens
    let token_client = soroban_sdk::token::Client::new(&env, &escrow.token);
    token_client.transfer(&escrow.empresa, &env.current_contract_address(), &monto);

    // Actualizar estado
    escrow.estado = Estado::Depositado;
    write_escrow(&env, &escrow);

    extend_ttl(&env);
    emit_deposit(&env, monto);

    Ok(())
}

// Aprobar hito
pub fn approve_milestone(env: Env, hito_id: u32) -> Result<(), Error> {
    let mut escrow = read_escrow(&env)?;

    if escrow.estado != Estado::Depositado {
        return Err(Error::NoDepositado.into());
    }

    escrow.empresa.require_auth();

    // Buscar el hito
    let mut encontrado = false;
    let mut index: u32 = 0;
    for i in 0..escrow.hitos.len() {
        let hito = escrow.hitos.get(i).unwrap();
        if hito.id == hito_id {
            if hito.aprobado {
                return Err(Error::HitoYaAprobado.into());
            }
            encontrado = true;
            index = i;
            break;
        }
    }

    if !encontrado {
        return Err(Error::HitoNoEncontrado.into());
    }

    // Aprobar hito
    let mut hito = escrow.hitos.get(index).unwrap();
    hito.completado = true;
    hito.aprobado = true;
    escrow.hitos.set(index, hito);

    write_escrow(&env, &escrow);
    extend_ttl(&env);
    emit_milestone_approved(&env, hito_id);

    Ok(())
}

// Liberar pago del hito
pub fn release(env: Env, hito_id: u32) -> Result<(), Error> {
    let mut escrow = read_escrow(&env)?;

    if escrow.estado != Estado::Depositado {
        return Err(Error::NoDepositado.into());
    }

    escrow.empresa.require_auth();  // Solo la empresa puede liberar

    // Buscar hito aprobado
    let mut encontrado = false;
    let mut index: u32 = 0;
    for i in 0..escrow.hitos.len() {
        let hito = escrow.hitos.get(i).unwrap();
        if hito.id == hito_id {
            if !hito.aprobado {
                return Err(Error::HitoNoAprobado.into());
            }
            encontrado = true;
            index = i;
            break;
        }
    }

    if !encontrado {
        return Err(Error::HitoNoEncontrado.into());
    }

    let hito = escrow.hitos.get(index).unwrap();
    let monto_hito = hito.monto;

    // Pagar al freelancer
    let token_client = soroban_sdk::token::Client::new(&env, &escrow.token);
    token_client.transfer(&env.current_contract_address(), &escrow.freelancer, &monto_hito);

    // Actualizar monto pagado
    escrow.monto_pagado = escrow.monto_pagado.checked_add(monto_hito).unwrap();

    // Si está todo pagado, marcar completado
    if escrow.monto_pagado == escrow.monto_total {
        escrow.estado = Estado::Completado;
    }

    write_escrow(&env, &escrow);
    extend_ttl(&env);
    emit_payment_released(&env, hito_id, monto_hito);

    Ok(())
}

// Abrir disputa
pub fn dispute(env: Env) -> Result<(), Error> {
    let escrow = read_escrow(&env)?;

    if escrow.estado != Estado::Depositado {
        return Err(Error::NoDepositado.into());
    }

    // Empresa o freelancer pueden abrir disputa
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

// Resolver disputa
pub fn resolve(env: Env, monto_freelancer: i128, monto_empresa: i128) -> Result<(), Error> {
    let mut escrow = read_escrow(&env)?;

    if escrow.estado != Estado::Disputado {
        return Err(Error::EstadoInvalido.into());
    }

    escrow.arbitro.require_auth();  // Solo el arbitro puede resolver

    let fondos_disponibles = escrow.monto_total - escrow.monto_pagado;

    if monto_freelancer + monto_empresa > fondos_disponibles {
        return Err(Error::FondosInsuficientes.into());
    }

    let token_client = soroban_sdk::token::Client::new(&env, &escrow.token);

    if monto_freelancer > 0 {
        token_client.transfer(&env.current_contract_address(), &escrow.freelancer, &monto_freelancer);
    }

    if monto_empresa > 0 {
        token_client.transfer(&env.current_contract_address(), &escrow.empresa, &monto_empresa);
    }

    escrow.estado = Estado::Completado;
    write_escrow(&env, &escrow);

    emit_dispute_resolved(&env, monto_freelancer, monto_empresa);

    Ok(())
}

// Cancelar contrato
pub fn cancel(env: Env) -> Result<(), Error> {
    let mut escrow = read_escrow(&env)?;

    if escrow.estado != Estado::Activo && escrow.estado != Estado::Depositado {
        return Err(Error::EstadoInvalido.into());
    }

    escrow.empresa.require_auth();

    // Si hay depósito, devolver fondos
    if escrow.estado == Estado::Depositado {
        // Verificar que ningún hito esté aprobado
        for i in 0..escrow.hitos.len() {
            let hito = escrow.hitos.get(i).unwrap();
            if hito.aprobado {
                return Err(Error::NoPuedeCancelar.into());
            }
        }

        let token_client = soroban_sdk::token::Client::new(&env, &escrow.token);
        token_client.transfer(&env.current_contract_address(), &escrow.empresa, &escrow.monto_total);
    }

    escrow.estado = Estado::Cancelado;
    write_escrow(&env, &escrow);

    emit_cancelled(&env);

    Ok(())
}

// Consultar estado del escrow
pub fn query_escrow(env: Env) -> Result<Escrow, Error> {
    read_escrow(&env)
}
```

---

## Paso 8: Contrato Principal (lib.rs)

Expone las funciones del contrato:

```rust
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
```

---

## Paso 9: Tests de Integración

```rust
#![cfg(test)]

mod test {
    use soroban_sdk::{
        testutils::Address as _, token, vec, Address, Env, Symbol,
    };
    use freelancer_escrow::{FreelancerEscrowClient, types::{Estado, Hito}};

    fn create_hito(env: &Env, id: u32, desc: &str, monto: i128) -> Hito {
        Hito {
            id,
            descripcion: Symbol::new(env, desc),
            monto,
            completado: false,
            aprobado: false,
        }
    }

    #[test]
    fn test_single_milestone_flow() {
        let env = Env::default();
        env.mock_all_auths();

        let empresa = Address::generate(&env);
        let freelancer = Address::generate(&env);
        let arbitro = Address::generate(&env);
        
        let token_addr = env.register_stellar_asset_contract_v2(empresa.clone());
        let token_client = token::StellarAssetClient::new(&env, &token_addr.address());
        token_client.mint(&empresa, &1000);
        let token = token_addr.address();

        let contract_id = env.register_contract(None, freelancer_escrow::FreelancerEscrow);
        let client = FreelancerEscrowClient::new(&env, &contract_id);

        let hitos = vec![&env, create_hito(&env, 1, "desarrollo", 1000)];
        client.initialize(&empresa, &freelancer, &arbitro, &token, &hitos);

        client.deposit(&1000);
        client.approve_milestone(&1);
        client.release(&1);

        let escrow = client.query_escrow();
        assert!(escrow.estado == Estado::Completado);
    }

    #[test]
    fn test_multiple_milestones() {
        let env = Env::default();
        env.mock_all_auths();

        let empresa = Address::generate(&env);
        let freelancer = Address::generate(&env);
        let arbitro = Address::generate(&env);
        
        let token_addr = env.register_stellar_asset_contract_v2(empresa.clone());
        let token_client = token::StellarAssetClient::new(&env, &token_addr.address());
        token_client.mint(&empresa, &1000);
        let token = token_addr.address();

        let contract_id = env.register_contract(None, freelancer_escrow::FreelancerEscrow);
        let client = FreelancerEscrowClient::new(&env, &contract_id);

        let hitos = vec![
            &env,
            create_hito(&env, 1, "fase1", 500),
            create_hito(&env, 2, "fase2", 500),
        ];
        client.initialize(&empresa, &freelancer, &arbitro, &token, &hitos);

        client.deposit(&1000);
        client.approve_milestone(&1);
        client.release(&1);
        client.approve_milestone(&2);
        client.release(&2);

        let escrow = client.query_escrow();
        assert!(escrow.estado == Estado::Completado);
    }
}
```

---

## Compilar y Ejecutar Tests

Una vez creado el proyecto, ejecuta:

```bash
cargo build
cargo test
```

---

## Flujo del Contrato

```
┌─────────────┐     ┌─────────────┐
│  initialize │────▶│   Activo    │
└─────────────┘     └──────────────┘
                          │
                          ▼
                   ┌──────────────┐
                   │   deposit    │────▶ Depositado
                   └──────────────┘
                          │
           ┌──────────────┼──────────────┐
           ▼            ▼            ▼
    ┌───────────┐ ┌────────┐  ┌─────────┐
    │ approve  │ │dispute │  │ cancel │
    └────┬────┘ └───┬────┘  └───┬────┘
         │          │            │
         ▼          ▼            ▼
   ┌──────────┐ ┌────────┐  ┌─────────┐
   │ release │ │dispute │  │canceled │
   └────┬────┘ └───┬────┘  └─────────┘
        │          │
        ▼          ▼
   ┌─────────────────┐
   │   Completado     │
   └─────────────────┘
```

---

## Ejercicios para Estudiantes

1. **Agregar función de emergencia**: Añade una función que permita al arbitro aprobar un hito si la empresa no responde en 30 días.

2. **Sistema de retención**: Modifica el contrato para retener el 10% del pago hasta que todos los hitos estén aprobados.

3. **Extensión de hitos**: Implementa la funcionalidad para agregar nuevos hitos después de que el contrato esté activo.

4. **Prueba de disputas**: Escribe un test que verifique que el freelancer puede abrir una disputa.

---

## Referencias

- [Stellar Soroban Documentation](https://developers.stellar.org/docs/smart-contracts/overview)
- [Soroban SDK](https://docs.rs/soroban-sdk/21.0.0/soroban_sdk/)