# ADR 002: Query Design and Indexing Strategy

## Status

Accepted

## Context

Telos needs to query intents by impact area (e.g., "find all intents affecting auth"). The query must be fast enough for interactive use (sub-second) and integrate into developer workflows like pre-commit hooks and CI pipelines.

## Decision

### O(n) Scan for Phase 1

The current implementation uses a linear scan over all objects in `.telos/objects/`:

1. List all files in the fan-out directory structure
2. Deserialize each JSON object
3. Filter by the requested impact tag
4. Return matching intents

This is deliberately simple. The O(n) scan is acceptable because:
- Most projects will have 10-1000 intents in Phase 1
- Each intent is 1-5 KB of JSON, so even 1000 intents is 1-5 MB of I/O
- Modern SSDs read small files at >100K IOPS, so 1000 files takes <10ms
- The fan-out directory structure prevents filesystem bottlenecks

### When Indexing Is Needed

Based on benchmarking targets, indexing becomes necessary when:

| Intent Count | Expected Scan Time | Acceptable? |
|-------------|-------------------|-------------|
| 100         | <10ms             | Yes         |
| 500         | <50ms             | Yes         |
| 1,000       | <500ms            | Marginal    |
| 2,000       | <1000ms           | No          |
| 5,000       | <2000ms           | No          |

Pass criteria from the validation suite:
- **<500ms at 1,000 intents**
- **<2,000ms at 5,000 intents**

The threshold for adding indexing is approximately 1,000-2,000 intents, where scan times approach user-perceptible latency.

### Future Indexing Approaches

When O(n) scan becomes insufficient, these approaches are planned in priority order:

#### 1. Secondary Impact Index

A file `.telos/indexes/impact.json` mapping impact tags to object IDs:

```json
{
  "auth": ["c3b941a7...", "9f3e2a3f...", "fd84cde9..."],
  "security": ["c3b941a7...", "0bf8ed22..."],
  "tasks": ["fd84cde9...", "3f4509c8..."]
}
```

- Rebuilt on `telos init` or `telos reindex`
- Updated incrementally on `telos intent`
- Reduces query to O(k) where k = matching intents

#### 2. Bloom Filters

For very large intent stores (>5,000), a bloom filter per impact tag provides probabilistic membership testing:

- False positive rate of 1% at 10,000 intents with 128 KB filter
- Eliminates most non-matching objects from the scan
- Complements the secondary index for multi-tag queries

#### 3. Inverted Index

For full-text search across intent statements and constraints:

- Tokenize statement and constraint text
- Build term -> object ID mapping
- Support queries like `telos query search "token expiry"`
- Could use a lightweight library like tantivy (Rust) for BM25 ranking

## Trade-offs

### Simplicity vs. Performance

**Benefit:** O(n) scan requires zero index maintenance, zero additional storage, and zero consistency concerns. The implementation is ~20 lines of Rust.

**Cost:** Performance degrades linearly with intent count.

**Mitigation:** The validation suite includes scale benchmarks that will signal when indexing is needed. The transition can be transparent to users.

### Index Consistency

**Benefit of no index:** Content-addressable storage is always consistent — the objects are the truth.

**Cost of adding indexes:** Indexes can become stale if objects are added outside of Telos (e.g., manual file copy). A `telos reindex` command handles this, but adds operational overhead.

**Mitigation:** The secondary index is purely a cache. If it is missing or stale, Telos falls back to O(n) scan automatically.

## Consequences

- Phase 1 queries work with zero configuration
- Scale benchmarks in CI catch performance regressions
- The indexing strategy can be implemented incrementally without changing the storage format
- All query paths must remain correct even without indexes (indexes are optimizations, not requirements)
