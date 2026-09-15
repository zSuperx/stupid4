#![allow(nonstandard_style)]
#![allow(clippy::upper_case_acronyms)]
#![allow(unused)]
// #![warn(unused_imports)]

pub(crate) mod common;
pub(crate) mod target;

pub use target::stir;

pub use target::x86::x86Function;
