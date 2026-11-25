//! Builds SDK driver `wasm32-wasip3` components and generates `gen.rs`.

fn main() {
    omnia_test::build::Components::in_workspace("../..")
        .package("test-programs")
        .scan("crates/test-programs/programs")
        .sync_examples("crates/test-programs/Cargo.toml")
        .track(["Cargo.lock", "crates/test-programs/src", "crates/test-programs/Cargo.toml"])
        .build()
        .write_gen("gen.rs");
}
