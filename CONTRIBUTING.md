# Contribution Guide

Augentic welcomes contributions to Emery. This guide covers how to set up your environment, run checks, and submit a pull request; for the full workspace map and architectural invariants, see [`AGENTS.md`](AGENTS.md).

## Contents

- [Getting started](#getting-started)
- [Code style](#code-style)
- [Developer's Certificate of Origin](#developers-certificate-of-origin)
- [Pull request procedure](#pull-request-procedure)
- [Conduct](#conduct)

## Getting started

Unless you are fixing a small, well-defined bug, please open a GitHub issue to discuss your proposed change with the maintainers before starting work.

This repository contains two main surfaces:

- **Rust workspace** (`src/`, `crates/`, `examples/`, `tests/`): The `emery` CLI and runtime binary, the engine guest, the adapter contract and SDK crates, and the mock adapters.
- **Documentation and Cursor plugin** (`docs/`, `plugins/emery/`, `.cursor-plugin/`): The mdBook documentation suite (tutorials, how-to guides, explanation, and reference) and the `/emery:*` Cursor skill wrappers.

First-party source and target adapters live in the sibling [`augentic/emery-adapters`](https://github.com/augentic/emery-adapters) repository.

To preview `/emery:*` skill changes against a local build of the CLI:

1. Install the local binary: `cargo install --path . --locked`
2. Launch Cursor with the local plugin directory: `cursor-agent --plugin-dir plugins/emery`

When you modify files under `plugins/emery/`, bump the version in both `.cursor-plugin/marketplace.json` (`metadata.version`) and `plugins/emery/.cursor-plugin/plugin.json` (`version`) in the same pull request.

## Code style

### Rust workspace

- Format code with nightly rustfmt: `make fmt`
- Run host and WebAssembly (`wasm32-wasip2`) clippy passes: `make lint`
- Run the full CI gate before submitting: `make ci`

### Documentation and Cursor skills

- Verify that the documentation book builds cleanly with no broken links: `mdbook build docs`
- Keep `/emery:*` skills thin: check `emery --version`, elicit missing arguments, invoke one `emery` command, and relay the output without adding orchestration logic in the skill prompt.
- Use `kebab-case` for file names and adapter identifiers.

## Developer's Certificate of Origin

All contributions must include acceptance of the [Developer Certificate of Origin (DCO)](https://developercertificate.org/):

```text
Developer Certificate of Origin
Version 1.1

Copyright (C) 2004, 2006 The Linux Foundation and its contributors.
660 York Street, Suite 102,
San Francisco, CA 94110 USA

Everyone is permitted to copy and distribute verbatim copies of this
license document, but changing it is not allowed.


Developer's Certificate of Origin 1.1

By making a contribution to this project, I certify that:

(a) The contribution was created in whole or in part by me and I
    have the right to submit it under the open source license
    indicated in the file; or

(b) The contribution is based upon previous work that, to the best
    of my knowledge, is covered under an appropriate open source
    license and I have the right under that license to submit that
    work with modifications, whether created in whole or in part
    by me, under the same open source license (unless I am
    permitted to submit under a different license), as indicated
    in the file; or

(c) The contribution was provided directly to me by some other
    person who certified (a), (b) or (c) and I have not modified
    it.

(d) I understand and agree that this project and the contribution
    are public and that a record of the contribution (including all
    personal information I submit with it, including my sign-off) is
    maintained indefinitely and may be redistributed consistent with
    this project or the open source license(s) involved.
```

To sign off on your commit (`git commit -s`), include this line at the end of your commit message with your real name and email address:

```text
Signed-off-by: Jane Example <jane@example.com>
```

For legal reasons, anonymous or pseudonymous contributions cannot be accepted. Please open a GitHub issue if you have questions.

## Pull request procedure

Target all pull requests at the `main` branch:

1. Create a feature branch from `main`.
2. [Rebase](https://git-scm.com/book/en/Git-Branching-Rebasing) your branch onto the latest `main`.
3. Run `make ci` (`fmt-check`, `lint`, `test`, `test-docs`, `docs`, `vet`, `deny`) and confirm every check passes.
4. Sign off all commits with `Signed-off-by:` (DCO).

Every pull request requires at least one maintainer review and passing CI before merge. Include tests at the public boundary for any behavioral change:

- **End-to-end CLI scenarios** live in `tests/` (driving `emery_cli::run` over scripted capabilities or `tests/runtime.rs` over the compiled Wasm guest).
- **Crate integration tests** live in `crates/*/tests/` for standalone library contracts (`emery-sdk`, `emery-prose`).
- **Documentation changes** under `docs/` must pass `mdbook build docs`.

## Conduct

We are committed to keeping the Emery community welcoming, respectful, and harassment-free for everyone:

- We welcome participants regardless of gender, sexual orientation, disability, ethnicity, religion, or background.
- Be kind and constructive in code reviews, issues, and discussions.
- Harassment, insults, demeaning behavior, and trolling are not tolerated in public or private project spaces.
- If you experience or witness unacceptable behavior, please contact a member of the core team immediately.

The [Code of Conduct](CODE_OF_CONDUCT.md) applies across all project spaces.
