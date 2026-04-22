#![allow(non_camel_case_types)]
#![allow(dead_code)]

use soroban_sdk::contracterror;

#[contracterror]
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(u32)]
pub enum Error {
    YaInicializado = 1,
    NoInicializado = 2,
    HitosVacios = 3,
    EstadoInvalido = 10,
    NoDepositado = 11,
    HitoNoEncontrado = 20,
    HitoYaAprobado = 21,
    HitoYaPagado = 22,
    HitoNoAprobado = 23,
    MontoIncorrecto = 30,
    FondosInsuficientes = 31,
    NoAutorizado = 40,
    NoPuedeCancelar = 50,
}