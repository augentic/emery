# target.verify

Verify the integrated project tree after a wave of slices has merged into it, and answer a verdict.

## Inputs

- `$WORKSPACE` — the integrated tree, lent with the shell. Read it through the workspace tools and run the checks below in it. Write nothing you mean to keep: the tree is the build's, and a check's by-products are discarded or sealed by the engine, never by you.

Nothing outside `$WORKSPACE` is reachable.

## Checks

The tree holds a Markdown stand-in for code: one directory `build/<slice-name>/` per slice built, each with an `index.md` and one `<REQ-NNN>.md` per requirement it implements. Run each check and read what it prints:

1. **Every slice directory has an index.** For each directory under `build/`, `index.md` exists and is not empty.
2. **Every index lists files the directory holds.** Each `REQ-NNN` an `index.md` names has a `REQ-NNN.md` beside it.
3. **No requirement file is orphaned.** Each `REQ-NNN.md` under `build/<slice-name>/` is named in that directory's `index.md`.
4. **Nothing was written outside `build/`.** `git status --porcelain` in `$WORKSPACE` shows no path outside `build/`.

A shell loop over `build/*/` with `test`, `grep`, and `ls` is enough for the first three; the fourth is one `git status` call.

## Verdict

Answer with one JSON object:

- `passed` — true when every check passed, false otherwise.
- `failures` — one entry per check that failed, naming the check and quoting the tail of what it printed; empty when `passed` is true.

```json
{
  "passed": false,
  "failures": ["check 2: build/greeting/index.md names REQ-002, but build/greeting/REQ-002.md does not exist"]
}
```

Report what the checks found, never what the tree should hold.
