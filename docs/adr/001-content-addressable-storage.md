# ADR 001: Content-Addressable Storage

## Status

Accepted

## Context

Telos needs a storage model for intents, decisions, and behaviors that supports:
- Immutable records (intents should never be silently changed)
- Deduplication (identical intents produce the same ID)
- Integrity verification (detect corruption or tampering)
- Simple implementation (no database required for Phase 1)

## Decision

We use content-addressable storage with the following design:

### SHA-256 Hashing

Each object is identified by the SHA-256 hash of its canonical JSON representation. We chose SHA-256 because:
- Widely available in every language and OS
- No known practical collision attacks
- 32-byte digest fits well in file paths and log output
- Git uses SHA (SHA-1, migrating to SHA-256), so the mental model is familiar

We considered SHA-1 (faster but weakened) and BLAKE3 (faster, newer, but less universal). SHA-256 provides the best balance of security, ubiquity, and longevity.

### Canonical JSON Serialization

Before hashing, objects are serialized to canonical JSON:
- Keys sorted alphabetically
- No trailing commas or whitespace variations
- UTF-8 encoding
- Deterministic floating-point representation (not currently used, but specified)

This ensures the same logical object always produces the same hash, regardless of which tool or language created it.

### Fan-Out Directory Structure

Objects are stored in a fan-out directory structure under `.telos/objects/`:

```
.telos/objects/
  ab/
    ab1234...json
  cd/
    cd5678...json
```

The first two hex characters of the hash form the subdirectory name. This prevents any single directory from accumulating too many entries, which degrades filesystem performance on many operating systems (particularly ext4 with large directories).

The threshold where this matters varies by filesystem, but 256 subdirectories with uniform distribution keeps each directory small even at tens of thousands of intents.

## Trade-offs

### Immutability vs. Flexibility

**Benefit:** Once an intent is recorded, it cannot be changed without producing a new hash. This creates a verifiable audit trail.

**Cost:** Updating an intent requires creating a new object and linking it as a child of the original (via the `parents` field). This means "editing" an intent is actually "superseding" it.

**Mitigation:** The `parents` field creates an explicit evolution chain. Tools can follow the chain to show the latest version while preserving full history.

### Storage Growth

**Benefit:** Nothing is ever deleted, so historical analysis is always possible.

**Cost:** Storage grows monotonically. A project with heavy intent churn will accumulate objects.

**Mitigation:** At typical intent sizes (1-5 KB each), even 10,000 intents consume only 10-50 MB. For Phase 1, this is negligible. A future `telos gc` command could prune unreachable objects.

### Deduplication

**Benefit:** Identical intents (same statement, constraints, impacts) automatically share the same hash. Recording the same intent twice is a no-op.

**Cost:** Near-identical intents (differing by a single character) get completely different hashes — no delta compression.

**Mitigation:** This is acceptable for structured metadata. If storage becomes a concern, a pack-file format (similar to Git's) could be introduced later.

## Consequences

- All Telos data is portable: copy `.telos/` to move a project's intent history
- Integrity can be verified by re-hashing any object and comparing to its filename
- Objects are safe to replicate, cache, or distribute without conflict
- The `parents` field provides a DAG structure for intent evolution
- No locking is needed for concurrent writes (hash collisions are astronomically unlikely)
