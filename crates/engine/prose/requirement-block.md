# Requirement block

Every requirement in `spec.md` is one block the engine renders from the requirement and your draft: heading, three provenance lines, the body, templated notes, and at least one scenario. You draft the scenarios for each listed subject; everything else — including the body — is rendered.

## What the engine renders

```markdown
### Requirement: <subject>[ <tag>]

ID: REQ-<NNN>
Sources: [<source>:<claim>, <source>:<claim>, …]
Status: <agreed|unknown|conflict|divergence>

<the winning claim's statement, verbatim; none for a requirement in conflict>

Note: <templated loser and gap lines, where the requirement has them>

#### Scenario: <your scenario name>

- **GIVEN** <your context, optional>
- **WHEN** <your trigger or input>
- **THEN** <your expected behaviour>
- **AND** <your follow-on outcome, optional>
```

## What you draft

One entry per requirement listed under *Requirements (draft one entry per subject)*, keyed by its `subject` exactly as listed:

- **`scenarios`** — at least one, each with a `name`, optional `given` lines, a `when`, a `then`, and optional `and` lines that follow the `then`, all single lines. Draft from the `criterion` claims covering the requirement; for an uncovered requirement, a scenario that states what is checked, with `then` as `[unknown]` rather than an invented outcome. For a requirement in `conflict` (see [authority.md](authority.md)), the scenario must not pick a side.

The engine refuses a draft that omits a listed requirement, drafts a subject that is not listed, drafts a subject twice, omits a scenario, opens a preamble paragraph line with `#`, `ID:`, `Sources:`, `Status:`, `Note:`, or `Type:`, or drafts boilerplate: a `when` or a `then` that is the requirement's statement (case and punctuation aside), a `then` shared across requirements, or an `[unknown]` `then` for a covered requirement.

## Scenario conventions

- **Verbatim source language where possible.** Draw the trigger and the outcome from the `criterion` and `requirement` claims; do not paraphrase behaviour into a different behaviour.
- **A trigger, not a restatement.** The `when` names the event or input that starts the scenario. The engine refuses the requirement's statement itself as a `when`; a paraphrase of it passes the gate and is no better, so do not draft one.
- **An outcome, not a refrain.** The `then` states what this scenario observes once the trigger has run — the response, the record, the error. The engine refuses the statement itself as a `then`, and one fixed phrase across requirements; a paraphrase of the statement passes the gate and is no better.
- **No commentary about provenance.** Winners, losers, and gaps are the engine's notes; a scenario never restates them.
- **No invented outcomes.** Where no criterion evidences the outcome, state what is checked and let `then` be `[unknown]`.
