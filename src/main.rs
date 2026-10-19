//! Defines the shipped `emery` runtime binary.
//!
//! The shared deployment of `deployment.rs` with Cursor answering the model:
//! every worker it runs for a lent tree is confined to the tree and the
//! shell roots, and reaches beyond them what the target adapter's grants
//! name.

cfg_select! {
    not(target_arch = "wasm32") => {
        #[path = "deployment.rs"]
        mod deployment;

        use omnia_cursor::{Client as Cursor, ConnectOptions as CursorOptions};

        deployment::runtime!(Cursor, CursorOptions);
    }
    _ => {
        fn main() {}
    }
}
