# Adapter Examples

This directory contains minimal WebAssembly (Wasm) source and target adapters ([`source/`](source/) and [`target/`](target/)) and a live shell walkthrough for testing `emery specify`, `emery apply`, `emery build`, and `emery amend` end to end over the Cursor backend ([`omnia-cursor`](https://github.com/augentic/omnia-backends/tree/main/crates/cursor)).

## Contents

- [Prerequisites](#prerequisites)
- [Build and run](#build-and-run)
  - [Tracing](#tracing)
  - [Backend callbacks](#backend-callbacks)
- [Amend and build again](#amend-and-build-again)
- [Two branches](#two-branches)
- [Start over](#start-over)
- [Host-to-guest tool calls](#host-to-guest-tool-calls)
- [Installing cursor-sdk-bridge](#installing-cursor-sdk-bridge)

---

## Prerequisites

- [`cursor-sdk-bridge`](https://github.com/cursor/sdk-bridge) on `PATH` (see [Installing `cursor-sdk-bridge`](#installing-cursor-sdk-bridge) below).
- `CURSOR_API_KEY` in a `.env` file (or exported in your environment).
- `git` 2.5 or newer on `PATH` (`GIT_BINARY` names another).

---

## Build and run

[`emery.toml`](emery.toml) references the two example adapters as `example:source@0.1.0` and `example:target@0.1.0`, which `emery` loads from `~/.emery/adapters/example_source@0.1.0.wasm` and `~/.emery/adapters/example_target@0.1.0.wasm`. Because `[target]` sets `repository = "."`, revisions and builds are committed into the project's own Git repository, so we run the walkthrough in a temporary scratch project.

```bash
# build the binary and the mock adapters
cargo build --release
cargo build --examples --target wasm32-wasip2 --release

# copy the adapters to the store under their references
mkdir -p ~/.emery/adapters
cp target/wasm32-wasip2/release/examples/source.wasm ~/.emery/adapters/example_source@0.1.0.wasm
cp target/wasm32-wasip2/release/examples/target.wasm ~/.emery/adapters/example_target@0.1.0.wasm

# a scratch project: the fixture tree and the config, sealed as one commit
set -a; source .env; set +a
emery=$PWD/target/release/emery
project=$(mktemp -d)
cp -R examples/docs examples/emery.toml "$project"
cd "$project"
printf '.emery/\n' > .gitignore
git init --quiet && git add --all && git commit --quiet --message "greeting docs"

# generate the specification set and stage it (the project-root emery.toml is discovered)
"$emery" -v specify

# review the staged candidate and its build plan
"$emery" show spec --staged
"$emery" show plan --staged

# seal it: spec/{spec,design,plan}.json at one commit over HEAD, labelled emery/<revision>
"$emery" apply                       # or, in one run: "$emery" -v specify --apply

# the sealed revision is what show and build read from now on
"$emery" show spec
git show --stat emery/<revision>

# build the plan: the layout's Initialise commit, then one merge commit per slice, a verified wave at a time, under the same label
"$emery" -v build

# read the build; the checkout itself is untouched
git log --oneline --first-parent HEAD..emery/<revision>
git diff --stat HEAD emery/<revision>

# a second run has nothing left to build: the label records every slice
"$emery" build

# take the revision and its build onto the branch: the label is a descendant of HEAD, so this fast-forwards
git merge --ff-only emery/<revision>
```

Without `.env`:

```bash
export CURSOR_API_KEY=<Cursor API key>
```

How the build works:

- **Mock target output:** Writes `build/<slice-name>/index.md` and one `REQ-NNN.md` per requirement inside `.emery/vcs/worktrees/<id>`, merges each slice into `.emery/vcs/integration` (using a `union` merge rule on `build/*/index.md`), and verifies each wave through the shell before advancing the `emery/<revision>` branch. See [Wave-based builds and verification](../docs/explanation/wave-builds.md).
- **Repository and state rules:** `.emery/` holds the staged item and temporary worktrees (ignored via `.gitignore`). Sealed specifications live in Git under `spec/` on `emery/<revision>`, never edited by hand ([`spec-mismatch`](../docs/reference/cli/apply.md#spec-is-the-engines)). Uncommitted changes outside `.emery/` fail `build` with [`base-not-sealed`](../docs/reference/cli/build.md#description); a commit on `main` after sealing leaves the label behind `HEAD` and fails `build` with [`spec-not-generated`](../docs/reference/cli/build.md#description) until merged or rebased. See [Revision lifecycle](../docs/explanation/revision-lifecycle.md).
- **Updating cached adapters:** Stored `.wasm` files in `~/.emery/adapters` are cached permanently until overwritten or deleted (`rm ~/.emery/adapters/example_*`). See [Install, pin, and vendor Wasm adapters](../docs/how-to/manage-adapters.md).

### Tracing

The examples above run with `-v`, which sets the tracing level to `debug`. Each additional `-v` raises verbosity (`trace`), while `-q` / `-qq` lowers it.

### Backend callbacks

Extraction, synthesis, and build turns all run through the Cursor backend, with the guest serving reference-tool calls in-process. See [Host-to-guest tool calls](#host-to-guest-tool-calls) below.

---

## Amend and build again

[`emery amend`](../docs/reference/cli/amend.md) extracts change sources against the currently sealed revision, classifies each requirement as `added`, `changed`, `retired`, `conflict`, or `unchanged`, and stages an `Amendment` for `emery show amendment` and [`emery apply`](../docs/reference/cli/apply.md#sealing-an-amendment).

Continue in the scratch project on the branch left by the first build:

```bash
# the docs change
cat > docs/greeting.md <<'EOF'
# Greeting

GET /greeting returns the static string 'hello, world'.

Acceptance:

- The response body is exactly `hello, world`.
EOF
git commit --quiet --all --message "greeting says hello, world"

# amend over the file's sources (the project-root emery.toml is discovered, as under specify): one requirement changed
"$emery" -v amend
"$emery" show amendment              # ## Changed: REQ-001 with Before: and After:

# seal the amendment as the next revision: one commit over HEAD, labelled emery/<revision>, its parent the first
"$emery" apply                       # prints `amends <parent>` beneath the sealed line
git log -1 --format=%B emery/<revision>   # Parent:, Base:, Added: 0, Changed: 1, Retired: 0
"$emery" show spec

# build the amended plan: every slice is built again over the first build, nothing initialised, nothing resumed
"$emery" -v build
git merge --ff-only emery/<revision>

# an inline brief beside the docs: a run naming a source on the command line reads no file, so a bare --config
# reads the project-root emery.toml for its [target] and its [[source]] entries, the brief appended beside `docs`
"$emery" -v amend --config --description "example:source@0.1.0=GET /farewell returns the static string 'bye'. Keep the farewell's phrasebook in PostgreSQL." --apply
"$emery" show spec                   # REQ-002 farewell beside the greeting, the greeting unchanged
"$emery" show design                 # the phrasebook's store under ## APIs and integrations
"$emery" show plan                   # SLICE-002 farewell beside the greeting's slice
"$emery" -v build
git merge --ff-only emery/<revision>

# a brief that contradicts the committed docs: the docs still say hello, world, the brief says howdy, and the two
# sources are one adapter's, so neither outranks the other and the greeting is a conflict the amendment carries
"$emery" -v amend --config --description "example:source@0.1.0=GET /greeting returns the static string 'howdy'."
"$emery" show amendment              # ## Conflict: the greeting [conflict], both statements noted, Operator reconciliation required.
"$emery" apply                       # seals it marked [conflict]: show spec prints the greeting with no body and both statements as notes
"$emery" show spec

# the way out is the next change: make the sources agree (edit docs/greeting.md to say howdy, or drop the brief) and amend again
"$emery" -v amend                    # the docs alone: 0 added, 1 changed — the conflict resolved to the docs' statement
"$emery" show amendment              # ## Changed: Before: [conflict] ... | ..., After: the docs' statement
"$emery" apply
"$emery" show spec                   # the greeting stated again, its notes gone
"$emery" -v build
git merge --ff-only emery/<revision>

# the history: five revisions, each sealed over the last, the conflicted one left unbuilt
git log --oneline --first-parent
git for-each-ref refs/heads/emery/
```

Key behaviours illustrated above:

- **Minimal redrafting:** In the first amendment, only `REQ-001` changed and no design section or slice structure was affected, so zero design or slicing model turns ran (see [`emery amend` limits](../docs/reference/cli/amend.md#limits)). In the second amendment, the inline description added a `farewell` stem and a PostgreSQL `decision` claim, opening the design and slicing turns (`design: overview, apis`, `slices: farewell (new)`).
- **Conflict resolution:** When two sources at the same authority rank disagree, the requirement is marked `[conflict]` with `Note: Operator reconciliation required.`. Updating the sources so they agree and running `emery amend` transitions the `[conflict]` requirement to `changed`. See [Set authority ranks and resolve source conflicts](../docs/how-to/resolve-conflicts.md).
- **CLI flag layering with `--config`:** Passing `--description` or `<adapter>` on the CLI disables automatic `emery.toml` discovery, so `--config` is required to include `[target]` and `[[source]]` from `emery.toml` ([`build-target-required`](../docs/reference/cli/amend.md#description) otherwise). See [Configure local, inline, and remote Git sources](../docs/how-to/configure-sources.md).

---

## Two branches

When two operators amend the same base revision in separate clones or branches and merge in Git, `spec/spec.json` will conflict because each seal writes `spec/` whole. Keep one branch's `spec/` directory whole (`git checkout <branch> -- spec/`), commit the merge, and re-run the other branch's `emery amend` over the merged commit (see [Merge specifications across Git branches](../docs/how-to/merge-branches.md)):

```bash
# a second operator: a clone with no state of its own reads the current revision from the head's tree
git clone --quiet "$project" "$project-b"

# each amends the current revision with a brief of their own and seals it; neither sees the other's seal
"$emery" -v amend --config --description "example:source@0.1.0=GET /farewell returns the static string 'so long'." --apply
( cd "$project-b" && "$emery" -v amend --config --description "example:source@0.1.0=GET /ping returns the static string 'pong'." --apply && git push --quiet origin 'refs/heads/emery/*:refs/heads/emery/*' )
git for-each-ref refs/heads/emery/   # both labels, each one commit over the revision they amend

# merge the first label onto main, then the second: the specification conflicts, since each side wrote it whole
git merge --ff-only emery/<farewell>
git merge emery/<ping>               # CONFLICT (content): spec/spec.json
git checkout emery/<farewell> -- spec/    # keep one side's three documents whole, whatever merged cleanly
git commit --quiet --no-edit
"$emery" show spec                   # the kept side: the farewell says so long, no ping; the clone reads the same once it pulls

# the other side's change, run again as an amendment of the kept revision: sealed over the merge, Parent: the kept
"$emery" -v amend --config --description "example:source@0.1.0=GET /ping returns the static string 'pong'." --apply
"$emery" show spec                   # the farewell and the ping, under the kept revision as parent
git merge --ff-only emery/<revision>
git for-each-ref refs/heads/emery/   # both branches' labels standing beside the new one
```

---

## Start over

Once a repository's current revision has a `parent` (a **lineage**), bare `emery specify` fails with [`lineage-exists`](../docs/reference/cli/specify.md#a-lineage-is-amended-or-started-over) before loading any adapters so you do not overwrite your amendment history accidentally. Pass `--fresh` to intentionally start a new parentless history over `HEAD`:

```bash
# a bare specify over a lineage: refused before any source is loaded, nothing staged
"$emery" specify                     # error lineage-exists: the current revision `<id>` amends `<parent>` ...
                                     # hint: emery amend <adapter>..., or emery specify --fresh <adapter>...

# start over deliberately: the candidate is staged fresh and sealed with no parent over HEAD
"$emery" -v specify --fresh --apply  # staged candidate <id> (fresh), then sealed revision <id> as emery/<id>
git log -1 --format=%B emery/<revision>   # Revision <id> over Base: alone, no Parent: trailer
git branch --list 'emery/*'               # every earlier label still standing beside the new one
"$emery" show spec                   # the specification as the sources alone state it, no notes carried over

# build it: the first slice over the lineage's build, nothing initialised since the layout stands
"$emery" -v build
git merge --ff-only emery/<revision>

# done: leave the scratch project and remove it, run and all
cd - && rm -rf "$project" "$project-b"
```

---

## Host-to-guest tool calls

During model turns, the SDK declares and serves guest tools in-process over `wasi-model` streams (the host writes each `ToolCall` to the session's `calls` stream, and the guest replies with a `ToolResult` carrying the matching correlation ID):

- **Reference tools (`list_docs` and `read_doc`):** Available on extraction, survey, build, and verify turns. Served in-process from the adapter's embedded `PROSE` table (plus `claims.md` and `reconciliation.md` on source turns). System prompts (`extract.md`, `survey.md`, `build.md`, `verify.md`) are excluded from `list_docs`.
- **Workspace write tools (`write_files` and `edit_files`):** Available on `Task::Build` and `Task::Verify` turns only, operating strictly beneath the lent worktree (`.emery/vcs/worktrees/<id>` or `.emery/vcs/integration`) through no-follow path validation (`beneath` and `standing`). See [Security, sandboxing, and gates](../docs/explanation/security-and-sandbox.md#4-safe-file-operations-and-symlink-protection).

---

## Installing cursor-sdk-bridge

```bash
# download and install
curl -fsSL -o /tmp/cursor-sdk-bridge.tar.gz \
  https://github.com/cursor/sdk-bridge/releases/latest/download/cursor-sdk-bridge-standalone-darwin-arm64.tar.gz \
  && tar -xzf /tmp/cursor-sdk-bridge.tar.gz -C /tmp \
  && install /tmp/bin/cursor-sdk-bridge ~/.local/bin/cursor-sdk-bridge

# verify
cursor-sdk-bridge --help
```

See [bridge docs](https://cursor.com/docs/sdk/bridge) for more information.
