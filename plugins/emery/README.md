# Emery Cursor Plugin

This directory contains the `/emery:*` Cursor plugin, providing thin slash-command wrappers around the `emery` CLI for specifying, amending, sealing, and building software inside Cursor.

## Skills

Every skill is an invoke-and-relay wrapper: it checks for missing arguments, invokes the corresponding `emery` CLI command, and displays the output verbatim. All synthesis, validation, and Git operations live in the `emery` binary.

| Skill | Command | Description |
| --- | --- | --- |
| [specify](skills/specify/SKILL.md) | `/emery:specify` | Extract claims from sources, synthesize `spec`, `design`, and `plan`, stage them as a revision candidate, and preview them with `emery show --staged` (`emery specify`) |
| [amend](skills/amend/SKILL.md) | `/emery:amend` | Match updated sources against the current revision, stage an amendment, report change counts, and offer `emery show amendment` (`emery amend`) |
| [apply](skills/apply/SKILL.md) | `/emery:apply` | Seal the staged candidate or amendment into the target repository under `spec/` on branch `emery/<id>` (`emery apply`) |
| [build](skills/build/SKILL.md) | `/emery:build` | Build and verify every slice of the current plan in waves on branch `emery/<id>` (`emery build`) |

## Previewing a skill locally

Launch Cursor Agent against the working-tree plugin directory to test changes before bumping the marketplace version:

```bash
cursor-agent --plugin-dir plugins/emery
```

To test `/emery:amend` end to end in a project that has an `emery.toml` `[target]` and a sealed revision:

1. Edit a source file or documentation file in the project.
2. Run `/emery:amend emery:documentation@<version> --config` in the Cursor session.
3. Verify that the skill confirms the CLI is installed, runs `emery --format json amend emery:documentation@<version> --config`, prints the staged amendment ID, base revision, and change counts, and offers `emery show amendment` and `/emery:apply` without sealing automatically.

---

## Prior versions

The `v1` delivery workflow (`plan`, `refine`, `execute`, `status`, `finalize`, and the `system-*` loop) is archived at Git tag [`v1`](https://github.com/augentic/emery/tree/v1).
