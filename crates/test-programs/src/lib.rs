//! Guest drivers, scripted adapter components, and shared test fixtures.
//!
//! Exports scenario and script protocols alongside stub survey fixtures on all
//! targets, guest I/O helpers on `wasm32`, and compiled artifact paths natively.

pub mod protocol;
pub mod script;
pub mod stub;

#[cfg(target_arch = "wasm32")]
pub mod helpers;
#[cfg(target_arch = "wasm32")]
pub mod script_io;
#[cfg(target_arch = "wasm32")]
pub mod seams;

#[cfg(not(target_arch = "wasm32"))]
include!(concat!(env!("OUT_DIR"), "/gen.rs"));
