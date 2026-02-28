#!/usr/bin/env python3
"""
Scale & Latency Benchmark for Telos Query Performance

Benchmarks `telos query intents --impact <area>` and
`telos context --impact <area> --json` at various scale points.

Usage:
    python bench_queries.py [telos_bin]

Pass criteria:
    - <500ms  at 1000 intents
    - <2000ms at 5000 intents

Output: validation/scale/bench_results.json
"""
import json
import os
import subprocess
import statistics
import sys
import time

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
SCALE_POINTS = [100, 500, 1000, 2000, 5000]
NUM_RUNS = 10

# Impact areas to query (subset for benchmarking)
QUERY_AREAS = ["auth", "security", "payments", "tasks", "billing"]

PASS_CRITERIA = {
    1000: 500,   # <500ms at 1000 intents
    5000: 2000,  # <2000ms at 5000 intents
}


def run_timed_command(cmd, cwd):
    """Run a command and return (stdout, elapsed_ms)."""
    start = time.monotonic()
    result = subprocess.run(
        cmd, shell=True, capture_output=True, text=True, cwd=cwd
    )
    elapsed_ms = (time.monotonic() - start) * 1000
    return result.stdout.strip(), result.returncode, elapsed_ms


def compute_stats(timings):
    """Compute mean, p50, p95 from a list of timings."""
    if not timings:
        return {"mean": 0, "p50": 0, "p95": 0, "min": 0, "max": 0}
    sorted_t = sorted(timings)
    n = len(sorted_t)
    p50_idx = n // 2
    p95_idx = min(int(n * 0.95), n - 1)
    return {
        "mean": round(statistics.mean(sorted_t), 2),
        "p50": round(sorted_t[p50_idx], 2),
        "p95": round(sorted_t[p95_idx], 2),
        "min": round(min(sorted_t), 2),
        "max": round(max(sorted_t), 2),
    }


def benchmark_scale_point(scale, telos_bin):
    """Benchmark all query types at a given scale point."""
    bench_dir = os.path.join(SCRIPT_DIR, f"bench_{scale}")
    if not os.path.isdir(bench_dir):
        print(f"  SKIP: Directory {bench_dir} not found (run generate_intents.sh first)")
        return None

    results = {
        "intent_count": scale,
        "queries": {},
    }

    for area in QUERY_AREAS:
        # Benchmark: telos query intents --impact <area>
        query_cmd = f"{telos_bin} query intents --impact {area} --json"
        query_timings = []
        for run in range(NUM_RUNS):
            _, rc, elapsed = run_timed_command(query_cmd, bench_dir)
            if rc == 0:
                query_timings.append(elapsed)

        query_key = f"query_intents_{area}"
        results["queries"][query_key] = {
            "command": query_cmd,
            "runs": NUM_RUNS,
            "successful_runs": len(query_timings),
            "timings_ms": [round(t, 2) for t in query_timings],
            "stats": compute_stats(query_timings),
        }

        # Benchmark: telos context --impact <area> --json
        context_cmd = f"{telos_bin} context --impact {area} --json"
        context_timings = []
        for run in range(NUM_RUNS):
            _, rc, elapsed = run_timed_command(context_cmd, bench_dir)
            if rc == 0:
                context_timings.append(elapsed)

        context_key = f"context_{area}"
        results["queries"][context_key] = {
            "command": context_cmd,
            "runs": NUM_RUNS,
            "successful_runs": len(context_timings),
            "timings_ms": [round(t, 2) for t in context_timings],
            "stats": compute_stats(context_timings),
        }

    # Compute aggregate stats across all queries
    all_timings = []
    for q in results["queries"].values():
        all_timings.extend(q["timings_ms"])

    results["aggregate_stats"] = compute_stats(all_timings)

    # Check pass criteria
    if scale in PASS_CRITERIA:
        threshold = PASS_CRITERIA[scale]
        p95 = results["aggregate_stats"]["p95"]
        results["pass_criteria"] = {
            "threshold_ms": threshold,
            "p95_ms": p95,
            "pass": p95 < threshold,
        }

    return results


def main():
    telos_bin = sys.argv[1] if len(sys.argv) > 1 else os.environ.get("TELOS_BIN", "telos")

    print("=" * 60)
    print("  Telos Scale & Latency Benchmark")
    print(f"  Binary: {telos_bin}")
    print(f"  Runs per query: {NUM_RUNS}")
    print(f"  Query areas: {', '.join(QUERY_AREAS)}")
    print("=" * 60)

    all_results = {
        "telos_bin": telos_bin,
        "num_runs": NUM_RUNS,
        "query_areas": QUERY_AREAS,
        "pass_criteria": PASS_CRITERIA,
        "scale_points": {},
    }

    for scale in SCALE_POINTS:
        print(f"\n--- Scale point: {scale} intents ---")
        result = benchmark_scale_point(scale, telos_bin)
        if result:
            all_results["scale_points"][str(scale)] = result

            # Print summary
            stats = result["aggregate_stats"]
            print(f"  Aggregate: mean={stats['mean']:.1f}ms  p50={stats['p50']:.1f}ms  p95={stats['p95']:.1f}ms")
            if "pass_criteria" in result:
                pc = result["pass_criteria"]
                status = "PASS" if pc["pass"] else "FAIL"
                print(f"  Criteria: p95 < {pc['threshold_ms']}ms -> {status} (p95={pc['p95_ms']:.1f}ms)")

    # Save results
    output_file = os.path.join(SCRIPT_DIR, "bench_results.json")
    with open(output_file, "w") as f:
        json.dump(all_results, f, indent=2)
        f.write("\n")

    print(f"\n{'='*60}")
    print(f"  Results saved to: {output_file}")

    # Final pass/fail summary
    print()
    any_fail = False
    for scale_str, result in all_results["scale_points"].items():
        if "pass_criteria" in result:
            pc = result["pass_criteria"]
            status = "PASS" if pc["pass"] else "FAIL"
            if not pc["pass"]:
                any_fail = True
            print(f"  {scale_str} intents: {status} (p95={pc['p95_ms']:.1f}ms, threshold={pc['threshold_ms']}ms)")

    print(f"{'='*60}")
    sys.exit(1 if any_fail else 0)


if __name__ == "__main__":
    main()
