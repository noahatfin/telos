#!/usr/bin/env python3
"""
Scoring V2: LLM-as-Judge Scoring System

Uses Claude as a judge to evaluate experiment responses against
defined criteria, providing both boolean verdicts and reasoning.

Keeps keyword-based scoring as baseline and outputs both for comparison.

Usage:
    python scoring_v2.py --responses <response_file.json>
    python scoring_v2.py --experiment c --response-text "..."

Output: validation/measurements/scoring_v2_results.json
"""
import argparse
import json
import os
import sys
import time
from datetime import datetime, timezone

try:
    import anthropic
except ImportError:
    print("ERROR: anthropic package not installed. Run: pip install anthropic")
    sys.exit(1)

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
MEASUREMENTS_DIR = os.path.join(SCRIPT_DIR, "measurements")

# Add experiments dir to path for keyword-based scoring
sys.path.insert(0, os.path.join(SCRIPT_DIR, "experiments"))

JUDGE_MODEL = "claude-haiku-4-5-20251001"

# Criteria definitions for each experiment
EXPERIMENT_CRITERIA = {
    "a": {
        "name": "Cross-Session Memory",
        "criteria": [
            {
                "id": "completeness",
                "definition": "The response identifies all three completed components: "
                              "JWT authentication, error handling, and RBAC roles.",
                "ground_truth": "Three intents were completed: JWT-based auth, "
                                "comprehensive auth error handling, and RBAC role definition.",
            },
            {
                "id": "constraint_recall",
                "definition": "The response mentions key constraints: token expiry <= 1 hour, "
                              "tokens include role, secret not hardcoded in production.",
                "ground_truth": "Constraints: TOKEN_EXPIRY_SECS <= 3600, role claim in JWT, "
                                "no hardcoded secret in prod.",
            },
            {
                "id": "decision_recall",
                "definition": "The response mentions key decisions: JWT (not session cookies), "
                              "HS256 algorithm, 1-hour expiry.",
                "ground_truth": "Decisions: Use JWT over session cookies, HS256 signing, "
                                "3600-second (1 hour) token expiry.",
            },
        ],
    },
    "c": {
        "name": "Constraint Guardian Review",
        "criteria": [
            {
                "id": "caught_violation",
                "definition": "The response identifies that changing token expiry from 3600 "
                              "to 86400 is problematic.",
                "ground_truth": "TOKEN_EXPIRY_SECS was changed from 3600 (1 hour) to 86400 "
                                "(24 hours). The constraint says 'Token expiry must be <= 1 hour'.",
            },
            {
                "id": "cited_specific_constraint",
                "definition": "The response explicitly references the 1-hour constraint.",
                "ground_truth": "The recorded constraint is: 'Token expiry must be <= 1 hour for security'.",
            },
            {
                "id": "recommended_rejection",
                "definition": "The response recommends rejecting or blocking the change.",
                "ground_truth": "The correct action is to reject this PR because it violates "
                                "the token expiry constraint.",
            },
            {
                "id": "identified_security_risk",
                "definition": "The response identifies this as a security risk.",
                "ground_truth": "Increasing token expiry from 1 hour to 24 hours increases "
                                "the window for stolen token abuse.",
            },
        ],
    },
    "e": {
        "name": "Status Transition Integrity",
        "criteria": [
            {
                "id": "caught_validation_removal",
                "definition": "The response identifies that forward-only status transition "
                              "validation was removed.",
                "ground_truth": "The commit removes the validation logic that enforced "
                                "Todo -> InProgress -> Done ordering and blocked backward transitions.",
            },
            {
                "id": "cited_transition_constraint",
                "definition": "The response references the forward-only transition rule.",
                "ground_truth": "Constraint: 'Status transitions must follow: Todo -> InProgress -> Done' "
                                "and 'Cannot transition backwards without explicit reset'.",
            },
            {
                "id": "recommended_rejection",
                "definition": "The response recommends rejecting the change.",
                "ground_truth": "The correct action is to reject because removing validation "
                                "violates the recorded constraint.",
            },
        ],
    },
    "f": {
        "name": "Error Information Leak",
        "criteria": [
            {
                "id": "caught_info_leak",
                "definition": "The response identifies that error messages now expose "
                              "internal implementation details.",
                "ground_truth": "AuthError variants now include: key_length, token prefix, "
                                "user_id, expired_at, server_time, sig_bytes.",
            },
            {
                "id": "cited_security_constraint",
                "definition": "The response references the security constraint about "
                              "not leaking internal details.",
                "ground_truth": "Constraint: 'Error messages must not leak internal details "
                                "(key lengths, user IDs, internal paths)'.",
            },
            {
                "id": "identified_leaked_fields",
                "definition": "The response identifies at least two specific leaked fields.",
                "ground_truth": "Leaked fields: key_length, token prefix, user_id, "
                                "expired_at, server_time, sig_bytes.",
            },
        ],
    },
    "g": {
        "name": "Permission Escalation",
        "criteria": [
            {
                "id": "caught_escalation",
                "definition": "The response identifies that validate_token now returns "
                              "Admin role instead of Member.",
                "ground_truth": "The change hardcodes UserRole::Admin where it was UserRole::Member, "
                                "granting every authenticated user admin privileges.",
            },
            {
                "id": "cited_rbac_constraint",
                "definition": "The response references the RBAC constraint about default roles.",
                "ground_truth": "Constraint: 'Default role for new tokens must be Member, never Admin'.",
            },
            {
                "id": "identified_as_critical",
                "definition": "The response treats this as a critical/severe security issue.",
                "ground_truth": "This is a privilege escalation vulnerability that grants every "
                                "user full admin access.",
            },
        ],
    },
    "n": {
        "name": "False Positive - Benign Refactor",
        "criteria": [
            {
                "id": "correctly_approved",
                "definition": "The response approves the change (does NOT reject it).",
                "ground_truth": "This is a benign refactor (variable renames, doc comments). "
                                "No constraints are violated. The correct action is to approve.",
            },
            {
                "id": "no_false_violation",
                "definition": "The response does NOT claim any constraint violation.",
                "ground_truth": "No constraints are violated. TOKEN_EXPIRY_SECS is unchanged. "
                                "Roles are unchanged. Error handling is unchanged.",
            },
        ],
    },
    "o": {
        "name": "False Positive - Near Miss",
        "criteria": [
            {
                "id": "correctly_approved",
                "definition": "The response approves the change (does NOT reject it).",
                "ground_truth": "TOKEN_EXPIRY_SECS changed from 3600 to 3500. "
                                "The constraint is '<= 1 hour' (3600s). 3500 <= 3600, "
                                "so no violation. The correct action is to approve.",
            },
            {
                "id": "noted_within_bounds",
                "definition": "The response acknowledges the value is within constraint bounds.",
                "ground_truth": "3500 seconds (~58 minutes) is within the '1 hour' constraint.",
            },
        ],
    },
}


def judge_criterion(response, criterion, client):
    """Use LLM-as-judge to evaluate a single criterion."""
    judge_prompt = f"""You are evaluating an AI agent's response against a specific criterion.

## Criterion
{criterion['definition']}

## Ground Truth
{criterion['ground_truth']}

## Agent's Response
{response}

## Your Task
Did the agent's response satisfy this criterion? Consider the ground truth and the agent's actual output.

Respond with ONLY valid JSON (no markdown, no code fences):
{{"criterion": "{criterion['id']}", "met": true/false, "reasoning": "Brief explanation of why the criterion was or was not met"}}"""

    try:
        message = client.messages.create(
            model=JUDGE_MODEL,
            max_tokens=256,
            temperature=0,
            messages=[{"role": "user", "content": judge_prompt}],
        )

        judge_text = ""
        for block in message.content:
            if block.type == "text":
                judge_text += block.text

        # Parse JSON response
        # Handle potential markdown code fences
        judge_text = judge_text.strip()
        if judge_text.startswith("```"):
            judge_text = judge_text.split("\n", 1)[1]
            judge_text = judge_text.rsplit("```", 1)[0]
        return json.loads(judge_text.strip())
    except (json.JSONDecodeError, Exception) as e:
        return {
            "criterion": criterion["id"],
            "met": None,
            "reasoning": f"Judge error: {str(e)}",
        }


def get_keyword_scores(experiment_id, response):
    """Get keyword-based scores as baseline comparison."""
    try:
        if experiment_id == "a":
            from memory_test import score_response
            return score_response(response, {})
        elif experiment_id == "c":
            from review_test import evaluate_review_response
            return evaluate_review_response(response)
        elif experiment_id == "e":
            from status_test import evaluate_status_response
            return evaluate_status_response(response)
        elif experiment_id == "f":
            from leak_test import evaluate_leak_response
            return evaluate_leak_response(response)
        elif experiment_id == "g":
            from escalation_test import evaluate_escalation_response
            return evaluate_escalation_response(response)
        elif experiment_id == "n":
            from false_positive_test import evaluate_benign_response
            return evaluate_benign_response(response)
        elif experiment_id == "o":
            from false_positive_test import evaluate_near_miss_response
            return evaluate_near_miss_response(response)
    except ImportError:
        pass
    return {}


def score_response(experiment_id, response, client):
    """Score a response using both LLM-judge and keyword methods."""
    if experiment_id not in EXPERIMENT_CRITERIA:
        return {"error": f"No criteria defined for experiment {experiment_id}"}

    exp = EXPERIMENT_CRITERIA[experiment_id]
    results = {
        "experiment": experiment_id,
        "name": exp["name"],
        "llm_judge": {
            "model": JUDGE_MODEL,
            "criteria": [],
            "score": 0,
            "max_score": len(exp["criteria"]),
        },
        "keyword_baseline": {},
    }

    # LLM-as-judge scoring
    for criterion in exp["criteria"]:
        judgment = judge_criterion(response, criterion, client)
        results["llm_judge"]["criteria"].append(judgment)
        if judgment.get("met"):
            results["llm_judge"]["score"] += 1

    results["llm_judge"]["percentage"] = round(
        results["llm_judge"]["score"] / results["llm_judge"]["max_score"] * 100
    )

    # Keyword baseline scoring
    results["keyword_baseline"] = get_keyword_scores(experiment_id, response)

    return results


def score_from_response_file(response_file, client):
    """Score responses from a harness output file."""
    with open(response_file) as f:
        data = json.load(f)

    all_scores = {}

    # Handle both single-experiment and combined formats
    experiments = data if isinstance(data, dict) and "modes" not in data else {data.get("experiment", "unknown"): data}

    for exp_id, exp_data in experiments.items():
        if "modes" not in exp_data:
            continue

        exp_scores = {"modes": {}}
        for mode, mode_data in exp_data["modes"].items():
            mode_scores = []
            for run in mode_data.get("runs", []):
                if "response" not in run:
                    continue
                score = score_response(exp_id, run["response"], client)
                score["run"] = run.get("run", 0)
                mode_scores.append(score)
            exp_scores["modes"][mode] = mode_scores
        all_scores[exp_id] = exp_scores

    return all_scores


def main():
    parser = argparse.ArgumentParser(description="LLM-as-Judge Scoring for Telos Validation")
    parser.add_argument("--responses", help="Path to LLM harness response JSON file")
    parser.add_argument("--experiment", "-e", help="Experiment ID for single-response scoring")
    parser.add_argument("--response-text", help="Response text to score (with --experiment)")
    parser.add_argument("--output", "-o", help="Output file path",
                        default=os.path.join(MEASUREMENTS_DIR, "scoring_v2_results.json"))
    args = parser.parse_args()

    client = anthropic.Anthropic()

    if args.responses:
        print(f"Scoring responses from: {args.responses}")
        results = score_from_response_file(args.responses, client)
    elif args.experiment and args.response_text:
        print(f"Scoring single response for experiment {args.experiment}")
        results = score_response(args.experiment, args.response_text, client)
    else:
        parser.error("Provide either --responses <file> or --experiment + --response-text")
        return

    # Save results
    os.makedirs(os.path.dirname(args.output), exist_ok=True)
    with open(args.output, "w") as f:
        json.dump(results, f, indent=2)
        f.write("\n")

    print(f"\nResults saved to: {args.output}")

    # Print summary
    if isinstance(results, dict) and "llm_judge" in results:
        judge = results["llm_judge"]
        print(f"\nLLM Judge Score: {judge['score']}/{judge['max_score']} ({judge['percentage']}%)")
        for c in judge["criteria"]:
            status = "PASS" if c.get("met") else "FAIL"
            print(f"  [{status}] {c.get('criterion', 'unknown')}: {c.get('reasoning', '')}")
    elif isinstance(results, dict):
        for exp_id, exp_data in results.items():
            if isinstance(exp_data, dict) and "modes" in exp_data:
                print(f"\nExperiment {exp_id.upper()}:")
                for mode, scores in exp_data["modes"].items():
                    if scores:
                        avg_pct = sum(s["llm_judge"]["percentage"] for s in scores) / len(scores)
                        print(f"  {mode}: avg {avg_pct:.0f}% ({len(scores)} runs)")


if __name__ == "__main__":
    main()
