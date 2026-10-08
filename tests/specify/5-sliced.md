---
emery: 5
revision: <revision>
---

# Plan

Two slices: sign-in with its session, and orders over it.

## Slice: authentication

ID: SLICE-001
Requirements: [REQ-001, REQ-002]

Delivers sign-in and the session it issues; a session cannot be verified without a sign-in to issue it, so the two stems are one build.

## Slice: orders

ID: SLICE-002
Requirements: [REQ-003, REQ-004]
Types: [orders.line, orders.order]
Depends on: [SLICE-001]

Delivers order creation and cancellation for a signed-in caller; verified by placing and cancelling an order under a session from `authentication`.
