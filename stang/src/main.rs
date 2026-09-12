#![allow(static_mut_refs)]
#![allow(nonstandard_style)]
#![allow(unused)]
// #![warn(unused_imports)]
#![warn(unused_qualifications)]
#![warn(unused_allocation)]

mod IRs;
mod ast;
mod codegen;
mod common;
mod driver;
mod parser;
mod sema;
mod translation_unit;

fn main() {
    driver::run();
}
