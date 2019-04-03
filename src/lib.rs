#![feature(specialization)]
#![feature(nll)]
#![feature(const_fn)]
#![allow(dead_code)]

#[macro_use]
extern crate failure_derive;
#[macro_use]
extern crate lazy_static;

extern crate bigdecimal;
extern crate bytes;
extern crate chrono;
extern crate failure;
extern crate lifeguard;
extern crate num_bigint;
extern crate smallvec;

pub mod binary;
pub mod ion_system;
pub mod result;
pub mod symbol_table;
pub mod types;
