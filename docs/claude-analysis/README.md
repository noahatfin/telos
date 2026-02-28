# Claude Analysis — Telos Dialectical Review & Test Suite

Generated: 2026-02-28

This directory contains the process artifacts and analysis from a structured dialectical
review of Telos's validation evidence, plus a comprehensive test suite specification.

## Contents

| File | Description |
|------|-------------|
| `debate-round1-position-papers.md` | All 4 initial position papers (Advocate, Skeptic, Empiricist, Practitioner) |
| `debate-round2-cross-examination.md` | Cross-examination rebuttals with numbered claim references |
| `debate-round3-key-questions.md` | 4 agents x 5 key questions — tabular answers |
| `synthesis.md` | Final synthesis: agreements, disagreements, strongest cases, next steps |
| `test-suite-inventory.md` | Inventory of all 14 test suite files created with descriptions |
| `execution-log.md` | Timeline and process log of the full analysis session |

## Key Outputs (in other locations)

- `docs/REVIEW.md` — The compiled dialectical review document (all rounds + synthesis)
- `validation/llm_harness.py` — Real LLM evaluation harness (Phase A)
- `validation/scoring_v2.py` — LLM-as-judge scoring overhaul (Phase F)
- `validation/experiments/false_positive_test.py` — False-positive experiments N & O (Phase B)
- `validation/experiments/decay_test.py` — Intent decay experiment K (Phase D)
- `validation/experiments/conflict_test.py` — Conflicting requirements experiments P & Q (Phase E)
- `validation/scale/bench_queries.py` — Scale & latency benchmarks (Phase C)
