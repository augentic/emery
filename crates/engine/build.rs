//! Rebuilds the engine when a document is added to or removed from `prose/`.
//!
//! `include_prose!` embeds every document under `prose/` by content, so an
//! edit rebuilds on its own; only a new or deleted file needs the tree watched.

fn main() {
    println!("cargo::rerun-if-changed=prose");
}
