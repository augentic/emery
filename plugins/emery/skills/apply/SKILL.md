---
name: emery-apply
description: Seal the staged candidate as the current revision by invoking `emery apply`, and relay the commit and label it set in the target's repository. Use whenever the operator has reviewed what `emery specify` staged and wants it sealed for `emery build`.
argument-hint: [--config <path>]
---

# Apply Skill

`emery apply` is the one seal verb: it takes the candidate `emery specify` staged in `.emery/storage` and seals its three documents as `spec/spec.json`, `spec/design.json`, and `spec/plan.json` in the repository the `[target]` table names — one commit `Revision <id>` over the repository's head, or over the current revision's commit where one stands over it, labelled `emery/<id>` — clearing the slot. The repository is the one `emery build` lands in: a clone of the `[target] repository` URL kept under `.emery/vcs/repos/<name>`, the project's own when the key is `"."`, or, with none named, a repository the engine creates at that path on the first seal, whose root commit is the revision's. From then on `emery build` and a bare `emery show` read the revision from that repository, and the next `emery specify` diffs against it. Nothing is pushed, and the project checkout is never written: the label stands in the clone, the checkout's own branches, or the created repository for the operator to read or push. This skill installs or refreshes the CLI, elicits the target, invokes the verb, re-projects the sealed revision, and relays what it sealed; it commits nothing itself.

## Invocation

1. **Install or refresh the CLI** — on a machine with no `emery` binary, invoking this skill is consent to install. When `emery` is already on `PATH`, confirm with the operator before reinstalling:

```bash
brew tap augentic/tap
brew install emery
# or: cargo install --git https://github.com/augentic/emery --locked
```

Then run `emery --version` and stop on failure. A seal also needs `git` 2.5 or newer on `PATH`.

2. **Check the candidate and the target** — a seal needs a staged candidate, so run `/emery:specify` first when none stands (`nothing-staged`, exit 2, otherwise), and a `[target]` table in `--config [<path>]` or the project-root `emery.toml` (`build-target-required`, exit 1, otherwise): its `repository` says where the specification lives, and `--repository <url|.>` stands in for that key alone. Under `repository = "."` the project checkout must be a repository with a commit (`repository-required`, `base-not-sealed`); the seal does not need it clean. Offer `emery show spec --staged` / `emery show design --staged` / `emery show plan --staged` when the operator has not reviewed the candidate yet.
3. **Invoke**, in JSON mode, so the relay can read what was sealed:

```bash
emery --format json apply
# or: emery --format json apply --config [<path>]
# or: emery --format json apply --repository .      # into the project checkout, in place of the file's repository
```

Apply puts no model turn: it writes three files and one commit. Tracing follows the plugin rule's *Tracing and output* contract.

## Re-project

After a successful seal, re-project the current revision beside the code, since the candidate the earlier projections rendered is now the revision:

```bash
emery show spec > spec.md
emery show design > design.md
emery show plan > plan.md
```

Never edit `spec/` in the target repository: its three documents are the sealed revision, held to the id their label names, and a hand edit fails every later verb closed (`spec-mismatch`).

## Relay

A successful run's envelope carries `revision`, `parent` (the revision it displaced, omitted on the first seal), `commit`, `label` (`emery/<revision>`), `repository` (`.` for the project checkout, else `./.emery/vcs/repos/<name>`), `kind`, and `diff` against the parent. Relay it, and offer the ways to take it:

- Surface the CLI output verbatim — `sealed revision <id> as emery/<id> at <commit>`, the repository when the seal landed outside the checkout, and the diff line — then the commands that read it, run in `repository` (`git -C .emery/vcs/repos/<name> …` when it is not the checkout): `git show --stat <label>` for the commit, `git merge <label>` to take it onto the branch, which a later `emery build` reads the revision from the head's own tree of.
- Offer `/emery:build` next: it reads the sealed revision from the same repository and builds its plan over the `Revision` commit.
- When the seal landed in a clone, offer to push the label to its origin, and push only when the operator asks, never forced: `git -C .emery/vcs/repos/<name> push origin emery/<revision>`. When it landed in a repository the engine created, the label is that repository's whole history: offer `git -C .emery/vcs/repos/<name> remote add origin <url>` and a push when the operator has somewhere for it to go, and leave the directory alone otherwise, since deleting it discards the revision and every build over it.
- Never commit, amend, rebase, or rewrite anything through this skill: the `Revision` commit is the engine's, its message carrying the revision, its parent, and the base it was sealed over.
- On non-zero exit, surface the structured error and stop; every refusal leaves the candidate staged. A `nothing-staged` failure means no candidate stands: run `/emery:specify` first. A `build-target-required` failure means no `[target]` was found: relay the hint and ask for the table. A `spec-mismatch` failure means `spec/` at the repository's head is not the labelled revision — the operator's own directory, or a document edited under the label: relay the hint (restore with `git checkout emery/<revision> -- spec/`, or move the directory aside) and let the operator decide, never moving or editing it through this skill. A `repository-required` or `base-not-sealed` failure means the `"."` checkout is no repository or has no commit: relay the hint and let the operator seal it. A `revision-not-found` failure means the `[target] repository` names no repository or has no default branch, or `.emery/vcs/repos/<name>` already holds a repository the engine created: relay the hint and let the operator decide, never removing the directory through this skill. A `secret-detected` failure means a document carries a credential: relay the paths and let the operator change the source and re-run `/emery:specify`. A `spec-outdated` failure means the candidate predates this binary's grammar: re-run `/emery:specify`.
