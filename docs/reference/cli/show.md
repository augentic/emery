# emery show

Print a reviewable artifact of the current revision to stdout.

## Synopsis

```bash
emery show spec
emery show design
```

## Description

The one read verb: renders the named artifact of the current revision — a verifiable, non-authoritative projection of the store, never a second authority. `spec` and `design` are the whole revision; there is no `show sources` or `show receipts`.

Text output is the Markdown projection alone — a deliberate exception to the result-line convention so `emery show spec > spec.md` is the document byte for byte. The projection opens with two lines of front matter, `emery: <grammar>` and `revision: <id>`, then the body rendered from the stored revision. The JSON envelope carries the revision id, the projection, and the typed revision itself (`document`).

Neither the projection nor the revision is edited by hand. Changing the specification means changing a *source* — the intent text, the workspace the adapters extract, or the adapter list — and re-running [`emery specify`](specify.md).

Before any revision is committed the verb fails typed with `spec-not-generated` (exit `2`). A current revision id naming a missing or unreadable revision fails closed with `server_error` (exit `3`) — corruption is never an empty result. A stored revision written under an older grammar than this binary reads fails typed with `spec-outdated` (exit `1`); the next `specify` regenerates over it.

## Options

| Option | Description |
|--------|-------------|
| `spec` \| `design` (positional) | Which reviewable artifact to print. |
| `--format` | Global output format: `json` wraps the projection with its revision id and the typed revision. |

## JSON output

When `--format json` is provided, returns:

- `revision` — the current revision id
- `body` — the Markdown projection, byte for byte what text mode prints
- `document` — the stored revision the projection was rendered from: the typed specification (`emery`, `preamble`, `requirements` with `id`, `subject`, `status`, `covered`, `sources` as `{ source, claim }` pairs, `body`, `losers`, `scenarios`) or design (`emery`, `preamble`, `sections` of `kind` and `blocks`); see [CLI output shapes](../cli-output-shapes.md#emery-show)

## See also

- [`emery specify`](specify.md) commits the revision this verb renders; see the [CLI reference](index.md).
