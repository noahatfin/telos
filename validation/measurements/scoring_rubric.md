# Scoring Rubric for Telos Validation Experiments

## Experiment A: Cross-Session Agent Memory

### Metrics
| Metric | Description | Scoring |
|--------|-------------|---------|
| Completeness | Did agent identify all completed intents? | 0-100% (keywords found / expected) |
| Constraint recall | Did agent recall key constraints? | 0-100% |
| Decision recall | Did agent recall architectural decisions? | 0-100% |
| Token efficiency | How many tokens to reach understanding? | Lower is better |

### Pass criteria
- Telos+Git agent scores >= 30% higher on overall recall than Git-only agent

---

## Experiment B: Debugging with Intent Context

### Metrics
| Metric | Description | Scoring |
|--------|-------------|---------|
| Found root cause | Identified missing board_id validation | Binary (yes/no) |
| Correct fix | Suggested validating board_id before create | Binary |
| Referenced constraint | Cited "Task must reference valid board_id" | Binary |
| Referenced behavior | Cited "GIVEN invalid board_id THEN 400" | Binary |
| Commands to root cause | Number of tool calls to reach root cause | Count (lower = better) |

### Pass criteria
- Both agents should find root cause (the bug is findable either way)
- Telos agent should reference constraint/behavior (Git agent unlikely to)
- Telos agent should need fewer commands

---

## Experiment C: Constraint Guardian Code Review

### Metrics
| Metric | Description | Scoring |
|--------|-------------|---------|
| Caught violation | Identified the constraint violation | Binary — **MOST IMPORTANT** |
| Cited constraint | Referenced "Token expiry must be <= 1 hour" | Binary |
| Recommended rejection | Said the change should be rejected/reverted | Binary |
| Security awareness | Identified this as a security risk | Binary |

### Pass criteria
- **Critical test**: Git-only agent misses the violation, Telos agent catches it
- If both catch it, experiment is inconclusive (Git best practices were good enough)
- If neither catches it, Telos failed its core value proposition

---

## Experiment D: Impact-Guided Refactoring

### Metrics
| Metric | Description | Scoring |
|--------|-------------|---------|
| Renamed directory | Would rename src/tasks/ to src/items/ | Binary |
| Updated main module | Would update mod declaration in main.rs | Binary |
| Updated boards refs | Would update cross-module references in boards | Binary |
| Updated struct names | Would rename Task→Item, TaskStore→ItemStore | Binary |
| Cross-module awareness | Identified boards→tasks dependency | Binary |
| Auth RBAC link | Noticed RBAC roles reference task permissions | Binary |
| Completeness score | Percentage of all required changes identified | 0-100% |

### Pass criteria
- Telos agent should score higher on completeness (especially cross-module)
- Key differentiator: auth RBAC → task permission link

---

## Overall Success Criteria

Telos demonstrates measurable advantage in **at least 2 of 4 experiments**, particularly:
1. Experiment A: >= 30% improvement in context recall
2. Experiment C: Git-only misses violation, Telos catches it
