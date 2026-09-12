//! The `emery:adapter` contract
//!
//! The agreement between the Emery engine and every adapter: the Rust side of
//! the `emery:adapter` WIT package, one module per axis. Each axis module
//! carries its WIT world's bindings, the Rust types that mirror its records,
//! the rules those records must satisfy, and the capability the engine calls
//! adapters of that axis through. Today there is one axis, [`source`]; the
//! grammar every name in the contract follows is shared at the root.
//!
//! Both sides depend on this one crate so they cannot drift apart. The engine
//! consumes it directly; adapters receive it re-exported through the
//! `emery-sdk` SDK.

mod grammar;
pub mod source;

pub use grammar::{CLAIM_ID_REGEX, is_kebab};
