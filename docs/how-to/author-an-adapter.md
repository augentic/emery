# Author a custom source or target adapter

This guide shows how to write, compile, and install a custom WebAssembly (Wasm) source adapter or target adapter in Rust using `emery-sdk`.

## Contents

- [Crate setup (`Cargo.toml`)](#crate-setup-cargotoml)
- [Author a source adapter (`SourceAdapter`)](#author-a-source-adapter-sourceadapter)
  - [1. Write the extraction prompt (`prose/extract.md`)](#1-write-the-extraction-prompt-proseextractmd)
  - [2. Implement `SourceAdapter` in `src/lib.rs`](#2-implement-sourceadapter-in-srclibrs)
- [Author a target adapter (`TargetAdapter`)](#author-a-target-adapter-targetadapter)
  - [1. Write `prose/build.md` and `prose/verify.md`](#1-write-prosebuildmd-and-proseverifymd)
  - [2. Implement `TargetAdapter` in `src/lib.rs`](#2-implement-targetadapter-in-srclibrs)
- [Test your adapter corpus and logic](#test-your-adapter-corpus-and-logic)
- [Build and install into `~/.emery/adapters`](#build-and-install-into-emeryadapters)
- [See also](#see-also)

---

## Crate setup (`Cargo.toml`)

Every Emery adapter is a Rust library crate compiled to `wasm32-wasip2` with `crate-type = ["cdylib", "rlib"]` (`cdylib` produces the `.wasm` component; `rlib` lets native Rust tests exercise your adapter directly on the host):

```toml
[package]
name = "acme-runbook"
version = "0.1.0"
edition = "2024"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
emery-sdk = { git = "https://github.com/augentic/emery.git" }
```

An adapter crate needs no `wit-bindgen` setup or `build.rs`—`emery-sdk` re-exports the contract types, prompt embedding macro (`prose!`), and guest export macros (`export_source!` and `export_target!`).

---

## Author a source adapter (`SourceAdapter`)

A source adapter inspects a workspace directory or inline string, divides it into one or more extraction **seams** (`Seam`), and calls `emery_sdk::extract` to run a gated model turn per seam.

### 1. Write the extraction prompt (`prose/extract.md`)

Create `prose/extract.md` in your crate root. This prompt instructs the model how to extract claims from your source format and must link `claims.md` (which the SDK serves automatically at runtime) and include a valid `## Worked example` JSON fence:

````markdown
# Runbook Extraction

Extract behavioural requirements and technology decisions from operational runbooks.
Follow the claim schema and technology rules in [claims.md](claims.md).

## Worked example

```json
{
  "claims": [
    {
      "kind": "requirement",
      "id": "alerts.escalate-unacknowledged",
      "path": "runbooks/oncall.md#L10-L18",
      "statement": "When an incident remains unacknowledged for 15 minutes, the service escalates the alert to the secondary rotation."
    }
  ]
}
```
````

### 2. Implement `SourceAdapter` in `src/lib.rs`

Implement `emery_sdk::SourceAdapter` on `pub struct Adapter;`, embed your prompt corpus with `emery_sdk::prose!`, and export the component under `#[cfg(target_arch = "wasm32")]`:

```rust
use emery_sdk::{
    Context, Doc, Error, Evidence, Metadata, Model, Seam, SourceAdapter, SourceContent,
    SourceInput, SourceKind, bad_request,
};

pub struct Adapter;

impl SourceAdapter for Adapter {
    fn metadata() -> Metadata {
        // Sets default authority rank (Intent = 1, Documentation = 2, Behaviour = 3)
        Metadata {
            kind: SourceKind::Documentation,
        }
    }

    async fn extract<P: Model>(ctx: &Context<'_, P>) -> Result<Evidence, Error> {
        let seams = survey(ctx.input)?;
        emery_sdk::extract(ctx, PROSE, &seams).await
    }
}

pub static PROSE: &[Doc] = emery_sdk::prose!["../prose/extract.md"];

fn survey(input: &SourceInput) -> Result<Vec<Seam>, Error> {
    if let SourceContent::Value(value) = &input.content
        && value.trim().is_empty()
    {
        return Err(bad_request!("the inline runbook description is empty"));
    }
    Ok(vec![Seam::whole()])
}

#[cfg(target_arch = "wasm32")]
emery_sdk::export_source!(Adapter);
```

See [`examples/source/lib.rs`](../../examples/source/lib.rs) in the `emery` repository for a complete runnable example.

---

## Author a target adapter (`TargetAdapter`)

A target adapter handles three tasks for `emery build`:

- **`Task::Init(InitContext)`:** Deterministically scaffolds missing project layout files with `create_new` (no model turn) and returns `Outcome::Initialised`.
- **`Task::Build(BuildContext)`:** Optionally scaffolds slice files (`ctx.with_scaffold(..)`), runs one gated turn via `emery_sdk::target::build` under `prose/build.md`, and returns `Outcome::Built(Report)`.
- **`Task::Verify(VerifyContext)`:** Runs one gated turn via `emery_sdk::target::verify` under `prose/verify.md` to run workspace-wide checks and repairs, returning `Outcome::Verified(Verdict)`.

### 1. Write `prose/build.md` and `prose/verify.md`

Every target adapter embeds `prose/build.md` (with a `## Worked example` JSON fence parsing as `Report`) and `prose/verify.md` (with a `## Worked example` JSON fence parsing as `Verdict`). See [`examples/target/prose/`](../../examples/target/prose/build.md) for minimal templates.

### 2. Implement `TargetAdapter` in `src/lib.rs`

```rust
use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Write as _};
use std::path::Path;

use emery_sdk::target::{
    Grant, InitContext, MergeRule, Metadata, Outcome, TargetAdapter, Task,
};
use emery_sdk::{Doc, Error, Model, server_error};

// Path-glob merge rules applied when slice commits merge into .emery/vcs/integration
const RULES: &[MergeRule] = &[MergeRule::union("src/index.ts")];

// Optional worker sandbox grants (network hosts, read-only ~/ roots, env vars)
const GRANTS: &[Grant] = &[];

pub struct Adapter;

impl TargetAdapter for Adapter {
    fn metadata() -> Metadata {
        Metadata {
            rules: RULES,
            grants: GRANTS,
        }
    }

    async fn run<P: Model>(task: Task<'_, P>) -> Result<Outcome, Error> {
        match task {
            Task::Init(ctx) => init(&ctx).map(|()| Outcome::Initialised),
            Task::Build(ctx) => emery_sdk::target::build(&ctx, PROSE, |_| Ok(Vec::new()))
                .await
                .map(Outcome::Built),
            Task::Verify(ctx) => emery_sdk::target::verify(&ctx, PROSE, |_| Ok(Vec::new()))
                .await
                .map(Outcome::Verified),
        }
    }
}

pub static PROSE: &[Doc] = emery_sdk::prose!["../prose/build.md", "../prose/verify.md"];

fn init(ctx: &InitContext<'_>) -> Result<(), Error> {
    let root = Path::new(ctx.workspace);
    let readme = root.join("README.md");
    if let Some(parent) = readme.parent() {
        fs::create_dir_all(parent).map_err(|e| server_error!("creating dir: {e}"))?;
    }
    match OpenOptions::new().write(true).create_new(true).open(&readme) {
        Ok(mut f) => f
            .write_all(b"# Generated Project\n")
            .map_err(|e| server_error!("writing README.md: {e}"))?,
        Err(e) if e.kind() == ErrorKind::AlreadyExists => {}
        Err(e) => return Err(server_error!("opening README.md: {e}")),
    }
    Ok(())
}

#[cfg(target_arch = "wasm32")]
emery_sdk::export_target!(Adapter);
```

See [`examples/target/lib.rs`](../../examples/target/lib.rs) for the complete mock target adapter.

---

## Test your adapter corpus and logic

Use `emery_sdk::check` in a native test to verify that every file in `prose/` is listed in `PROSE`, every relative link resolves, and every document is reachable from your root prompts:

```rust
#[test]
fn prose_corpus_is_sound() {
    let tree = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("prose");
    emery_sdk::check(
        acme_runbook::PROSE,
        &tree,
        &["extract.md"],
        emery_sdk::RUNTIME,
    );
}
```

---

## Build and install into `~/.emery/adapters`

Compile the component for `wasm32-wasip2` and copy the resulting `.wasm` file into `~/.emery/adapters/` using the `<namespace>_<name>@<version>.wasm` filename convention:

```bash
cargo build --target wasm32-wasip2 --release

mkdir -p ~/.emery/adapters
cp target/wasm32-wasip2/release/acme_runbook.wasm \
  ~/.emery/adapters/acme_runbook@0.1.0.wasm
```

You can now reference `adapter = "acme:runbook@0.1.0"` in `emery.toml` or on the command line.

---

## See also

- [`crates/sdk/README.md`](../../crates/sdk/README.md) — `emery-sdk` API overview.
- [`wit/README.md`](../../wit/README.md) — the `emery:adapter` WIT contract.
- [Install, pin, and vendor Wasm adapters](manage-adapters.md) — managing `~/.emery/adapters` and `digest` pins.
