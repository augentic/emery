# Specification

Sign-in, its session, and orders over both.

### Requirement: auth.login [unknown]

ID: REQ-001
Sources: [docs:auth.login]
Status: unknown

Users sign in with a credential.

Note: acceptance criteria not evidenced.

#### Scenario: Login

- **WHEN** a valid credential is presented
- **THEN** the caller is signed in

### Requirement: session.timeout [unknown]

ID: REQ-002
Sources: [docs:session.timeout]
Status: unknown

Sessions expire after an hour of inactivity.

Note: acceptance criteria not evidenced.

#### Scenario: Timeout

- **WHEN** a session is idle for an hour
- **THEN** it times out
