---
emery: 8
revision: <revision>
---

# Amendment

Base: 3f2a9c0e7d1b4a6f8e5c2d9b0a7f4e1c3b6d8a2f5e9c1b4d7a0f3e6c9b2d5a8f at 9e267c3a1b4d7f0a2c5e8b1d4f7a0c3e6b9d2f5a

1 added, 1 changed, 1 retired, 1 conflict, 1 unchanged.

## Added

### Requirement: greeting.locale

ID: REQ-004
Sources: [brief:greeting.locale, source:greeting.locale]

A greeting request naming a locale is answered in that locale's greeting word.

#### Scenario: Locale named

- **GIVEN** the locale `fr` is known
- **WHEN** `/greeting?locale=fr` is requested
- **THEN** the response is `bonjour`

## Changed

### Requirement: greeting.behaviour

ID: REQ-001
Sources: [brief:greeting.behaviour]

Before: GET /greeting returns the static string 'hello'.

After: GET /greeting returns the caller's name after 'hello', or 'hello' alone with none.

#### Scenario: Greeting with a name

- **WHEN** `/greeting?name=Ada` is requested
- **THEN** the response is `hello Ada`

#### Scenario: Greeting without a name

- **WHEN** `/greeting` is requested
- **THEN** the response is `hello`

## Retired

### Requirement: greeting.farewell

ID: REQ-003
Sources: [brief:greeting.farewell]

GET /farewell returns the static string 'goodbye'.

## Conflict

### Requirement: greeting.rate-limit [conflict]

ID: REQ-005
Sources: [brief:greeting.rate-limit, notes:greeting.rate-limit]

Note: brief (documentation, rank 2, greeting.rate-limit): A caller is answered at most ten times a minute.
Note: notes (documentation, rank 2, greeting.rate-limit): A caller is answered at most a hundred times a minute.
Note: Operator reconciliation required.

#### Scenario: Caller over the limit

- **WHEN** a caller exceeds the limit within a minute
- **THEN** [unknown]

## Unchanged

### Requirement: greeting.refuse-empty

ID: REQ-002
Sources: []

A greeting request naming an empty caller is refused.

## Design

Changed: Domain model.

### Domain model

Type: greeting.greeting
```
interface Greeting { word: string }
```

## Plan

### Slice: farewell [removed]

ID: SLICE-002
