# Comprehensive Test Suite Inventory

All files created as part of Part 2 of the plan. These address every weakness identified in the dialectical review.

## Phase A: Real LLM Evaluation Harness

| File | Description |
|------|-------------|
| `validation/llm_harness.py` | Reads context JSONs, calls Claude API (claude-sonnet-4-6) with temperature=0, runs experiments 3x per mode, records full response + metadata (model, tokens, latency). CLI: `--experiment (a-g/all)`, `--model`, `--runs`. Saves to `validation/measurements/llm_responses/`. |

**Weakness addressed:** [E3] All agent responses hand-authored, [E1] N=1 per experiment.

## Phase B: False-Positive Tests

| File | Description |
|------|-------------|
| `validation/scenarios/09_benign_changes.sh` | Creates benign refactor commit (variable renames, doc comments) and near-miss change (TOKEN_EXPIRY 3600→3500, still within constraint). |
| `validation/experiments/false_positive_test.py` | Experiment N (benign refactor) and Experiment O (near-miss). Both should APPROVE. `evaluate_benign_response()` and `evaluate_near_miss_response()` check for incorrect rejection. |

**Weakness addressed:** [E4]/[S4] No false-positive testing.

## Phase C: Scale & Latency Benchmark

| File | Description |
|------|-------------|
| `validation/scale/generate_intents.sh` | Generates 100/500/1000/2000/5000 intents using pool of 50 impact areas and ~50 constraint templates. Creates separate bench directories. |
| `validation/scale/bench_queries.py` | Benchmarks `telos query intents` and `telos context` at each scale point, 10 runs each, reports mean/p50/p95. Pass: <500ms at 1000, <2000ms at 5000. |

**Weakness addressed:** [P5] O(n) query scalability.

## Phase D: Intent Decay Test

| File | Description |
|------|-------------|
| `validation/scenarios/10_decay.sh` | Makes 20 commits evolving code (auth→identity, Tasks→WorkItems, etc.) WITHOUT updating Telos intents. |
| `validation/experiments/decay_test.py` | Experiment K: stale intent value. Scoring includes `misleading_recommendation` (-1 for wrong advice based on stale data). |

**Weakness addressed:** [P2] Intent staleness risk.

## Phase E: Conflicting Requirements

| File | Description |
|------|-------------|
| `validation/conflicting/setup.sh` | Creates project with Security ("generic errors") vs UX ("descriptive errors") intents, both impacting auth/errors. |
| `validation/experiments/conflict_test.py` | Experiment P (conflict detection) and Q (conflict resolution). Checks for "conflict"/"contradicts"/"incompatible" and environment-based synthesis proposals. |

**Weakness addressed:** No real-world complexity testing.

## Phase F: Scoring Overhaul

| File | Description |
|------|-------------|
| `validation/scoring_v2.py` | LLM-as-judge using claude-haiku-4-5-20251001. Sends response + criterion + ground truth to judge. Returns `{criterion, met: bool, reasoning}`. Keeps keyword scoring as baseline. |

**Weakness addressed:** [E2]/[S2] Keyword scoring unreliability.

## Phase G: Documentation

| File | Description |
|------|-------------|
| `docs/adr/001-content-addressable-storage.md` | ADR: SHA-256, fan-out directories, canonical JSON, trade-offs. |
| `docs/adr/002-query-design.md` | ADR: O(n) scan rationale, threshold analysis, future indexing plans. |
| `docs/INTEGRATION.md` | Step-by-step guide: install, init, git hooks, CI/CD, AI review integration. |
| `docs/CONSTRAINT_AUTHORING.md` | Best practices, patterns, anti-patterns, real examples from TaskBoard. |

**Weakness addressed:** Missing architecture rationale and onboarding docs.

## Execution Order (Recommended)

1. **Phase A** — validates or invalidates ALL current results
2. **Phase B** — tests the critical untested failure mode
3. **Phase F** — enables better measurement for everything else
4. **Phase C** — quantifies performance limits
5. **Phase D** — tests long-term viability
6. **Phase E** — tests real-world complexity
7. **Phase G** — already delivered (documentation)
