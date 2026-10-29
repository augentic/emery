# Adapter Examples

Live `specify`, `build`, and `amend` journey via [omnia-cursor](https://github.com/augentic/omnia-backends/tree/main/crates/cursor): the mock source adapter extracts greeting claims from [docs/](docs/) through the host model, the engine synthesises `spec.md` / `design.md` and slices `plan.md` and stages the three as one candidate, `apply` seals them under `spec/` in the project repository as one commit labelled `emery/<revision>`, and the mock target adapter builds the plan wave by wave — each ready slice in a working copy of its own cut from that commit, merged into the integrated tree in id order and verified by the adapter — under the same label; then the docs change, `amend` reads the change against the sealed revision, `apply` seals the amendment as the next revision over it, and `build` builds the plan again ([below](#amend-and-build-again)).

The adapters live at [source/](source/) and [target/](target/) — the same anatomy as a first-party adapter. The shipped `emery` binary hosts them: [emery.toml](emery.toml) names each as an exact package reference of its own, `example:source@0.1.0` and `example:target@0.1.0`, which the binary reads from its store, `~/.emery/adapters`, as `example_source@0.1.0.wasm` and `example_target@0.1.0.wasm` — the names `cp` writes below. The store lies apart from the project, so a component is never loaded from a tree a run can write, and nothing under the `example` namespace is fetched: a reference the store holds is read from it before any registry is asked. The source input is [docs/](docs/); the specification and the build land as commits in the project repository, which the `[target]` table's `repository = "."` selects, and which is why the journey runs from a scratch project of its own rather than this checkout.

## Prerequisites

- [cursor-sdk-bridge](https://github.com/cursor/sdk-bridge). See [below](#installing-cursor-sdk-bridge) for installation.
- `CURSOR_API_KEY` in a `.env` file
- `git` 2.5 or newer on `PATH` (`GIT_BINARY` names another)

## Build and run

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

The mock target writes a Markdown stand-in for code: `build/<slice-name>/index.md` and one `REQ-NNN.md` per requirement, where a real target writes the implementation. Each slice builds in `.emery/vcs/worktrees/<id>`, and what it wrote is sealed there and merged into `.emery/vcs/integration` as one merge commit whose message carries the slice's trailers (`Slice:`, `Revision:`, `Wave:`, among others); the mock's one merge rule keeps both slices' lines when two write one `index.md`, and a path the rule leaves conflicted is resolved with the model and merged in the slice's place. After each wave the adapter's `verify.md` checks the integrated tree through the shell — every index lists files its directory holds, nothing written outside `build/` — and the label moves to the verified head, so a run that stops is resumed by the next from the last verified wave, and from the slices `.emery/vcs/integration` holds merged since when the stop came before a wave was verified. `--jobs N` caps how many slices build at once. The `.emery/` beneath the project holds the staged candidate and the clones and working copies a run cuts, and is ignored so the checkout stays sealed; the sealed revision lives in the repository itself, under `spec/` at the labelled commit, never in the checkout's tree until the label is merged, and a document edited there is refused by every later verb (`spec-mismatch`). A `build` over a checkout with an uncommitted change is refused as `base-not-sealed`, naming the paths; commit or stash them and run it again. A commit made on `main` after the seal leaves the label behind the head, and `build` refuses as `spec-not-generated` until the label is merged or rebased onto it, or `specify --apply` seals a fresh revision over it. A stored release is final until removed, so after rebuilding a mock copy it in again; `rm ~/.emery/adapters/example_*` takes both out of the store.

The `repository` key of the `[target]` table in [emery.toml](emery.toml) says where the specification lives and the build lands. `"."` is the project checkout, as above. A URL seals into another repository — a brownfield project — over its default branch, in a clone kept under `.emery/vcs/repos/<name>`, `<name>` the table's `name` or the adapter's, and the label stands in the clone for you to read or push. With the key omitted, `apply` creates a repository of its own at the same path on the first seal and keeps it, its root commit the revision's and the adapter's layout sealed over it by the first build — a greenfield project, which the project directory need not be a repository for.

### Tracing

The examples above run with the `-v` flag, which sets tracing level to `debug`. Each additional `v` raises the level, while `-q[q]` lowers it.

### Backend callbacks

*Extract*, *synthesis*, and *build* all run through the Cursor backend. The guest answers reference-tool calls in-process, just like the [omnia-cursor example](https://github.com/augentic/omnia-backends/tree/main/examples/cursor) does.

See [#host-to-guest-tool-calls](#host-to-guest-tool-calls) below for more detail.

## Amend and build again

A sealed revision is amended, never regenerated: `emery amend` reads its sources as a *change* against the current revision, matches each claim to the requirement it revises, withdraws, or adds beside, and stages an `Amendment` — every requirement classed `added`, `changed`, `retired`, `conflict`, or `unchanged`, the specification redrafted for the drafted classes alone, the design and plan derived from the base. `emery apply` seals it as the next revision over the current one, `Parent:` naming the base, and `emery build` builds the plan again under the new label. Continue in the scratch project, on the branch the fast-forward left at the first revision's build.

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

The first amendment reads the docs alone and stages one `changed` requirement; its plan is settled by code, so no slicing turn is put, and a requirement claim informs no design section, so no design turn is put either and the design is the base's — its overview still describes the greeting as the first revision had it, a paragraph that goes stale while nothing it cites is retired. The plan is the base's byte for byte too — the revised requirement keeps its id and its slice, so no slice changes, `show amendment` prints no `## Plan`, and the greeting slice's brief still says what the first revision had it deliver. The second reads the docs beside the inline brief: the docs restate the greeting, which stays `unchanged`, the brief adds the farewell under a stem of its own, and its `decision` opens the design turn, so `show amendment` reports `1 added, 0 changed, 0 retired, 0 conflict, 1 unchanged`, `design: overview, apis`, and `slices: farewell (new)`, and `--apply` seals in the same run, printing the staged block and then the seal lines. A slice the slicing turn finds touched — one that gained, lost, or had redrafted a requirement — has its brief redrafted by that turn; a slice changed by code alone, as one is when a retirement takes a requirement out of it, keeps the base's brief, is marked `(brief kept)` on the `slices:` line, and carries `Note: the brief is the base's; no slicing turn redrafted it.` under its `## Plan` block, so the reviewer knows the brief to read with care. A change that leaves every requirement unchanged and no section or slice different stages nothing. An amendment `specify` would displace is refused `amendment-pending` while one stands staged — review it with `emery show amendment`, seal it with `emery apply`, or run `emery amend` again to replace it — and `apply` over an amendment whose base is no longer the current revision is refused `amendment-stale`; amend again over the revision that is.

A change whose sources disagree over one requirement — two statements at one rank, as the docs and an inline brief through the same adapter are — is a `conflict`, never a choice the engine makes: `show amendment` lists the requirement under `## Conflict` with each competing statement noted beside its source and `Note: Operator reconciliation required.`, and `apply` seals it as a `specify` seals one, the requirement marked `[conflict]` in `show spec` with no body and the statements among its notes, a revision to resolve rather than build. The conflict is resolved by the next change, not by the engine: once the sources agree, or the contradicting one is dropped, `emery amend` lists the standing requirement to the grouping as one entry whose statement is `[conflict]` followed by the competing statements, and the claim that lands on it is a `changed` requirement — `Before:` the competing statements, `After:` the claim's — stated again under the status the change sources alone settle. A source cannot conflict with itself: a `retire` beside the same source's `requirement` under one id is read as the restatement, and only two sources at one rank conflict.

Two rules of the grammar the amendment steps meet. A run that names a source on the command line — a positional `<adapter>` or a `--description` — reads no `emery.toml` unless `--config` says so, so the `[target]` table reaches it under `--config` alone (bare for the project-root file, `--config <path>` for another); without it the run is refused `build-target-required`. And a positional adapter and a `--description` for one adapter bind one source name, the adapter's package name, so the pair is refused as a source named twice: an inline change over an adapter is `--description` alone, or a named `[[source]]` entry.

## Two branches

Carrying on in the same scratch project: two operators amend the one revision apart, each in a checkout of their own, and their labels diverge from one commit. The merge is git's to make and the operator's to resolve, keeping one side's `spec/` whole; the other side's change is then run again as an amendment of what the merge kept.

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

What a verb reads as the current revision is the repository's, and the state under `.emery/` is a cache it bears out or not: the label the last seal in this checkout left is read where it stands over the head, and otherwise the head's own tree is read, as a clone with no state reads it — three documents that read as one revision are that revision, whatever label the repository holds or lacks for them, since a clone sees no remote's label and an operator may delete a merged one. So once two branches are merged with one side's `spec/` kept whole, both checkouts read the kept revision from the head, the other side's label behind it, and an amendment prepared there seals over the merge commit with the kept revision as its parent. A merge of two seals always conflicts in `spec/spec.json`, since each seal writes the specification whole; whether `design.json` and `plan.json` conflict too is the diff's, so the resolution is the same either way: one side's three files whole, never one file from each.

## Start over

Carrying on in the same scratch project, with the current revision an amendment's: the repository holds a lineage, and a `specify` drafted from the sources alone is refused rather than let seal over it unasked. `--fresh` is the deliberate start-over.

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
cd - && rm -rf "$project"
```

A revision whose `parent` names an earlier one is a lineage, and a bare `emery specify` over a `[target]` that holds one fails `lineage-exists` before any adapter is loaded, since a seal over it would read as the next amendment while carrying none of the history's reconciliation; the ways out are `emery amend`, a change against the current revision, and `emery specify --fresh`, which stages the candidate marked fresh and `apply` seals with no `Parent:` trailer, cut over the head whatever revision it holds, so every earlier label stays where it was and the fresh revision's own starts a history of its own. A `[target]` whose current revision has no parent is no lineage, so a first revision is sealed over by a bare `specify` as before, and a run naming no `[target]` reads no repository and is held to nothing. One refusal of the hand's: a `spec/` at the head whose three documents read as no revision — the state a `git merge` of two branches that each sealed a revision leaves, the conflict markers committed — or as no one revision's — one side's `spec.json` kept beside the other side's `design.json` and `plan.json`, so the plan places a requirement the specification does not state, or leaves one it states in no slice, or owns a type the design lacks — is `spec-mismatch` saying the documents were edited or merged by hand, from `show`, `amend`, and `apply` alike, the description naming what failed to read or which ids and keys the documents disagree over; keep one side's three files whole (`git checkout <branch> -- spec/`), commit, and run the other side's amendment again over it, as [above](#two-branches), since the documents are the engine's to write and `emery show` prints them.

## Host-to-guest tool calls

In Emery, the tools a completion session declares are the reference tools — `list_docs` and `read_doc` — over the adapter's embedded prose corpus, and, on a build or verify turn alone, `write_files` and `edit_files` over the lent tree. `wasi-model` delivers them as two streams rather than direct callbacks: the host writes each `ToolCall` to the session's `calls` stream, and the guest answers with a `ToolResult` on a second stream it created and passed to `create`, carrying the same correlation ID so the host can resume the completion.

Every answer is served in-process by the SDK from the adapter's listed `PROSE`: `list_docs` returns the adapter's reference paths and Emery's `reconciliation.md` — never a system document (`extract.md`, `claims.md`, `build.md`, `verify.md`), which a turn either carries already or has nothing to learn from — `read_doc` returns one document body by adapter-relative path, and anything else — an unknown tool, malformed arguments, an unembedded path — comes back as a repairable error. No HTTP shelf, no MCP callback, and no access to the source input or the revision store crosses this boundary; the model reaches nothing but the adapter's own reference documents and, on a build turn, the two writes beneath the lent tree. The lent tree is read through the host's workspace tools; a build turn is lent the slice's working copy, never the checkout, and writes it through `write_files`, served the same way from the guest, one or more files per call beneath the lent root with a `delete` list beside them, a call refused whole under `.emery/`, `.git/`, or the root-level `spec/`, at the projections, and where an entry would leave a file empty or under half its size without saying so, and through `edit_files`, each edit the one place its `find` matches in a file the tree holds; the files the adapter's init scaffolds, once per repository and before any slice, go through the same guest preopen with no turn. A verify turn is lent the integration working copy with the shell and the same two tools: it runs the adapter's checks, repairs what they find, runs them again, and answers a verdict.

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
