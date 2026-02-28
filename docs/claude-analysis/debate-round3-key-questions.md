# Round 3: Five Key Questions

## Tabular Summary

| Question | Advocate | Skeptic | Empiricist | Practitioner |
|----------|----------|---------|------------|--------------|
| Q1: Prove useful? | Mechanism yes, product no | No — two strings scored differently | No — demonstration coherence, not validation | No — plausible mechanism, not probative |
| Q2: Authority gap? | Real, magnitude uncertain | Indistinguishable from artifact | Indistinguishable from artifact | Real phenomenon, artificial measurement |
| Q3: Real LLMs? | Probably directional match | Git-only responses may be too generous | Almost certainly not in ways that matter | Partially — subtler cases more plausible |
| Q4: Overhead worth it? | Not yet, but fixable with integration | Not as demonstrated | Empirical question not addressed | Not at current tooling maturity |
| Q5: Change your mind? | No diff in live experiments, or staleness net-negative | Real LLMs + FP tests + vs CONSTRAINTS.md | N>=30, identical prompts, FP rate <15% | Live agents + longitudinal study + telos check |

---

## Full Answers

### Q1. Do the current experiments actually prove Telos is useful?

**Advocate:** They prove the mechanism, not the product. The experiments demonstrate that structured constraint data changes the information available to a reviewer in ways that shift decisions from approve to reject. They do not prove that real LLM agents would exhibit the same behavioral shift, which requires live-agent follow-up. The correct framing is: the experiments validate the hypothesis that queryable constraints provide decision-relevant context that Git alone does not, while leaving the behavioral claim as a well-supported prediction rather than an established fact.

**Skeptic:** No. The experiments prove that the developer can write two different strings and score them differently using keyword matching. Since all seven experiments use hand-authored simulated responses rather than actual LLM outputs, the results are predetermined by authorial choice. The one experiment where the methodology could have produced a genuine surprise — Experiment A — failed, with git-only scoring 95% vs. Telos's 100%.

**Empiricist:** No. The experiments demonstrate that a human can write two plausible code reviews — one approving, one rejecting — and that a keyword rubric can distinguish between them. They do not demonstrate that real AI agents behave differently when given Telos context versus Git-only context, because no AI agent was ever invoked. "Strong Pass (6/7)" is a claim about demonstration coherence, not empirical validation.

**Practitioner:** No. The experiments demonstrate a plausible mechanism — structured constraints give agents something authoritative to cite when rejecting changes — but the evidence is not probative. Hand-authored responses, asymmetric prompts, keyword scoring. The concept has a defensible rationale; the tool as built is unvalidated.

### Q2. Is the "authority gap" a real advantage or a measurement artifact?

**Advocate:** Real, though magnitude is uncertain. The qualitative difference between "detect-but-approve" and "detect-and-reject" maps to a well-documented code review failure mode. The structural explanation — that agents need authoritative external references to override plausible commit messages — is coherent and testable.

**Skeptic:** Currently indistinguishable from a measurement artifact. Three confounded variables (data, prompts, authorial intent) cannot be separated. An untested conjecture presented as a finding.

**Empiricist:** Currently indistinguishable from a measurement artifact. Until prompt asymmetry is eliminated and real LLM outputs collected, it could be entirely explained by prompt priming rather than data quality. Theoretically plausible but empirically undemonstrated.

**Practitioner:** Real phenomenon wrapped in artificial measurement. The pattern exists in real code review. The 6/7 pass rate as evidence is unreliable since outcome was determined at authoring time.

### Q3. Would real LLMs behave like the simulated agents?

**Advocate:** Probably in direction, though not in exact magnitude. Real LLMs are known to be influenced by system prompt framing and available context. The specific concern is that real LLMs might be more aggressive at catching violations even without Telos (narrowing the gap), or might ignore constraint data when it conflicts with a plausible commit message. The live-agent evaluation is required, not optional.

**Skeptic:** The git-only simulated responses are likely too generous to the Telos thesis. Modern LLMs are quite good at catching security issues from diffs alone — a `Member -> Admin` change or `3600 -> 86400` next to a comment saying "CONSTRAINT: must be <= 1 hour" would likely trigger rejection without Telos. Real experiments could produce opposite results.

**Empiricist:** Almost certainly not in the ways that matter most. Real LLMs are more variable, more hedging, and less binary. The actual distribution across 30+ trials might show overlapping outcomes rather than clean separation.

**Practitioner:** Partially. Telos agent behavior in Experiment C is realistic. The more interesting question is whether git-only agents would really approve — modern LLMs would likely flag a 24x constant increase. The experiments where Telos would most plausibly help are subtler cases — Experiment F (error info leak) and Experiment D (cross-module RBAC dependency) — not the ones with obvious numerical red flags.

### Q4. Is the overhead of maintaining intents worth the benefit?

**Advocate:** For the current Phase 1 with manual CLI, probably not for most teams — [P1] is valid. But the overhead question is about the current UX, not the concept. If intent capture were integrated into the natural workflow (hooks, PR extraction, LLM inference), the marginal cost drops dramatically. The benefit is clearest for high-stakes domains (security, compliance, safety-critical systems).

**Skeptic:** Not as demonstrated. Substantial manual ceremony with unproven benefit and unacknowledged cost. Never compared against lighter alternatives.

**Empiricist:** An empirical question that the evaluation does not address. Without false-positive rates and workflow overhead data, the cost-benefit calculus cannot be computed. It is entirely possible the benefit is real but the overhead makes it net-negative.

**Practitioner:** Not at current tooling maturity. Every developer pays overhead on every change; benefit materializes only when an agent queries the right impact area during review of a violating change. Improves dramatically with git hooks, auto-surfacing, and intent supersession. Without those, overhead outweighs benefit.

### Q5. What evidence would change your mind?

**Advocate:** (1) Live-agent experiments with identical prompts showing no difference in rejection rates → authority gap is simulation artifact. (2) Longitudinal study showing staleness causes more false rejections than presence prevents violations → net-negative.

**Skeptic:** (1) Real LLM experiments, N>=30, identical prompts, statistical significance. (2) False-positive experiments with <15% FP rate. (3) Comparison against CONSTRAINTS.md flat file to isolate Telos vs. just having constraints.

**Empiricist:** Experiments C, E, F with real LLMs, N>=30, identical prompts, effect sizes with confidence intervals. Additionally, at least 3 false-positive experiments showing FP rate <15%.

**Practitioner:** (1) Live LLM agents using Telos tools interactively, showing measurable review accuracy difference. (2) Longitudinal study, 5+ devs, 3+ months, measuring both benefit and maintenance cost. (3) Working `telos check` or git pre-commit hook proving friction reduction.
