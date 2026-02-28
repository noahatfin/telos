# Telos Integration Guide

Step-by-step guide for adding Telos to existing projects.

## Quick Start

### 1. Install Telos

```bash
# Build from source
git clone <telos-repo>
cd telos
cargo build --release
export PATH="$PWD/target/release:$PATH"
```

### 2. Initialize in Your Project

```bash
cd your-project
telos init
```

This creates a `.telos/` directory in your project root with the content-addressable object store.

### 3. Record Your First Intent

```bash
telos intent \
  --statement "Users can authenticate via email/password" \
  --constraint "Passwords must be >= 12 characters" \
  --constraint "Failed login attempts locked after 5 tries" \
  --impact "auth" \
  --impact "security" \
  --behavior "GIVEN valid credentials|WHEN login is attempted|THEN return session token"
```

### 4. Query Before Coding

```bash
# What constraints apply to the auth module?
telos query intents --impact auth

# Get full context as JSON (for feeding to AI agents)
telos context --impact auth --json
```

## Git Hook Integration

### Pre-Commit Hook: Remind About Constraints

Create `.git/hooks/pre-commit`:

```bash
#!/usr/bin/env bash
# Remind developer of relevant constraints before committing

# Get list of changed files
CHANGED_FILES=$(git diff --cached --name-only)

# Detect impacted areas from file paths
IMPACTS=""
if echo "$CHANGED_FILES" | grep -q "auth\|login\|session"; then
    IMPACTS="$IMPACTS auth"
fi
if echo "$CHANGED_FILES" | grep -q "payment\|billing\|charge"; then
    IMPACTS="$IMPACTS payments"
fi
if echo "$CHANGED_FILES" | grep -q "task\|board\|project"; then
    IMPACTS="$IMPACTS tasks"
fi

# Show relevant constraints
for impact in $IMPACTS; do
    echo "--- Telos constraints for '$impact' ---"
    telos query intents --impact "$impact" 2>/dev/null || true
    echo ""
done
```

### Pre-Push Hook: Validate Constraints

Create `.git/hooks/pre-push`:

```bash
#!/usr/bin/env bash
# Check that recent commits reference known impact areas

REMOTE="$1"
URL="$2"

while read local_ref local_sha remote_ref remote_sha; do
    # Get commits being pushed
    if [ "$remote_sha" = "0000000000000000000000000000000000000000" ]; then
        COMMITS=$(git log --oneline "$local_sha" --not --remotes)
    else
        COMMITS=$(git log --oneline "$remote_sha..$local_sha")
    fi

    echo "Checking $(echo "$COMMITS" | wc -l) commits against Telos intents..."

    # Optional: Generate context JSON for AI review
    telos context --json > /tmp/telos-push-context.json 2>/dev/null
done

exit 0
```

### Commit-Msg Hook: Link Commits to Intents

Create `.git/hooks/commit-msg`:

```bash
#!/usr/bin/env bash
# Append relevant intent IDs to commit messages

COMMIT_MSG_FILE="$1"
MSG=$(cat "$COMMIT_MSG_FILE")

# Skip if already has intent references
if echo "$MSG" | grep -q "Intent:"; then
    exit 0
fi

# Detect impacts from staged changes
CHANGED_FILES=$(git diff --cached --name-only)
IMPACTS=""
for f in $CHANGED_FILES; do
    case "$f" in
        *auth*|*login*) IMPACTS="$IMPACTS auth" ;;
        *task*|*board*) IMPACTS="$IMPACTS tasks" ;;
        *payment*)      IMPACTS="$IMPACTS payments" ;;
    esac
done

# Append intent context as trailer
if [ -n "$IMPACTS" ]; then
    UNIQUE_IMPACTS=$(echo "$IMPACTS" | tr ' ' '\n' | sort -u | tr '\n' ',')
    echo "" >> "$COMMIT_MSG_FILE"
    echo "Telos-Impact: $UNIQUE_IMPACTS" >> "$COMMIT_MSG_FILE"
fi
```

Make hooks executable:

```bash
chmod +x .git/hooks/pre-commit .git/hooks/pre-push .git/hooks/commit-msg
```

## CI/CD Integration

### GitHub Actions: Constraint Check

```yaml
# .github/workflows/telos-check.yml
name: Telos Constraint Check

on:
  pull_request:
    branches: [main]

jobs:
  constraint-check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0  # Full history for diff

      - name: Install Telos
        run: |
          cargo install --path .
          # Or download pre-built binary

      - name: Generate PR context
        run: |
          # Get changed files in this PR
          CHANGED=$(git diff --name-only origin/main...HEAD)

          # Detect impacted areas
          IMPACTS=""
          echo "$CHANGED" | grep -q "auth" && IMPACTS="$IMPACTS auth"
          echo "$CHANGED" | grep -q "task" && IMPACTS="$IMPACTS tasks"

          # Generate context for review
          for impact in $IMPACTS; do
            echo "=== Constraints for: $impact ==="
            telos query intents --impact "$impact"
          done

      - name: Generate context JSON for AI review
        run: |
          telos context --json > telos-context.json
          echo "::set-output name=context::$(cat telos-context.json)"
```

### GitLab CI: Intent Drift Detection

```yaml
# .gitlab-ci.yml
telos-check:
  stage: test
  script:
    - telos query intents --json > current-intents.json
    - |
      # Compare intent count to last recorded baseline
      INTENT_COUNT=$(cat current-intents.json | python3 -c "import sys,json; print(len(json.load(sys.stdin)))")
      echo "Total intents: $INTENT_COUNT"

      # Warn if no intents exist for changed modules
      CHANGED_MODULES=$(git diff --name-only $CI_MERGE_REQUEST_DIFF_BASE_SHA | cut -d/ -f2 | sort -u)
      for module in $CHANGED_MODULES; do
        COUNT=$(telos query intents --impact "$module" --json 2>/dev/null | python3 -c "import sys,json; print(len(json.load(sys.stdin)))" 2>/dev/null || echo "0")
        if [ "$COUNT" = "0" ]; then
          echo "WARNING: No Telos intents found for module '$module'"
        fi
      done
```

### Integrating with AI Code Review

Generate context JSON and feed it to an LLM-based reviewer:

```bash
#!/usr/bin/env bash
# generate-review-context.sh
# Called in CI to produce context for AI code review

# Get the diff
DIFF=$(git diff origin/main...HEAD)

# Get all relevant intents
CONTEXT=$(telos context --json)

# Combine into a review prompt
cat <<EOF > review-input.json
{
  "task": "Review this PR against the project's recorded constraints and intents. Flag any violations.",
  "git_diff": $(echo "$DIFF" | python3 -c "import sys,json; print(json.dumps(sys.stdin.read()))"),
  "telos_context": $CONTEXT
}
EOF

echo "Review context saved to review-input.json"
```

## Project Structure

After setup, your project will look like:

```
your-project/
  .git/
    hooks/
      pre-commit    # Optional: show constraints
      pre-push      # Optional: validate
      commit-msg    # Optional: link intents
  .telos/
    config          # Telos configuration
    objects/        # Content-addressable store
      ab/
        ab1234...   # Intent objects
      cd/
        cd5678...
  .gitignore        # Should NOT ignore .telos/
  src/
    ...
```

The `.telos/` directory should be committed to git so the entire team shares the same intent history.

## Tips

- Record intents at the start of a feature, not after implementation
- Use impact tags consistently across the team (agree on a taxonomy)
- Constraints should be verifiable: "Token expiry <= 1 hour" is better than "tokens should expire reasonably"
- Behavior specs use Given/When/Then format for clarity
- Review `.telos/` changes in PRs just like code changes
