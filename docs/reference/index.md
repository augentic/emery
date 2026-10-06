# Reference

The lookup table for the shipped `emery` surface. The v1 delivery workflow's reference pages (plan, slice, adapters, artifacts, lifecycle) are archived at git tag `v1` alongside the code they described.

## Sections

- [CLI Reference](cli/index.md) — the shipped verbs.
- [CLI output shapes](cli-output-shapes.md) — the JSON envelopes and exit contract every verb honours.
- [Adapters](adapters.md) — the package reference a run names an adapter by, the store the binary reads it from, and how a release gets there.
- [Deployment profiles](deployment-profiles.md) — how the runtime binds engine storage and mounts the project, and how other deployments swap those bindings.
