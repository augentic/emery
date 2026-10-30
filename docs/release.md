# Release process

This runbook describes the end-to-end release process for maintainers cutting, publishing, and patching an Emery release.

## Contents

- [Shipped artifacts](#shipped-artifacts)
- [Three version axes](#three-version-axes)
- [Release lines](#release-lines)
- [Three release shapes](#three-release-shapes)
- [Jobs that run](#jobs-that-run)
- [Publishing the wasm-pkg packages](#publishing-the-wasm-pkg-packages)
- [Adapter components](#adapter-components)
- [Installing a release](#installing-a-release)
- [Adding a new target triple](#adding-a-new-target-triple)
- [Troubleshooting](#troubleshooting)

---

## Shipped artifacts

An Emery release ships four artifacts:

- **Platform binaries:** Prebuilt archives produced by the `binaries` matrix job in `publish.yaml` and attached to the GitHub Release.
- **Workspace crates:** Rust library crates published to crates.io as `emery-<crate>` by the shared `crates.yaml` job (the root `emery` binary package and `emery-precompile` remain `publish = false`).
- **Engine guest (`emery:engine@<version>`):** The compiled WebAssembly (Wasm) engine component, published manually with `wkg publish`.
- **Adapter contract (`emery:adapter@<wit version>`):** The WebAssembly Interface Types (WIT) package, published manually with `wkg publish` whenever the `package` version in `wit/emery.wit` changes.

> **Note on crates.io publishing:** While `Cargo.toml` carries `[patch.crates-io]` git pins for unreleased `omnia` commits, Cargo rejects crates.io publishing because patches do not propagate to downstream consumers. Until the pinned `omnia` release is live on crates.io, the `crates` job will fail on dependency resolution while the binary and Wasm artifacts succeed.

---

## Three version axes

Three surfaces version independently—never force them to share a version number:

| Axis | Identity | Where it lives |
| ---- | -------- | -------------- |
| **Host** | `emery` binary / `emery:engine@<version>` | `[workspace.package].version` in `Cargo.toml`, the `v*` Git tag, `RELEASES.md` |
| **WIT contract** | `emery:adapter@<wit version>` | The `package` declaration in `wit/emery.wit` |
| **Adapter train** | `emery:<name>@<semver>` → `ghcr.io/augentic/emery/<name>:<version>` | The `augentic/emery-adapters` repository's shared `[workspace.package]` version |

- **Engine guest:** Imports `emery:adapter/source` and `emery:adapter/target` without defining a separate WIT package. Its published component (`emery:engine@<version>`) always matches the host binary version.
- **Host–adapter compatibility:** Declared via exact `namespace:name@version` references and each adapter's minimum `emery-version` field, rather than matching version numbers.
- **Cursor plugin (`plugins/emery/`):** Versions independently in `.cursor-plugin/marketplace.json` and `plugins/emery/.cursor-plugin/plugin.json`. Bump it only when files under `plugins/emery/` change.
- **Adapter resolution:** The `emery` binary embeds the engine guest and routes the `emery` namespace to `augentic.io`, reading cached `.wasm` components from `~/.emery/adapters` and fetching missing `emery:*` releases on demand ([Adapters](reference/adapters.md)). Adapters from other namespaces are installed manually with `wkg get <reference> -o ~/.emery/adapters/`.

---

## Release lines

Releases live on durable `release-X.Y.Z` branches driven by the shared `augentic/toolkit` workflows. `main` always carries the next unreleased version and an `Unreleased` heading in `RELEASES.md`.

1. **Cut:** Dispatch **Create Release** on `main`. This pushes `release-X.Y.Z` from the current tip and opens a pull request bumping `main` to the next unreleased version and resetting `RELEASES.md`. Merge that pull request, and edit release notes on the `release-X.Y.Z` branch.
2. **Stabilize:** On `release-X.Y.Z`, verify that `cargo build --locked` resolves cleanly (ensure any `[patch.crates-io]` git entries point to pushed commits, never local paths) and cherry-pick any fixes from `main`.
3. **Publish:** Dispatch **Publish Release** on `release-X.Y.Z`. This runs CI, builds the platform archives, dates `RELEASES.md`, pushes tag `vX.Y.Z`, creates the GitHub Release with the archives attached, and runs the crates.io publish job. Afterward, publish the `wasm-pkg` packages manually ([Publishing the wasm-pkg packages](#publishing-the-wasm-pkg-packages)).
4. **Patch:** For bug fixes or security patches on an existing `release-X.Y.Z` branch, land the fix on `main` first (when applicable), cherry-pick it to `release-X.Y.Z`, dispatch **Create Patch** on `release-X.Y.Z` (which bumps `X.Y.Z` to `X.Y.Z+1` and prepares `RELEASES.md`), and then dispatch **Publish Release** on `release-X.Y.Z`.

Pre-1.0 SemVer follows Omnia's convention: **minor versions may contain breaking changes**, while patch versions remain compatible within the release line.

---

## Three release shapes

Every release follows one of three ordering shapes so adapters are never published against an unreleased WIT contract or engine:

| Shape | Trigger | Order |
| ----- | ------- | ----- |
| **WIT-breaking** | `package emery:adapter@…` version changes | 1) Cut engine release branch and publish `emery:adapter` WIT<br>2) Publish engine release<br>3) Bump WIT and engine pins in `emery-adapters` and publish adapter train<br>4) Note `emery specify --fresh` in release notes if schema version `EMERY` changed |
| **Host-only** | CLI, lifecycle, or engine guest changes; WIT unchanged | Cut and publish engine release; adapters remain unchanged unless their minimum `emery-version` rises |
| **Adapter-only** | Adapter prompts, surveys, or target checks change; WIT unchanged | Cut and publish in `emery-adapters`; engine remains unchanged |

Include a compatibility line in each `RELEASES.md` entry showing the tested pair:

```text
engine 0.28.x  ↔  adapters 0.5.x  (WIT emery:adapter@0.1.0, emery-version ≥ 0.28.0)
```

---

## Jobs that run

The **Publish Release** workflow runs four jobs after the release-branch gate:

1. **`ci`:** Runs the shared `augentic/toolkit` CI checks on the release branch.
2. **`binaries`:** Builds and packages each target platform archive in `publish.yaml`, uploading `archive-<target>` workflow artifacts.
3. **`publish`:** Runs the shared `augentic/toolkit` publish workflow to date `RELEASES.md`, push tag `vX.Y.Z`, and create the GitHub Release with all `archive-*` artifacts attached.
4. **`crates`:** Runs `cargo publish --workspace --locked` across publishable `emery-*` crates using `CARGO_REGISTRY_TOKEN`.

### How `binaries` builds and verifies the embedded engine

Each matrix leg in `binaries` runs `cargo build --release --locked --target <triple> --bin emery` followed by `emery --help`:

- During a `--release` build, `build.rs` compiles the engine guest to `wasm32-wasip2` and then ahead-of-time (AOT) precompiles it for the target architecture using the workspace's `emery-precompile` binary with `omnia_compile::CompileOptions::default()`.
- At runtime, the host's Wasmtime settings (`MAX_FUEL`, `BRANCH_HINTING`, `MEMORY_RESERVATION`, `MEMORY_GUARD_SIZE`, `DEBUG_SYMBOLS`, `GENERATE_ADDRESS_MAP`) default to those same options.
- Running `emery --help` in CI loads the precompiled engine guest immediately, catching any Wasmtime version or compile-option mismatch before the archive can ship.

Supported native targets (built without `cross`):

- `x86_64-unknown-linux-gnu` on `ubuntu-latest`
- `x86_64-apple-darwin` on `macos-15-intel` (last hosted Intel macOS runner; scheduled for retirement in August 2027)
- `aarch64-apple-darwin` on `macos-latest`
- `x86_64-pc-windows-msvc` on `windows-latest` (temporarily excluded until upstream `omnia-wasi-model` restores Windows support)

Each leg outputs `emery-v${VERSION}-${TARGET}.tar.gz` (or `.zip` on Windows) and a `.sha256` checksum file, matching the `[package.metadata.binstall]` template in the root `Cargo.toml`.

---

## Publishing the wasm-pkg packages

Both `wasm-pkg` packages are published manually with `wkg publish`. Configure `~/.config/wasm-pkg/config.toml` to map the `emery` namespace to `augentic.io` with a GitHub token carrying `packages: write` (see [`wit/README.md`](../wit/README.md)). Registry versions are immutable—never overwrite an existing version.

- **Engine guest (`emery:engine@<version>`):** After tagging the release, build and publish the `wasm32-wasip2` engine component so other Omnia hosts can compose it at the exact binary version:

```bash
cargo build --lib -p emery --release --target wasm32-wasip2
wkg publish target/wasm32-wasip2/release/emery.wasm \
  --package "emery:engine@$(cargo pkgid -p emery | sed 's/.*#//')"
```

- **Adapter contract (`emery:adapter@<ver>`):** When `package emery:adapter@<ver>;` in `wit/emery.wit` moves, build and publish the WIT package before or alongside the engine release (see [`wit/README.md`](../wit/README.md)).

---

## Adapter components

First-party adapters live in [`augentic/emery-adapters`](https://github.com/augentic/emery-adapters) and are published from that repository's **Publish Release** workflow to `ghcr.io/augentic/emery/<name>:<version>` (which `augentic.io` resolves `emery:<name>@<version>` to).

Before cutting an adapter release in `augentic/emery-adapters`, verify that:

- Its vendored `wit/emery.wit` matches a published `emery:adapter` version.
- Its `[patch.crates-io]` git entries for `emery-sdk`, `emery-prose`, and `emery-lexicon` are pinned to a released `emery` Git tag (`tag = "vX.Y.Z"`), with no uncommented local `path` patches.
- The engine release it depends on is already published so `emery-version` checks succeed when users pull the new adapters.

---

## Installing a release

Users can install a published `emery` release through four channels:

- **Homebrew** (via [`augentic/homebrew-tap`](https://github.com/augentic/homebrew-tap)):

```bash
brew tap augentic/tap
brew install emery
```

- **`cargo binstall`** (downloads the prebuilt GitHub Release archive without compiling locally):

```bash
cargo binstall --git https://github.com/augentic/emery emery@<version>
```

- **GitHub Release archives:** Download the archive for your platform from the GitHub Releases page, verify its `.sha256` checksum, and place `emery` on your `PATH`.
- **Source build** (`build.rs` compiles and embeds the `wasm32-wasip2` engine guest automatically):

```bash
cargo install --git https://github.com/augentic/emery --locked
```

After publishing a new host release, update the `version` and `sha256` checksums in [`augentic/homebrew-tap`](https://github.com/augentic/homebrew-tap). Note that model-backed commands (`specify`, `amend`, `build`) also require `cursor-sdk-bridge` on `PATH` (from [cursor/sdk-bridge releases](https://github.com/cursor/sdk-bridge/releases)) and `CURSOR_API_KEY` (or a local `emery-dev` build).

---

## Adding a new target triple

1. Add a native `matrix.include` entry to the `binaries` job in `.github/workflows/publish.yaml` (`runs-on` must build the target natively without `cross`).
2. If the target requires system packages (such as `musl-tools` for `*-musl`), add a conditional `apt-get install` step gated on `matrix.target`.
3. Update `[package.metadata.binstall]` in `Cargo.toml` if the archive extension differs from `.tar.gz`.
4. Add the new target triple to the supported targets list on this page.

---

## Troubleshooting

- **Archive SHA-256 mismatch:** Never edit a `.sha256` file by hand. The checksum files are generated inside the `binaries` workflow job alongside each archive.
- **`wkg publish` rejects an existing version:** Registry versions are immutable. Bump the crate version (for `emery:engine`) or the `package` version in `wit/emery.wit` (for `emery:adapter`) and publish the new version.
- **Publish Release fails because the Git tag already exists:** Releases are immutable. Dispatch **Create Patch** on the `release-X.Y.Z` branch to bump the patch version, then run **Publish Release** again.
