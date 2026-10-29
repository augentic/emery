---
name: emery-apply
description: Seal the staged candidate or amendment as the current revision by invoking `emery apply`, and relay the commit, label, and parent it set in the target's repository. Use whenever the operator has reviewed what `emery specify` or `emery amend` staged and wants it sealed for `emery build`.
argument-hint: [--config <path>]
---

# Apply Skill

`emery apply` is the one seal verb: it takes whatever stands in `.emery/storage` — the candidate `emery specify` staged, or the amendment [`/emery:amend`](../amend/SKILL.md) staged — and seals its three documents as `spec/spec.json`, `spec/design.json`, and `spec/plan.json` in the repository the `[target]` table names, one commit `Revision <id>` labelled `emery/<id>`, clearing the slot. A candidate seals over the repository's head, or over the current revision's commit where one stands over it, that revision its `Parent:`; an amendment seals over the commit its base was read at, the base revision its `Parent:` and the counts and slices it changed as trailers, so the history is a lineage each revision sealed over the last. The repository is the one `emery build` lands in: a clone of the `[target] repository` URL kept under `.emery/vcs/repos/<name>`, the project's own when the key is `"."`, or, with none named, a repository the engine creates at that path on the first seal, whose root commit is the revision's. From then on `emery build` and a bare `emery show` read the revision from that repository, and the next `emery amend` reads it as the base. Nothing is pushed, and the project checkout is never written: the label stands in the clone, the checkout's own branches, or the created repository for the operator to read or push. This skill installs or refreshes the CLI, elicits the target, invokes the verb, re-projects the sealed revision, and relays what it sealed; it commits nothing itself.

## Invocation

1. **Install or refresh the CLI** — on a machine with no `emery` binary, invoking this skill is consent to install. When `emery` is already on `PATH`, confirm with the operator before reinstalling:

```bash
brew tap augentic/tap
brew install emery
# or: cargo install --git https://github.com/augentic/emery --locked
```

Then run `emery --version` and stop on failure. A seal also needs `git` 2.5 or newer on `PATH`.

2. **Check what is staged and the target** — a seal needs a staged candidate or amendment, so run `/emery:specify` or `/emery:amend` first when nothing stands (`nothing-staged`, exit 2, otherwise), and a `[target]` table in `--config [<path>]` or the project-root `emery.toml` (`build-target-required`, exit 1, otherwise): its `repository` says where the specification lives, and `--repository <url|.>` stands in for that key alone. Under `repository = "."` the project checkout must be a repository with a commit (`repository-required`, `base-not-sealed`); the seal does not need it clean. Offer `emery show spec --staged` / `emery show design --staged` / `emery show plan --staged` when the operator has not reviewed a candidate yet, and `emery show amendment` for an amendment. A candidate over a current revision that itself amends an earlier one is refused (`lineage-exists`): the way on is `/emery:amend`, or `emery specify --fresh` to start the history over deliberately.
3. **Invoke**, in JSON mode, so the relay can read what was sealed:

```bash
emery --format json apply
# or: emery --format json apply --config [<path>]
# or: emery --format json apply --repository .      # into the project checkout, in place of the file's repository
```

Apply puts no model turn: it writes three files and one commit. Tracing follows the plugin rule's *Tracing and output* contract.

## Re-project

After a successful seal, re-project the current revision beside the code, since the candidate or amendment the earlier projections rendered is now the revision:

```bash
emery show spec > spec.md
emery show design > design.md
emery show plan > plan.md
```

Never edit `spec/` in the target repository: its three documents are the sealed revision, held to the id their label names, and a hand edit fails every later verb closed (`spec-mismatch`).

## Relay

A successful run's envelope carries `revision`, `parent` (the revision it amends or displaced, omitted on the first seal and on a fresh one), `commit`, `label` (`emery/<revision>`), `repository` (`.` for the project checkout, else `./.emery/vcs/repos/<name>`), `kind` (`revision` for a candidate, `amendment` for an amendment), `amendment` (the amendment's id, for that kind), and `diff` against the revision it amends or displaced. Relay it, and offer the ways to take it:

- Surface the CLI output verbatim — `sealed revision <id> as emery/<id> at <commit>`, `amends <parent>` beneath it for an amendment, the repository when the seal landed outside the checkout, and the diff line — then the commands that read it, run in `repository` (`git -C .emery/vcs/repos/<name> …` when it is not the checkout): `git show --stat <label>` for the commit, `git log -1 --format=%B <label>` for its message — `Parent:` and `Base:`, and for an amendment the `Added:`, `Changed:`, `Retired:`, and `Slices:` trailers — and `git merge <label>` to take it onto the branch, which a later `emery build` reads the revision from the head's own tree of. For an amendment, say that the base's label still stands beside the new one.
- Offer `/emery:build` next: it reads the sealed revision from the same repository and builds its plan over the `Revision` commit; a revision sealed from an amendment is built whole, every slice again over the earlier build, since nothing is resumed across revisions. A revision sealed with a requirement marked `[conflict]` is one to resolve with the next `/emery:amend` rather than build; say so when the amendment's `conflicts:` line named one.
- When the seal landed in a clone, offer to push the label to its origin, and push only when the operator asks, never forced: `git -C .emery/vcs/repos/<name> push origin emery/<revision>`. When it landed in a repository the engine created, the label is that repository's whole history: offer `git -C .emery/vcs/repos/<name> remote add origin <url>` and a push when the operator has somewhere for it to go, and leave the directory alone otherwise, since deleting it discards the revision and every build over it.
- Never commit, amend, rebase, or rewrite anything through this skill: the `Revision` commit is the engine's, its message carrying the revision, its parent, and the base it was sealed over.
- On non-zero exit, surface the structured error and stop; every refusal leaves the candidate or the amendment staged. A `nothing-staged` failure means nothing stands: run `/emery:specify` or `/emery:amend` first. A `build-target-required` failure means no `[target]` was found: relay the hint and ask for the table. A `lineage-exists` failure means the staged candidate was drafted from the sources alone over a current revision that amends an earlier one: relay the hint (`/emery:amend` to stage a change against it, or `emery specify --fresh` to start over, every earlier label left standing) and let the operator decide. An `amendment-stale` failure means the current revision moved since the amendment was prepared — another seal landed, or the head moved: relay the hint (re-run `/emery:amend`, then apply) and let the operator decide; the stale amendment stays staged until then. A `spec-mismatch` failure means `spec/` at the repository's head is not a sealed revision — the operator's own directory, a document edited under the label, or three documents edited or merged by hand, as a merge of two branches that each sealed one leaves them: relay the hint (restore with `git checkout emery/<revision> -- spec/`, or move the directory aside; or, for a hand merge, keep one side's three files whole with `git checkout <branch> -- spec/`, commit, and run the other side's amendment again) and let the operator decide, never moving or editing it through this skill. A `repository-required` or `base-not-sealed` failure means the `"."` checkout is no repository or has no commit: relay the hint and let the operator seal it. A `revision-not-found` failure means the `[target] repository` names no repository or has no default branch, or `.emery/vcs/repos/<name>` already holds a repository the engine created: relay the hint and let the operator decide, never removing the directory through this skill. A `secret-detected` failure means a document carries a credential: relay the paths and let the operator change the source and re-run `/emery:specify`. A `spec-outdated` failure means the candidate predates this binary's grammar: re-run `/emery:specify`.
