//! The `emery:adapter` contract
//!
//! The agreement between the Emery engine and every adapter: the Rust side of
//! the `emery:adapter` WIT package, one module per axis. Each axis module
//! carries its WIT world's bindings, the Rust types that mirror its records,
//! the rules those records must satisfy, and the capability the engine calls
//! adapters of that axis through. Today there is one axis, [`source`]; the
//! kebab grammar every name in the contract follows is shared at the root.
//!
//! Both sides depend on this one crate so they cannot drift apart. The engine
//! consumes it directly; adapters receive it re-exported through the
//! `emery-sdk` SDK.

pub mod source;

/// Tells whether `value` follows the kebab grammar shared by claim-id
/// segments, source keys, and adapter names: `[a-z0-9]+(-[a-z0-9]+)*`.
#[must_use]
pub fn is_kebab(value: &str) -> bool {
    value.split('-').all(|segment| {
        !segment.is_empty() && segment.bytes().all(|byte| matches!(byte, b'a'..=b'z' | b'0'..=b'9'))
    })
}