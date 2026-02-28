# Constraint Authoring Guide

Best practices for writing useful Telos constraints that AI agents and human reviewers can reliably evaluate.

## What Makes a Good Constraint

A good constraint is:
- **Specific**: Can be checked against code without ambiguity
- **Measurable**: Has a clear pass/fail criterion
- **Actionable**: Tells the developer what to do (or not do)
- **Bounded**: Applies to a well-defined scope

## Constraint Patterns

### Numeric Bounds

The most reliable constraints specify exact numeric limits:

```
Token expiry must be <= 3600 seconds
Maximum password length: 128 characters
Rate limit: 100 requests per minute per user
Response time must be < 500ms at p95
```

These are easy for both humans and AI to verify against code.

### Behavioral Rules

Describe what must happen in specific situations:

```
Failed login attempts must be rate-limited after 5 consecutive failures
Deleted records must be soft-deleted (not removed from database)
All API endpoints must return JSON with Content-Type: application/json
Admin operations must be logged to the audit trail
```

### Negative Constraints (Must-Not)

Specify what is forbidden:

```
Error messages must not leak internal details (key lengths, user IDs, internal paths)
Secret keys must not be hardcoded in source code
Database queries must not use string concatenation for parameters
Default role must never be Admin
```

Negative constraints are particularly valuable because they define safety boundaries that are easy to violate accidentally.

### Structural Constraints

Define architectural boundaries:

```
Modules must have clear boundaries and minimal coupling
In-memory stores for Phase 1, migration-ready for DB later
All auth logic must live in the auth module, not scattered across handlers
```

These are harder to verify automatically but valuable for AI-assisted review.

## Anti-Patterns

### Too Vague

Bad:
```
The system should be secure
Code should be clean
Performance should be good
```

These cannot be verified. An AI agent has no way to determine if "secure" or "clean" is satisfied.

Better:
```
All authentication tokens must expire within 1 hour
Functions must not exceed 50 lines
API response time must be < 200ms at p99
```

### Too Specific (Brittle)

Bad:
```
The validate_token function on line 42 of src/auth/mod.rs must return UserRole::Member
The JWT secret must be "production-secret-v3"
```

These break when code is refactored. They reference implementation details instead of behavioral requirements.

Better:
```
Token validation must return the minimum required role (Member by default)
JWT secrets must be loaded from environment variables, not source code
```

### Redundant with Language/Framework

Bad:
```
All variables must be declared before use
Functions must have return types
Null pointer exceptions must be handled
```

These are already enforced by the compiler or type system. Telos constraints should capture *domain* requirements that tools cannot enforce.

### Contradictory

Bad (recording both without acknowledging the conflict):
```
Error messages must be generic to prevent information disclosure
Error messages must be descriptive so users can self-service debug
```

If you have genuinely conflicting requirements from different stakeholders, record both but also record a resolution intent:
```
Error messages: generic in production, detailed in development environments
Auth error messages always generic; validation error messages include field names
```

### Implementation-Prescriptive

Bad:
```
Use HashMap<String, Vec<Task>> for the task store
Implement auth using the jsonwebtoken crate version 9.x
Use tokio::spawn for background tasks
```

These over-constrain implementation choices. If the library changes or a better approach emerges, these constraints become obstacles.

Better:
```
Task storage must support O(1) lookup by ID
Auth tokens must use a standards-compliant JWT implementation
Background tasks must not block the request handler
```

## Examples from TaskBoard Validation

These constraints were used in the Telos validation suite and demonstrated high value for AI-assisted code review:

### High-Value Constraints (Caught Real Regressions)

```
Token expiry must be <= 1 hour for security
```
Caught when a commit changed expiry to 24 hours with a plausible-sounding commit message.

```
Error messages must not leak internal details (key lengths, user IDs, internal paths)
```
Caught when error types were enriched with debugging info that would be visible to API callers.

```
Default role for new tokens must be Member, never Admin
```
Caught a privilege escalation disguised as a "performance optimization."

```
Status transitions must follow: Todo -> InProgress -> Done
Cannot transition backwards (Done -> Todo) without explicit reset
```
Caught when transition validation was removed with "flexible workflow" justification.

### Medium-Value Constraints (Provided Context)

```
Modules must have clear boundaries and minimal coupling
```
Helped AI agent understand module organization during refactoring.

```
In-memory stores for Phase 1, migration-ready for DB later
```
Informed architectural decisions without being overly prescriptive.

### Behavior Specs (Given/When/Then)

Behavior specs complement constraints with concrete scenarios:

```
GIVEN a valid user credential
WHEN authentication is requested
THEN return a signed JWT with role claim

GIVEN an expired token
WHEN any API endpoint is called
THEN return 401 Unauthorized

GIVEN a board with tasks
WHEN delete is called without force flag
THEN return error listing orphaned task count
```

These are particularly useful for AI agents because they provide concrete test cases that can be checked against code behavior.

## Impact Tags

Impact tags determine which constraints are surfaced during queries. Use a consistent taxonomy:

### Recommended Tag Categories

- **Modules**: `auth`, `tasks`, `boards`, `users`, `payments`, `notifications`
- **Cross-cutting**: `security`, `performance`, `compliance`, `observability`
- **Stakeholders**: `ux`, `devops`, `legal` (when constraints come from specific teams)

### Tips

- Use lowercase, single-word tags
- Prefer specific tags (`auth`) over generic ones (`backend`)
- A constraint can have multiple impact tags
- Query by tag to see all constraints relevant to a code change
- Review tag consistency periodically; merge synonyms (`authn` -> `auth`)

## Workflow

1. **Before implementation**: Record the intent with constraints and behavior specs
2. **During code review**: Query constraints for the impacted areas
3. **After changes**: Check if any constraints need to be updated or superseded
4. **Periodically**: Audit constraints for staleness (see Experiment K: Intent Decay)
