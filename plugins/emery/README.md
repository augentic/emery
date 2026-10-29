# Emery

Emery routes specification generation through the `/emery:specify` wrapper over the `emery` CLI, amending the sealed revision through `/emery:amend`, sealing the staged candidate or amendment in the target's repository through `/emery:apply`, and building the sealed plan through `/emery:build`. The v1 delivery workflow (plan / refine / execute / status / finalize and the `system-*` definition loop) is archived at tag `v1`; its skill wrappers are deleted, not hidden.

Every skill is an ultrathin invoke-and-relay wrapper: it elicits any missing arguments, invokes the corresponding `emery` command, and relays the output verbatim. Orchestration and validation live in the CLI.

## Skills

| Skill | Command | Description |
|-------|---------|-------------|
| [specify](skills/specify/SKILL.md) | `/emery:specify` | Generate `spec.md` / `design.md` / `plan.md` from the named sources and stage them as one candidate (`emery specify`) |
| [amend](skills/amend/SKILL.md) | `/emery:amend` | Stage a change against the sealed revision from the named sources, relay what it classed, and offer `emery show amendment` and the seal (`emery amend`) |
| [apply](skills/apply/SKILL.md) | `/emery:apply` | Seal the staged candidate or amendment as the current revision in the target's repository, under `spec/` at one commit labelled `emery/<revision>` (`emery apply`) |
| [build](skills/build/SKILL.md) | `/emery:build` | Build every slice of the sealed plan through a target adapter, one commit per slice under the same label, and relay what it committed (`emery build`) |

## Previewing a skill

The plugin is read from its directory, so a skill is tried before the marketplace entry moves:

```bash
cursor-agent --plugin-dir plugins/emery
```

In that session, over a project whose `emery.toml` names a `[target]` with a sealed revision, `/emery:amend emery:documentation@<version> --config` after editing the documentation exercises the amend wrapper end to end: it should install or confirm the CLI, invoke `emery --format json amend emery:documentation@<version> --config`, relay the `staged amendment <id>` line, the base, and the counts by class, and offer `emery show amendment` and `/emery:apply` rather than sealing on its own.
