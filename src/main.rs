//! Runs the shipped `emery` runtime.
//!
//! The runtime is [`emery::runtime`]. The binary is host-only; on `wasm32`
//! it compiles to an empty `main` so the workspace-wide wasm32 clippy pass
//! can include it.

cfg_select! {
    not(target_arch = "wasm32") => {
        fn main() -> std::process::ExitCode {
            emery::runtime::main()
        }
    }
    _ => {
        fn main() {}
    }
}
