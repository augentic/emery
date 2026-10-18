//! Precompiles a `wasm32-wasip2` component for the release `emery` binary.
//!
//! Invoked by the root build script as
//! `emery-precompile <wasm> <output> --target <triple>` using default runtime
//! compile options.

cfg_select! {
    not(target_arch = "wasm32") => {
        use std::ffi::OsString;
        use std::path::PathBuf;

        use anyhow::{Context as _, Result, bail};
        use omnia_compile::CompileOptions;

        const USAGE: &str = "usage: emery-precompile <wasm> <output> --target <triple>";

        fn main() -> Result<()> {
            #[expect(
                clippy::disallowed_methods,
                reason = "a host tool, not a guest: argv is the build script's call"
            )]
            let args: Vec<OsString> = std::env::args_os().skip(1).collect();
            let [wasm, output, flag, triple] = args.as_slice() else {
                bail!("{USAGE}");
            };
            if flag != "--target" {
                bail!("{USAGE}");
            }
            let triple =
                triple.to_str().with_context(|| format!("{USAGE}: the triple is not UTF-8"))?;

            let wasm = PathBuf::from(wasm);
            let output = PathBuf::from(output);
            omnia_compile::compile(&wasm, Some(output), Some(triple), &CompileOptions::default())
                .with_context(|| format!("compiling {} for `{triple}`", wasm.display()))
        }
    }
    _ => {
        fn main() {}
    }
}
