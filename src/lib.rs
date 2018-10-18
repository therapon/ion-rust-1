#![feature(specialization)]
#![feature(bufreader_seek_relative)]
#![allow(dead_code)]

#[macro_use] extern crate failure_derive;
//#[macro_use] extern crate log;

extern crate bigdecimal;
extern crate bytes;
extern crate chrono;
//extern crate env_logger;
extern crate failure;
extern crate num_bigint;
extern crate smallvec;

pub mod binary;
pub mod result;
pub mod types;