//! Core CRuby bindings module.

#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(clippy::upper_case_acronyms)]

pub type size_t = u64;
pub type RedefinitionFlag = u32;

/// An object handle similar to VALUE in the C code.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
#[repr(transparent)]
pub struct VALUE(pub usize);

#[allow(unsafe_op_in_unsafe_fn)]
#[allow(dead_code)]
#[allow(non_snake_case)]
#[allow(clippy::all)]
mod autogened {
    use super::*;
    include!("cruby_bindings.inc.rs");
}
pub use autogened::*;
