#![feature(iterator_step_by)]
#![feature(libc)]
#![feature(conservative_impl_trait)]

extern crate ocl;
extern crate hex_slice;
extern crate num_iter;
extern crate failure;
extern crate byteorder;
extern crate hex;
#[macro_use] extern crate serde_derive;
extern crate serde_json;
extern crate serde;
extern crate reqwest;
extern crate config;
#[macro_use] extern crate lazy_static;
#[macro_use] extern crate arrayref;

pub mod mine;
pub mod plot;
pub mod util;
