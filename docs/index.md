# Emery

Emery is a **spec generator**. The v1 delivery engine — the `plan → refine → execute → finalize` workflow, the target-adapter build loop, and the definition loop — is archived at git tag `v1`:

```bash
git worktree add ../emery-v1 v1
```

This book is the [reference](reference/index.md) for the shipped `emery` CLI: the `specify` spec generator, the `build` verb that hands the plan to a target adapter, the `show` read verb, their output shapes, and the deployment profile the binary ships with. Contributor guidance is [`AGENTS.md`](https://github.com/augentic/emery/blob/main/AGENTS.md) and [`CONTRIBUTING.md`](https://github.com/augentic/emery/blob/main/CONTRIBUTING.md) in the repository.
