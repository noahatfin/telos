#!/usr/bin/env bash
# Generate intents at various scale points for benchmarking
#
# Creates separate .telos/ directories with 100, 500, 1000, 2000, 5000 intents
# each using realistic impact tags and constraint templates.
#
# Usage: ./generate_intents.sh [telos_bin]
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
TELOS_BIN="${1:-${TELOS_BIN:-telos}}"

export TELOS_AUTHOR_NAME="Benchmark Agent"
export TELOS_AUTHOR_EMAIL="bench@telos.dev"

# Pool of 50 impact areas
IMPACT_AREAS=(
    auth security payments users billing
    notifications emails search analytics dashboard
    api database cache sessions logging
    tasks boards projects comments attachments
    reports exports imports migrations backups
    webhooks integrations oauth saml sso
    profiles settings preferences themes localization
    inventory orders shipping returns refunds
    subscriptions plans quotas limits throttling
    audit compliance gdpr encryption certificates
)

# Pool of 200 constraint templates (50 shown, we'll cycle through patterns)
CONSTRAINT_TEMPLATES=(
    "Response time must be < {N}ms for {AREA} endpoints"
    "{AREA} data must be encrypted at rest"
    "{AREA} operations require authentication"
    "Max {N} {AREA} items per request"
    "{AREA} changes must be audited"
    "Rate limit: {N} {AREA} requests per minute"
    "{AREA} input must be validated before processing"
    "No PII in {AREA} error messages"
    "{AREA} must support pagination with max page size {N}"
    "{AREA} cache TTL must be <= {N} seconds"
    "Failed {AREA} operations must retry up to {N} times"
    "{AREA} writes must be idempotent"
    "{AREA} must support graceful degradation"
    "Concurrent {AREA} access must use optimistic locking"
    "{AREA} data retention: {N} days maximum"
    "{AREA} module must have > 80% test coverage"
    "{AREA} API must be backward compatible"
    "Batch {AREA} operations limited to {N} items"
    "{AREA} must emit structured log events"
    "{AREA} circuit breaker threshold: {N} failures"
    "{AREA} webhook delivery must guarantee at-least-once"
    "No hardcoded credentials in {AREA} config"
    "{AREA} must handle timezone-aware timestamps"
    "{AREA} must support multi-tenancy isolation"
    "{AREA} feature flags must default to disabled"
    "{AREA} must validate content-type headers"
    "{AREA} file uploads limited to {N}MB"
    "{AREA} session timeout: {N} minutes"
    "{AREA} must support CORS for allowed origins"
    "{AREA} database queries must use parameterized statements"
    "{AREA} must implement request deduplication"
    "{AREA} must support bulk operations via CSV import"
    "Max {N} concurrent {AREA} connections"
    "{AREA} must implement health check endpoint"
    "{AREA} password hashing must use bcrypt with cost {N}"
    "{AREA} tokens must expire within {N} hours"
    "{AREA} must support webhook signature verification"
    "{AREA} must log all admin operations"
    "{AREA} must enforce RBAC at API layer"
    "{AREA} must support soft delete"
    "{AREA} must validate email format"
    "{AREA} search results limited to {N} items"
    "{AREA} must compress responses > {N}KB"
    "{AREA} must support ETags for caching"
    "{AREA} must sanitize HTML input"
    "{AREA} must enforce unique constraints"
    "{AREA} must handle UTF-8 encoding"
    "{AREA} background jobs must timeout after {N} seconds"
    "{AREA} must implement dead letter queue"
    "{AREA} must support rolling deployments"
)

STATEMENT_TEMPLATES=(
    "Implement {AREA} module with production constraints"
    "Design {AREA} service boundary and validation rules"
    "Add {AREA} rate limiting and access controls"
    "Define {AREA} data model and integrity constraints"
    "Establish {AREA} monitoring and alerting thresholds"
    "Set up {AREA} caching strategy and invalidation"
    "Create {AREA} error handling and recovery flows"
    "Build {AREA} integration with external systems"
    "Define {AREA} SLA and performance targets"
    "Implement {AREA} audit trail and compliance checks"
)

BEHAVIOR_TEMPLATES=(
    "GIVEN a {AREA} request|WHEN input is invalid|THEN return 400 with validation details"
    "GIVEN an unauthorized user|WHEN {AREA} access is attempted|THEN return 403 Forbidden"
    "GIVEN a valid {AREA} operation|WHEN processing succeeds|THEN emit audit event"
    "GIVEN high load|WHEN {AREA} rate limit exceeded|THEN return 429 with retry-after"
    "GIVEN a {AREA} timeout|WHEN circuit breaker trips|THEN return 503 with fallback"
)

SCALE_POINTS=(100 500 1000 2000 5000)

generate_intent() {
    local dir="$1"
    local idx="$2"
    local total="$3"

    # Pick impact areas (1-3 per intent)
    local num_impacts=$(( (idx % 3) + 1 ))
    local impact_args=""
    for i in $(seq 0 $((num_impacts - 1))); do
        local area_idx=$(( (idx * 7 + i * 13) % ${#IMPACT_AREAS[@]} ))
        impact_args="$impact_args --impact ${IMPACT_AREAS[$area_idx]}"
    done

    # Pick primary area for templates
    local primary_area_idx=$(( idx % ${#IMPACT_AREAS[@]} ))
    local area="${IMPACT_AREAS[$primary_area_idx]}"

    # Pick statement
    local stmt_idx=$(( idx % ${#STATEMENT_TEMPLATES[@]} ))
    local statement="${STATEMENT_TEMPLATES[$stmt_idx]}"
    statement="${statement//\{AREA\}/$area}"

    # Pick constraints (1-3 per intent)
    local num_constraints=$(( (idx % 3) + 1 ))
    local constraint_args=""
    for i in $(seq 0 $((num_constraints - 1))); do
        local c_idx=$(( (idx * 3 + i * 11) % ${#CONSTRAINT_TEMPLATES[@]} ))
        local constraint="${CONSTRAINT_TEMPLATES[$c_idx]}"
        constraint="${constraint//\{AREA\}/$area}"
        local n_val=$(( (idx * 17 + i * 23) % 900 + 100 ))
        constraint="${constraint//\{N\}/$n_val}"
        constraint_args="$constraint_args --constraint \"$constraint\""
    done

    # Pick behavior (1 per intent)
    local b_idx=$(( idx % ${#BEHAVIOR_TEMPLATES[@]} ))
    local behavior="${BEHAVIOR_TEMPLATES[$b_idx]}"
    behavior="${behavior//\{AREA\}/$area}"

    # Execute telos intent command
    eval "$TELOS_BIN intent \
        --statement \"$statement\" \
        $constraint_args \
        $impact_args \
        --behavior \"$behavior\"" 2>/dev/null
}

for scale in "${SCALE_POINTS[@]}"; do
    echo "=== Generating $scale intents ==="

    BENCH_DIR="$SCRIPT_DIR/bench_${scale}"
    rm -rf "$BENCH_DIR"
    mkdir -p "$BENCH_DIR"

    # Initialize git repo (telos requires it)
    cd "$BENCH_DIR"
    git init -q
    git commit --allow-empty -m "init" -q

    # Initialize telos
    $TELOS_BIN init 2>/dev/null || true

    for i in $(seq 1 "$scale"); do
        generate_intent "$BENCH_DIR" "$i" "$scale"

        # Progress reporting
        if (( i % 100 == 0 )); then
            echo "  Generated $i / $scale intents"
        fi
    done

    echo "  Done: $scale intents in $BENCH_DIR"
    echo "  Telos store size: $(du -sh "$BENCH_DIR/.telos" 2>/dev/null | cut -f1)"
    echo ""
done

echo "=== Intent generation complete ==="
echo "Scale point directories:"
for scale in "${SCALE_POINTS[@]}"; do
    echo "  bench_${scale}: $scale intents"
done
