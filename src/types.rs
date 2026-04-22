use soroban_sdk::{contracttype, Address, Symbol, Vec};

#[contracttype]
#[derive(Clone, PartialEq, Debug)]
pub enum Estado {
    Activo,
    Depositado,
    Completado,
    Disputado,
    Cancelado,
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