# Coding standards

The external baseline is the [Pragmatic Rust Guidelines](https://microsoft.github.io/rust-guidelines/guidelines/index.html) (and the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/) they build on): follow it for anything this document and [style.md](./style.md) do not address. Every section below is a house delta — a project contract, a sharper rule, or an explicit override — and where a section disagrees with the baseline, this document wins. Enforced by clippy (`make lint`) and review. When a rule fights you, add the case to the rule with a before/after — don't carve out a local exception.

## Lints

Workspace lints live in `Cargo.toml`. Defaults are aggressive — clippy `all`/`cargo`/`nursery`/`pedantic` are all `warn`, plus a curated set of `restriction` lints and a tightened rust lint set (`missing_docs`, `unsafe_code`, and the rest of the `[workspace.lints.rust]` table). Compile under `RUSTFLAGS=-Dwarnings` (`make test` does this), so any new warning fails CI.

Visibility on internal items follows clippy's `redundant_pub_crate` (nursery) rather than rustc's `unreachable_pub`: prefer bare `pub` and let the parent module's privacy do the constraining. The two lints are mutually exclusive — enabling both would loop. `unreachable_pub` stays at its allow-by-default, and any `#[expect(unreachable_pub, …)]` carve-out is a rot signal, not a tool you reach for.

Doc idents such as `GitHub`, `MiB`, `OAuth`, `OpenTelemetry`, `SemVer`, `WebAssembly`, and `YAML` live in `clippy.toml` `doc-valid-idents`. Suppression rules are in [Lint suppression posture](#lint-suppression-posture) below.

`taplo.toml` formats `Cargo.toml` files. Dependency arrays under `*-dependencies` and `dependencies` reorder alphabetically; preserve that on edit.

## Lint suppression posture

Site-local suppressions are `#[expect(<lint>, reason = "…")]` at the **smallest possible scope**, not `#[allow]` — a dead `#[expect]` is a build failure, so the suppression cannot rot (the baseline's M-LINT-OVERRIDE-EXPECT). The house additions: module-level suppressions stay `#![allow(<lint>, reason = "…")]` because lint-rot detection at the module root is not useful (the suppression typically covers many sites), and identical `reason = "…"` strings across three or more files mean you should promote a single `#![allow]` to the parent module — the file-level repetition is noise, not signal.

```rust
// BAD — site-local #[allow]
#[allow(clippy::cognitive_complexity, reason = "linear state machine")]
fn step(...) { ... }

// GOOD — same scope, #[expect]
#[expect(clippy::cognitive_complexity, reason = "linear state machine")]
fn step(...) { ... }
```

## Comments

Comments answer "why does this look like this *today?*" — non-obvious intent, trade-offs, or constraints the code itself can't convey. Migration trails, old labels, and "this used to be X" rationale belong in commit messages — not in code or doc comments. Doc comments on items that surface in `--help` (clap `#[derive]` fields) must be operator-facing one-liners; rationale moves below the derive block where it doesn't leak into help output.

What each kind of comment is for, in Rust sources and WIT contracts (`wit/`, `crates/*/wit/`) alike. There are no length caps: a comment is as long as its why takes, and no longer.

- **Module `//!` docs** answer "what is this module, and why does it exist?" for a reader who has not opened the file: a short title line, then plain-language paragraphs. Say what the module is for and what it guarantees; never how it works — that is the code's job, and prose about mechanics goes stale first. No deployment tours, no AGENTS.md restatements, no RFC archaeology, and no house shorthand (`fail-closed`, `typed`, kernel names) the reader would have to look up.
- **Item `///` docs** follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/documentation.html): a one-line summary, then whatever the reader needs, with `# Errors` / `# Panics` sections where they apply.
- **`//` comments** sit beside the surprising branch they explain, never in a preamble essay.
- **Historical phrases** are banned in comments and docs: `Phase `, `formerly`, `previously lived`, `old contract`, `former tests`, `to avoid the`. Git history is the record.

```rust
// BAD
//! Per the workspace split 2.9 ("Specify wires components, not adapters"),
//! `specify` commits only the revision documents — `spec.md` plus
//! `design.md`. The pre-Phase-3.7 filename was `charter.md`;
//! Historical rename detail belongs in git history, not module docs.

// GOOD
//! The `specify` operation
//!
//! Emery's central operation: given a list of sources, extract each
//! source's claims, derive the requirements under authority precedence,
//! synthesise `spec.md` and `design.md`, and commit the pair as one new
//! revision.
//!
//! The result reports what was committed — the revision id and the
//! diff against the outgoing revision — so a caller can see what
//! changed without reading the documents.
```

The composition-root failure mode is the essay that restates architecture and hides the tip. The module doc says what the deployment is and why it is fixed; the operational tip stays at the site that needs it:

```rust
// BAD — 22-line //! deployment tour restating AGENTS.md, with the one
// operational fact (the read-only project mount) buried in the middle.

// GOOD
//! The `emery` executable
//!
//! The shipped runtime: one omnia deployment that embeds the engine guest
//! and declares everything it is allowed to touch.
//!
//! The deployment is fixed at compile time so a given `emery` binary always
//! runs with the same policy; there is no runtime configuration to audit.

// …inside the macro body:
// The invocation directory mounts read-only — nothing writes the tree.
mounts: [{ name: ".", path: "." }],
```

Doc comments describe what this is today. Version-history tables, dated bumps, commit hashes, and migration notes belong in git log — not in `///` blocks. Longer prose belongs in the standards docs.

`cargo doc` is part of `make ci`, so doc comments must compile. Reference paths inside backticks (`` `Self::config_path` ``) are fine; bare links (`[Foo]`) need a corresponding intra-doc target or rustdoc fails the build.

## Naming

Prefer short, idiomatic Rust names. Don't restate context the surrounding module, type, or function already supplies. Avoid `_local` / `_value` / `_helper` suffixes. Predicates start with `is_` / `has_`. A handler's DTOs are `<Verb>Input` and `<Verb>Output` (`SpecifyInput` → `SpecifyOutput`, `ShowInput` → `ShowOutput`): omnia's own names for the two positions, the `input: I` the fn takes and its `Handler::Output`. Never `<Verb>Body` — in omnia's vocabulary a body is the *encoded* wire form (`Encoded`, `ErrorBody`) the projector produces from the output. Never `<Verb>Response` — `omnia_guest::api::command::Response` is the buffered envelope the façade owns. Never `<Verb>Json` — the format dispatch lives in the command projector (see [handler-shape.md](./handler-shape.md)). The prefix repeats the module (`specify::SpecifyInput`) on purpose: the types are consumed cross-crate, where `emery_engine::specify::SpecifyInput` is what the reader sees.

**Tests.** A `#[test]` `fn` names the *scenario* (`gen_spec`, `shared_roots`), never the outcome or the assertion (`rendered_documents_read_back`, `clean_evidence_passes`). The `//` requirement comment above the test carries the why; the identifier does not.

A function defined in `mod <name>` (or `commands/<name>.rs`) MUST NOT carry `<name>` as a suffix or prefix on its own name — the module path already supplies that context. Review only: clippy's `module_name_repetitions` sits in the `restriction` group and stays off, because it would flag the `<Verb>Input` / `<Verb>Output` DTOs, which repeat their module deliberately (see above).

```rust
// BAD — file is commands/registry.rs / mod registry
fn show_registry(ctx: &Ctx) -> ... { ... }
fn validate_registry(ctx: &Ctx) -> ... { ... }
fn add_to_registry(ctx: &Ctx) -> ... { ... }

// GOOD — caller writes registry::show, registry::validate, registry::add
fn show(ctx: &Ctx) -> ... { ... }
fn validate(ctx: &Ctx) -> ... { ... }
fn add(ctx: &Ctx) -> ... { ... }

// BAD — the identifier narrates the assertion
fn rendered_documents_read_back() { ... }

// GOOD — the scenario; the comment carries the why
// A re-run over changed evidence reports the heading-level remine.
fn remine_supersedes() { ... }
```

## Brevity

The codebase optimises for short reading over short writing. Concretely:

- **Names**: predicates start with `is_` / `has_`. Avoid `_local` / `_value` / `_helper` / `_path` / `_dir` suffixes when the parameter type or surrounding context already says so (`is_slot(p: &Path)`, not `is_slot_path`).
- **Cross-module redundancy**: `WorkspaceBranchPreparationFailed` inside `Error` reads as `Error::WorkspaceBranchPreparationFailed` — drop the `Workspace` prefix when every variant in the cluster already operates on a workspace. In-module and cross-module redundancy are both on you and reviewers (`module_name_repetitions` is off — see [Naming](#naming)).
- **One-variant enums** are dead overhead. Drop the variant or the enum. If the type's name already discriminates, the enum adds nothing.
- **Field prefixes**: a struct named `RegistryAmendmentArgs` does not carry `proposed_` on every field — the struct name already says "proposal".
- **Comment redundancy**: don't paraphrase a `match` arm's variant in a `// …` comment when the variant's doc-comment already explains it.

Reviewers catch comment redundancy (see [Comments](#comments)) and module-name restatement (see [Naming](#naming)).

## Module shape

A module reads top-down: what it does, what it yields, how. **Review only.**

- **Order**: module doc, `use`, constants, the public entry function(s), the public types those entries take or return (each `struct`/`enum` immediately followed by its `impl` blocks), then private helpers in call order, then `#[cfg(test)]`. A private state machine or DTO the entry uses goes *below* the entry, not above it.
- **Phases, not statements**: one blank line separates the phases of a function body (acquire → transform → validate → return) and precedes a trailing `Ok(...)` when the body has more than a few statements. Do not blank-line every statement.
- **Comment by visibility**: exported items carry `///`. Private and `pub(crate)` items carry a `//` line only when it answers "why" — a comment that restates the name is deleted. Clippy's `missing_errors_doc` / `missing_panics_doc` only check exported items, so a `# Errors` section on a non-exported fn is noise, not a requirement; reducing visibility is the lever that lets you drop it.
- **Inline single-use wrappers**: a private fn with one caller whose body is one expression, and whose name adds nothing the expression does not say, is inlined at the call site. Keep the fn when it has two or more callers, names a concept the call site should not spell out (`store::failed`), or is a multi-step body.
- **Name the capability at the dispatch site**: when the receiver is a generic bounded by more than one capability trait (`P: Source + Plugins`, `S: StateStore + BlobStore`), call `Source::extract(provider, …)` / `BlobStore::put(store, …)` rather than `provider.extract(…)`, so the seam being crossed is visible without resolving the bound.
- **Keep an `impl` with its type**: no `impl ForeignType` in a consumer module. A consumer that needs behaviour over a type it does not own writes a free fn taking `&Type`.

```rust
// BAD — entry buried under a private helper, wrapper with one caller
async fn dispatch<P: Source>(provider: &P, id: &str, input: &SourceInput) -> Result<Evidence, Error> {
    provider.extract(id, input).await
}

/// Loads, extracts, and validates every source.
pub async fn evidence<P: Source + Plugins>(...) -> Result<Vec<Extract>, Error> {
    for source in sources {
        let adapter = /* … */;
        let evidence = dispatch(provider, &adapter.id, &source.input()?).await?;
        gate(&evidence)?;
        extracted.push(Extract { /* … */ });
    }
    Ok(extracted)
}

// GOOD — entry first, capability named, phases separated, wrapper inlined
/// Loads, extracts, and validates every source.
pub async fn evidence<P: Source + Plugins>(...) -> Result<Vec<Extract>, Error> {
    for source in sources {
        let input = source.input()?;
        let adapter = /* … */;

        let evidence = Source::extract(provider, &adapter.id, &input).await?;

        let findings = evidence.findings();
        if !findings.is_empty() {
            return Err(bad_request!(/* … */));
        }
        extracted.push(Extract { /* … */ });
    }

    Ok(extracted)
}

/// One source's evidence, under the key the documents cite it by.
pub struct Extract { /* … */ }
```

## Format dispatch

Operations do **not** open-code `match format { Json, Text }`. They return typed outputs; omnia's command projector (`omnia_guest::api::command::Command::call`, driven from `crates/cli/src/lib.rs`) owns format dispatch through `omnia_guest::api::Format::encode`. Operations never pick a sink directly. See [handler-shape.md](./handler-shape.md) for the operation and projector contract.

```rust
// BAD
match format {
    Format::Json => serde_json::to_writer(stdout(), &SomeOutput::from(&r))?,
    Format::Text => println!("..."),
}

// GOOD — the operation returns the typed output; the projector encodes it
Ok(SomeOutput::from(&result))
```

Text mode renders through the output's render fn in `crates/cli/src/text.rs` (`fn(&Output, &mut dyn fmt::Write) -> fmt::Result`, passed to `Command::call` as the verb's text form); the JSON path goes through `serde::Serialize` automatically. Engine outputs carry no `Display` — their terminal shape is a CLI concern, and an engine `Display` would quietly become part of every other transport's contract. New code must not introduce `match … format`.

## One emit path

Outputs and failures leave operations as typed values. Omnia's command projector encodes those values at the command boundary — the output as the success body in the selected format on stdout, the `Failure` envelope on stderr; no handler writes stdout or stderr. If you need a bespoke failure shape, construct an Omnia `Error` (macros for defaults; explicit variants only for the four recovery codes); do not hand-roll a `*ErrBody` DTO or a second envelope. `emery_cli` contributes only the render fns and the hint table; it never encodes.

## DTOs

Output DTOs (`*Output`, and any row type they carry) are **top-level** structs under `mod`. Declaring a DTO inside a function body, match arm, or closure forces a per-file `#![allow(items_after_statements, …)]` suppression and is the signal that a handler hasn't been migrated yet.

**Construct DTOs through `From` impls, not named builders.** Use `impl From<&Domain> for FrobOutput` so the conversion is discoverable at the trait surface and call sites read `FrobOutput::from(&domain)`. Named constructors are reserved for multi-arg or fallible builders (e.g. `RegistryProposalRow::from_kind` returns `Option<Self>`); each survivor carries a one-line doc justification.

**Typed fields, not stringly-typed ones.** `pub status` / `pub kind` (and any other field whose domain has a finite enum) carry the underlying domain enum with `#[derive(Serialize)]` + `#[serde(rename_all = "kebab-case")]`. Drop `.to_string()` at construction sites; the wire shape is unchanged.

**`PathBuf` for path fields.** `*Output` fields that hold a filesystem path are `path: PathBuf`. Do not store `String` paths in DTOs; serde's default `PathBuf` serialization carries the bytes losslessly.

**Field-type allowlist.** DTO fields use the strictest type the wire shape supports:

| Domain                                   | Type                                                        | Notes                                                       |
| ---------------------------------------- | ----------------------------------------------------------- | ----------------------------------------------------------- |
| Filesystem path                          | `PathBuf`                                                   | never `String`; serde's default carries the path losslessly |
| Status / kind / phase with finite domain | the underlying enum + `#[serde(rename_all = "kebab-case")]` | drop `.to_string()` at construction                         |
| Stable kebab discriminant                | `&'static str`                                              | lives in the binary                                         |
| Count                                    | `usize`                                                     | JSON has neither `u32` nor `u64`                            |

**Single-variant enums are dead overhead.** Drop either the variant or the enum; the type's name already says "this DTO represents kind X". The `BriefAction::Init` pattern is the canonical example of what not to add.

```rust
// BAD — DTO inside fn body
fn handle(...) {
    #[derive(Serialize)]
    struct HandleOutput { name: String }
    output::write(format, &HandleOutput { name }, write_text)?;
}

// BAD — named builder, stringly-typed status, String path
impl HandleOutput {
    pub(crate) fn from_outcome(outcome: &Outcome, path: PathBuf) -> Self {
        Self {
            status: outcome.status.to_string(),
            path: path.display().to_string(),
        }
    }
}

// GOOD — the engine output is a Serialize-only DTO …
#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct HandleOutput {
    pub name: String,
    pub status: OutcomeStatus,
    pub path: PathBuf,
}

impl From<&Outcome> for HandleOutput {
    fn from(outcome: &Outcome) -> Self { /* ... */ }
}

// … and its text mode is a render fn in the CLI (crates/cli/src/text.rs)
pub fn handle(output: &HandleOutput, w: &mut dyn fmt::Write) -> fmt::Result {
    writeln!(w, "{}", output.name)
}
```

## Errors

Engine operations, the adapter SDK, and adapters return `omnia_guest::Error` (`BadRequest`, `NotFound`, `ServerError`, `BadGateway`). Construct Omnia defaults with the crate-root macros (`bad_request!`, `not_found!`, `server_error!`, `bad_gateway!`); those emit snake_case codes (`bad_request`, …). Keep explicit variant construction only for the four recovery discriminants (`specify-source-required`, `unsupported-version`, `spec-not-generated`, `spec-outdated`). Do not introduce a house error type or constructor wrappers; the adapter WIT `error` variant is lowered and lifted inside the contract crate's `source::bindings` (`emery-adapter`) alone (see [style.md](./style.md#failures-are-omnia-errors)).

**Class on a direct match.** Pick the Omnia variant that matches the failure: operator or input refusals are `BadRequest` (exit 1), missing resources are `NotFound` (exit 2), upstream or model failures are `BadGateway` (exit 4). Anything else — I/O, storage, leftover conversions — is `ServerError` (exit 3). An adapter's `BadRequest` keeps its class through the `Source` capability, so an adapter refusing its input exits 1 like any other input refusal; every other adapter failure reaches the engine as `BadGateway`. Do not invent new codes or new exit slots. See [handler-shape.md §"Exit codes"](./handler-shape.md#exit-codes).

**Hint lookup.** Long-form recovery hints live in `crates/cli/src/lib.rs` (`hint` on `unsupported-version` / `specify-source-required` / `spec-not-generated` / `spec-outdated` and the loader discriminants, attached through `Command::hints`). Adding a new hint extends that lookup, not the error type. Engine descriptions stay transport-neutral — they name the path, adapter, or rule, never a flag, a verb, or "the CLI"; flag-vocabulary recovery text belongs in the hint table.

**Production code does not panic.** The engine and every adapter run as wasm guests, where a panic is a trap — no `Failure` envelope, no exit code — so `unwrap()`, `expect()`, `panic!`, and indexing a position the code has not just checked belong in tests and build scripts alone (there a panic *is* the failure report). An invariant the type system cannot express still fails as an Omnia `Error`: the engine's own defect — a document that does not serialise, a fact an accepted answer names that the brief cannot place — is `server_error!` (exit 3) with a description naming the defect. Library accessors return `Option` or `Result` rather than panicking on a miss (`emery_prose::registry::body`), leaving the caller to report it.

## `#[non_exhaustive]`

**Deliberate override of general library guidance, including the baseline's.** Public enums and structs are exhaustive by default: the workspace treats adding a variant as an ordinary pre-1.0 SemVer-minor event, and exhaustive matching at every consumer is the compile-time drift check the closed taxonomies (journal events, exit codes, lifecycle states) rely on. Reach for `#[non_exhaustive]` only when a type is genuinely open-ended *and* external consumers must keep compiling across additions; document that choice in a doc-line.

## JSON and storage

Structured interchange is JSON (`serde_json`). There is no live YAML path and no `Error::YamlDe` / `Error::YamlSer` variants.

Engine state rides the storage capabilities (`StateStore` / `BlobStore`), never tree writes — blobstore writes are complete-on-finalize, so no atomic-rename helper exists. `fs::write` is reserved for files outside engine state that no other live process reads (one-shot scratch output, fixtures inside a tempdir test).

## Module layout

Use the modern Rust module layout: `<parent>/<module>.rs` is the module entry point and child modules live under `<parent>/<module>/`. **Do not add `mod.rs` files** — `<module>/mod.rs` is the legacy 2018-edition pattern and is forbidden in workspace crates. The single allowed exception is `tests/<helper>/mod.rs`, which is the documented Rust idiom for sharing code between integration test binaries (`tests/<helper>.rs` would be picked up as its own test target). When you split a file, create `<module>.rs` + `<module>/<concern>.rs`; never reach for `<module>/mod.rs`.

```text
crates/foo/src/
├── widget.rs            ← module entry (was widget/mod.rs)
└── widget/
    ├── parse.rs
    └── render.rs
```

There is no module length cap. Split a file when a reader gains a seam — a concern with its own consumers or its own vocabulary — never because it crossed a line count; a type and the one brief or judgment that uses it read better together than apart. When you do split, prefer `<parent>/<module>.rs` + `<parent>/<module>/<concern>.rs` over `// ---` separators inside one file.

## No-op forwarders

A clap-parsed flag that is destructured and silently dropped (`let _ = cli.<flag>;` or pattern matches that never reach a handler) is a YAGNI smell. Either the flag is wired up (the façade's `*Args::decode` carries it into the engine input and the handler reads it) or it is removed from clap.

## Wired-but-ignored flags

A flag whose doc-comment says "Currently equivalent to the default …" or whose handler ignores the value is the same defect as `no-op-forwarders` dressed up as documentation. Drop the flag from the façade's `*Args` until the differentiated behaviour exists.

## Drift audit

When you remove a symbol, run `rg <SymbolName> -- AGENTS.md docs/` and update every hit in the same PR. Stale symbol references in docs are worse than missing docs — they teach the reader something false. Doc drift on internal symbols (error variants, type names, field keys) is caught only by this audit habit.
