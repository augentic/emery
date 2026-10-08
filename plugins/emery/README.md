# Emery

Emery routes specification generation through the `/emery:specify` wrapper over the `emery` CLI, and building the committed plan through `/emery:build`. The v1 delivery workflow (plan / refine / execute / status / finalize and the `system-*` definition loop) is archived at tag `v1`; its skill wrappers are deleted, not hidden.

Every skill is an ultrathin invoke-and-relay wrapper: it elicits any missing arguments, invokes the corresponding `emery` command, and relays the output verbatim. Orchestration and validation live in the CLI.

## Skills

| Skill | Command | Description |
|-------|---------|-------------|
| [specify](skills/specify/SKILL.md) | `/emery:specify` | Generate `spec.md` / `design.md` / `plan.md` from the named sources (`emery specify`) |
| [build](skills/build/SKILL.md) | `/emery:build` | Build every slice of the current plan through a target adapter, one commit per slice under the label `emery/<revision>`, and relay what it committed (`emery build`) |
