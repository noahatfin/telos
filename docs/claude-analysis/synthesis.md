# Synthesis

## Points of Agreement (All or Most Agents)

1. **The "authority gap" concept is plausible and interesting.** All four agents agree that the pattern of "detect-but-approve" is a real failure mode in code review, and that structured constraints could address it. Disagreement is over whether the evidence demonstrates this.

2. **The current evidence is not empirically rigorous.** All four — including the Advocate — acknowledge that simulated responses, keyword scoring, N=1, and single test application are significant limitations.

3. **Prompt asymmetry is a critical confound.** Three of four agents identify the systematic prompt difference as a major issue. Even the Advocate concedes it "is the strongest objection" and agrees a control with identical prompts should be added.

4. **False-positive testing is a critical gap.** All agents agree the evaluation never tests whether Telos agents over-reject benign changes.

5. **Live LLM evaluation is required, not optional.** Every agent's answer to Q5 includes "run these experiments with real LLM agents." Unanimously identified as the #1 next step.

6. **The content-addressable architecture is sound in principle.** Even critics agree the engineering foundation is well-designed. The `iter_all()` concern is about implementation, not architecture.

## Points of Disagreement

1. **Whether the authority gap has been demonstrated or merely hypothesized.** The Advocate treats simulated results as demonstrating the authority gap. The other three treat them as illustrating an untested hypothesis.

2. **How to interpret Experiment A's failure.** Advocate: intellectual honesty, identifies a boundary condition. Skeptic: undermines the entire value proposition. Both readings are defensible.

3. **Whether prompt asymmetry is a confound or natural usage.** Advocate: it reflects how the tool would naturally be used. Others: it makes the experiment uninterpretable.

4. **Whether intent staleness is manageable.** Advocate: DAG supports supersession, just needs CLI tooling. Others: immutability makes staleness dangerous and potentially net-negative.

5. **Whether Telos is differentiated from simpler alternatives.** Skeptic's [S6]: any structured constraint repository would produce the same advantage. Advocate: Telos's queryable structure provides unique capabilities. Unresolved without comparative testing.

6. **Whether "6/7 Strong Pass" framing is appropriate.** Advocate: successful with limitations. Empiricist: misleading. Practitioner: "proof of concept" would be more accurate.

## Strongest Case FOR Telos

Telos identifies a genuine gap in development tooling: the absence of structured, queryable intent and constraint data for AI agent consumption. The "authority gap" concept maps to a real code review failure mode. The architecture is sound (content-addressable, DAG-structured, inspired by Git). Even critics agree the concept is interesting and engineering is competent. With live LLM experiments, false-positive testing, and workflow integration, Telos could demonstrate real value — particularly in high-stakes domains where missed constraint violations outweigh recording overhead.

## Strongest Case AGAINST Telos

The evaluation claims "Strong Pass (6/7)" but the evidence is circular: the same person wrote scenarios, constraints, agent responses, and scoring rubrics. No real LLM was ever called. The one experiment with genuine measurement (A) failed, showing good commit messages achieve 95% of Telos's recall. Prompt asymmetry means every result could be explained by instruction differences. No false-positive testing. Substantial manual overhead without workflow integration. Immutable intents will become stale and potentially harmful. Simpler alternatives (ADRs, CONSTRAINTS.md) were never compared.

## Recommended Next Steps (Prioritized by Consensus)

1. **Run live LLM experiments** — Re-run C, E, F with real Claude/GPT-4, N>=30 per condition, **identical prompts** for both agents. Report effect sizes with confidence intervals.

2. **Add false-positive experiments** — At least 3 scenarios with benign changes. Target: FP rate <15%.

3. **Test against simplest alternative** — Run same experiments with CONSTRAINTS.md flat file as third condition. Isolates whether value comes from Telos architecture or from having constraints at all ([S6]).

4. **Fix prompt asymmetry** — Both agents get identical instructions. Only context/tools differ.

5. **Implement constraint lifecycle management** — `telos supersede` and `telos deprecate` commands to address staleness ([P2]).

6. **Build workflow integration** — Git hooks that surface constraints at commit/PR time. `telos check` command. Addresses [P1] and [P6].

7. **Replace keyword scoring** — LLM-as-judge or human expert rating. Keep keyword as baseline. Addresses [E2].

8. **Scale test queries** — Benchmark `iter_all()` at 100, 1000, 5000 intents. Add secondary indexes if needed. Addresses [P5].

9. **Reframe evaluation claims** — Change "Strong Pass (6/7)" to "Proof of Concept — mechanism demonstrated, empirical validation pending."
