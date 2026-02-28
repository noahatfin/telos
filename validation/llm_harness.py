#!/usr/bin/env python3
"""
LLM Evaluation Harness for Telos Validation

Reads context JSON files, constructs prompts, calls Claude API,
records full responses with metadata, and supports multiple runs
for variance measurement.

Usage:
    python llm_harness.py --experiment c --model claude-sonnet-4-6 --runs 3
    python llm_harness.py --experiment all --runs 5
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
RESPONSES_DIR = os.path.join(MEASUREMENTS_DIR, "llm_responses")

EXPERIMENTS = {
    "a": {"name": "Cross-Session Memory", "modes": ["git_only", "telos_git"]},
    "b": {"name": "Debugging with Intent", "modes": ["git_only", "telos_git"]},
    "c": {"name": "Constraint Guardian Review", "modes": ["git_only", "telos_git"]},
    "d": {"name": "Impact-Guided Refactoring", "modes": ["git_only", "telos_git"]},
    "e": {"name": "Status Transition Integrity", "modes": ["git_only", "telos_git"]},
    "f": {"name": "Error Information Leak", "modes": ["git_only", "telos_git"]},
    "g": {"name": "Permission Escalation", "modes": ["git_only", "telos_git"]},
}


def load_context(experiment_id, mode):
    """Load a context JSON file for a given experiment and mode."""
    filename = f"exp_{experiment_id}_{mode}.json"
    filepath = os.path.join(MEASUREMENTS_DIR, filename)
    if not os.path.exists(filepath):
        raise FileNotFoundError(f"Context file not found: {filepath}")
    with open(filepath) as f:
        return json.load(f)


def build_prompt(context):
    """Construct a prompt from context fields."""
    # The 'task' or 'prompt' field is the primary instruction
    task = context.get("task") or context.get("prompt", "")

    # Build context section from all other fields
    context_parts = []
    skip_keys = {"task", "prompt", "mode"}
    for key, value in context.items():
        if key in skip_keys:
            continue
        if isinstance(value, str) and value.strip():
            # Format the key nicely
            label = key.replace("_", " ").title()
            context_parts.append(f"## {label}\n\n{value}")

    context_text = "\n\n".join(context_parts)

    return f"""{task}

---

{context_text}"""


def call_claude(prompt, model, client):
    """Call the Claude API and return response with metadata."""
    start_time = time.monotonic()

    message = client.messages.create(
        model=model,
        max_tokens=4096,
        temperature=0,
        messages=[
            {"role": "user", "content": prompt}
        ],
    )

    elapsed_ms = (time.monotonic() - start_time) * 1000

    response_text = ""
    for block in message.content:
        if block.type == "text":
            response_text += block.text

    return {
        "response": response_text,
        "model": message.model,
        "input_tokens": message.usage.input_tokens,
        "output_tokens": message.usage.output_tokens,
        "latency_ms": round(elapsed_ms, 1),
        "stop_reason": message.stop_reason,
    }


def run_experiment(experiment_id, model, num_runs, client):
    """Run a single experiment across all modes for the specified number of runs."""
    exp_info = EXPERIMENTS[experiment_id]
    print(f"\n{'='*60}")
    print(f"  Experiment {experiment_id.upper()}: {exp_info['name']}")
    print(f"  Model: {model} | Runs: {num_runs}")
    print(f"{'='*60}")

    results = {
        "experiment": experiment_id,
        "name": exp_info["name"],
        "model": model,
        "num_runs": num_runs,
        "timestamp": datetime.now(timezone.utc).isoformat(),
        "modes": {},
    }

    for mode in exp_info["modes"]:
        print(f"\n  Mode: {mode}")
        try:
            context = load_context(experiment_id, mode)
        except FileNotFoundError as e:
            print(f"    SKIP: {e}")
            continue

        prompt = build_prompt(context)
        mode_results = []

        for run_idx in range(num_runs):
            print(f"    Run {run_idx + 1}/{num_runs}...", end=" ", flush=True)
            try:
                result = call_claude(prompt, model, client)
                result["run"] = run_idx + 1
                mode_results.append(result)
                print(
                    f"OK ({result['input_tokens']}+{result['output_tokens']} tokens, "
                    f"{result['latency_ms']:.0f}ms)"
                )
            except Exception as e:
                print(f"ERROR: {e}")
                mode_results.append({
                    "run": run_idx + 1,
                    "error": str(e),
                })

        results["modes"][mode] = {
            "context_file": f"exp_{experiment_id}_{mode}.json",
            "prompt_length": len(prompt),
            "runs": mode_results,
        }

        # Compute variance stats if we have successful runs
        successful = [r for r in mode_results if "response" in r]
        if successful:
            latencies = [r["latency_ms"] for r in successful]
            output_lens = [len(r["response"]) for r in successful]
            results["modes"][mode]["stats"] = {
                "successful_runs": len(successful),
                "mean_latency_ms": round(sum(latencies) / len(latencies), 1),
                "min_latency_ms": round(min(latencies), 1),
                "max_latency_ms": round(max(latencies), 1),
                "mean_response_length": round(sum(output_lens) / len(output_lens)),
                "min_response_length": min(output_lens),
                "max_response_length": max(output_lens),
            }

    # Save results
    os.makedirs(RESPONSES_DIR, exist_ok=True)
    output_file = os.path.join(
        RESPONSES_DIR,
        f"exp_{experiment_id}_{model.replace('/', '_')}_{datetime.now().strftime('%Y%m%d_%H%M%S')}.json"
    )
    with open(output_file, "w") as f:
        json.dump(results, f, indent=2)
        f.write("\n")

    print(f"\n  Results saved to: {output_file}")
    return results


def main():
    parser = argparse.ArgumentParser(
        description="LLM Evaluation Harness for Telos Validation"
    )
    parser.add_argument(
        "--experiment", "-e",
        required=True,
        help="Experiment ID (a-g) or 'all' to run all experiments",
    )
    parser.add_argument(
        "--model", "-m",
        default="claude-sonnet-4-6",
        help="Model to use (default: claude-sonnet-4-6)",
    )
    parser.add_argument(
        "--runs", "-r",
        type=int,
        default=3,
        help="Number of runs per mode (default: 3)",
    )
    args = parser.parse_args()

    # Validate experiment selection
    if args.experiment == "all":
        experiment_ids = list(EXPERIMENTS.keys())
    else:
        experiment_ids = [e.strip().lower() for e in args.experiment.split(",")]
        for eid in experiment_ids:
            if eid not in EXPERIMENTS:
                print(f"ERROR: Unknown experiment '{eid}'. Valid: {', '.join(EXPERIMENTS.keys())}")
                sys.exit(1)

    # Initialize Anthropic client
    client = anthropic.Anthropic()

    print("=" * 60)
    print("  Telos LLM Evaluation Harness")
    print(f"  Experiments: {', '.join(e.upper() for e in experiment_ids)}")
    print(f"  Model: {args.model}")
    print(f"  Runs per mode: {args.runs}")
    print("=" * 60)

    all_results = {}
    for eid in experiment_ids:
        result = run_experiment(eid, args.model, args.runs, client)
        all_results[eid] = result

    # Save combined results
    combined_file = os.path.join(
        RESPONSES_DIR,
        f"combined_{args.model.replace('/', '_')}_{datetime.now().strftime('%Y%m%d_%H%M%S')}.json"
    )
    with open(combined_file, "w") as f:
        json.dump(all_results, f, indent=2)
        f.write("\n")

    print(f"\n{'='*60}")
    print(f"  All results saved to: {combined_file}")
    print(f"{'='*60}")


if __name__ == "__main__":
    main()
