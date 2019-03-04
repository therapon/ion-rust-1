#![feature(specialization)]
#![feature(nll)]
#![allow(dead_code)]

#[macro_use]
extern crate failure_derive;
#[macro_use]
extern crate lazy_static;

extern crate bigdecimal;
extern crate bytes;
extern crate chrono;
extern crate failure;
extern crate num_bigint;
extern crate smallvec;
extern crate lifeguard;

pub mod binary;
pub mod result;
pub mod types;
pub mod ion_system;
pub mod symbol_table;