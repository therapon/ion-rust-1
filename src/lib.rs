#![feature(specialization)]
//#![feature(bufreader_seek_relative)]
#![feature(nll)]
#![feature(int_to_from_bytes)]
#![allow(dead_code)]
//#![feature(bufreader_buffer)]
//#![feature(const_let)]

#[macro_use]
extern crate failure_derive;
#[macro_use]
extern crate lazy_static;

extern crate bigdecimal;
extern crate bytes;
extern crate chrono;
//extern crate env_logger;
extern crate failure;
extern crate num_bigint;
extern crate smallvec;
extern crate wasm_bindgen;
extern crate lifeguard;
extern crate cfg_if;
extern crate memmap;

pub mod binary;
pub mod result;
pub mod types;
pub mod ion_system;
pub mod symbol_table;