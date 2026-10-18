//! Minimal example target adapter.
//!
//! Initialises the workspace with `build/README.md`, builds slices and
//! verifies waves through [`emery_sdk::target`], and union-merges
//! `build/*/index.md`.

use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Write as _};
use std::path::Path;

use emery_sdk::target::{InitContext, MergeRule, Metadata, Outcome, TargetAdapter, Task};
use emery_sdk::{Doc, Error, Model, server_error};

// two slices that list requirements in one index keep both lists
const RULES: &[MergeRule] = &[MergeRule::union("build/*/index.md")];

/// The adapter implementation.
pub struct Adapter;

impl TargetAdapter for Adapter {
    fn metadata() -> Metadata {
        Metadata {
            rules: RULES,
            ..Metadata::default()
        }
    }

    async fn run<P: Model>(task: Task<'_, P>) -> Result<Outcome, Error> {
        match task {
            Task::Init(ctx) => init(&ctx).map(|()| Outcome::Initialised),
            Task::Build(ctx) => emery_sdk::target::build(&ctx, PROSE, None, |_| Ok(Vec::new()))
                .await
                .map(Outcome::Built),
            Task::Verify(ctx) => emery_sdk::target::verify(&ctx, PROSE, |_| Ok(Vec::new()))
                .await
                .map(Outcome::Verified),
        }
    }
}

/// The build prompt and the verify prompt embedded in the adapter.
pub static PROSE: &[Doc] = emery_sdk::prose!["prose/build.md", "prose/verify.md"];

// The layout's one root file, laid when the repository is initialised and
// found by every slice.
const FILES: &[(&str, &str)] = &[(
    "build/README.md",
    "# Build\n\nOne directory per slice, each with an `index.md` naming the requirements it \
     implements and one file per requirement.\n",
)];

// Lays each file of the layout the tree lacks; one the tree holds stands.
fn init(ctx: &InitContext<'_>) -> Result<(), Error> {
    let root = Path::new(ctx.workspace);
    for (path, content) in FILES {
        let file = root.join(path);
        if let Some(dir) = file.parent() {
            fs::create_dir_all(dir)
                .map_err(|err| server_error!("laying `{path}`: creating its directory: {err}"))?;
        }
        match OpenOptions::new().write(true).create_new(true).open(&file) {
            Ok(mut file) => file
                .write_all(content.as_bytes())
                .map_err(|err| server_error!("laying `{path}`: {err}"))?,
            Err(err) if err.kind() == ErrorKind::AlreadyExists => {}
            Err(err) => return Err(server_error!("laying `{path}`: {err}")),
        }
    }
    Ok(())
}

#[cfg(target_arch = "wasm32")]
emery_sdk::export_target!(Adapter);
