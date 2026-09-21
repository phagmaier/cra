#!/usr/bin/env bash
# M2-06: explicit bounded Rust diagnostics, never part of the default suite.
# Existing files are immutable. An interrupted/failed bundle cannot be reused.
set -euo pipefail

if [[ $# -ne 1 ]]; then
    printf 'Usage: bash scripts/run_score_diagnostics.sh NEW_OUTPUT_DIRECTORY\n' >&2
    exit 2
fi
# Interpret relative output paths from the caller's directory, then run all
# commands at the repository root (the Rust provenance helpers expect this).
mkdir -- "$1"
output_dir=$(cd -- "$1" && pwd)
repo_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cd -- "$repo_dir"
trap 'code=$?; printf "%s\n" "$code" > "$output_dir/exit_code"' EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

# Prevent a caller's optional per-test output setting from overwriting or
# reserving an unrelated path. Each export below belongs to this new bundle.
unset CRA_M2_DIRECTION_EVIDENCE CRA_M2_RECURRENT_EVIDENCE CRA_M2_ROLLOUT_EVIDENCE

git rev-parse HEAD > "$output_dir/revision.txt"
git status --porcelain > "$output_dir/git-status.txt"
rustc --version > "$output_dir/rustc.txt"
cargo --version > "$output_dir/cargo.txt"
uname -srm > "$output_dir/platform.txt"
date -u '+%Y-%m-%dT%H:%M:%SZ' > "$output_dir/started-utc.txt"
sha256sum \
    Cargo.toml Cargo.lock rust-toolchain.toml spec.md \
    scripts/run_score_diagnostics.sh \
    src/agent/{actor,score,weights,topology}.rs src/{config,rng}.rs \
    src/experiments/finite_rollout.rs \
    tests/{score,score_log_probability,score_learning_direction,finite_rollout,score_recurrent}.rs \
    docs/evidence/m2-{03,05}/plan.md > "$output_dir/source.sha256"
cp docs/evidence/m2-03/plan.md "$output_dir/direction-plan.md"
cp docs/evidence/m2-05/plan.md "$output_dir/recurrent-plan.md"
# Deterministic fixture parameters/analytic expressions are embedded here;
# preserve their exact source alongside the observed derivative test output.
cp tests/score.rs "$output_dir/score-fixture.rs"
cp tests/score_log_probability.rs "$output_dir/derivative-fixture.rs"

run_check() {
    local name=$1
    shift
    printf 'Running %s\n' "$name"
    printf '%q ' "$@" >> "$output_dir/commands.txt"
    printf '\n' >> "$output_dir/commands.txt"
    local code=0
    "$@" > "$output_dir/$name.log" 2>&1 || code=$?
    printf '%s\t%s\n' "$name" "$code" >> "$output_dir/checks.tsv"
    if [[ $code -ne 0 ]]; then
        cat "$output_dir/$name.log" >&2
        exit "$code"
    fi
}

run_check fast env CRA_M2_ROLLOUT_EVIDENCE="$output_dir/rollout-golden.json" \
    cargo test --locked --test score --test score_log_probability \
    --test score_learning_direction --test finite_rollout --test score_recurrent \
    -- --nocapture
run_check api cargo test --locked --doc
run_check direction env CRA_M2_DIRECTION_EVIDENCE="$output_dir/direction.json" \
    cargo test --release --locked --test score_learning_direction \
    one_neuron_learning_direction -- --ignored --exact --nocapture
run_check recurrent env CRA_M2_RECURRENT_EVIDENCE="$output_dir/recurrent.json" \
    cargo test --release --locked --test score_recurrent \
    two_neuron_recurrent_finite_difference -- --ignored --exact --nocapture

# A typo in an exact Cargo test filter can exit successfully with zero tests.
# Require the actual successful test names and output artifacts, too.
for pair in 'direction one_neuron_learning_direction' 'recurrent two_neuron_recurrent_finite_difference'; do
    read -r name test_name <<< "$pair"
    if ! grep -Fxq "test $test_name ... ok" "$output_dir/$name.log"; then
        printf 'Required test missing: %s\n' "$test_name" >&2
        exit 1
    fi
done
for artifact in rollout-golden.json direction.json recurrent.json; do
    test -s "$output_dir/$artifact"
done
# Source changes while compiling/running make the combined evidence ambiguous.
sha256sum --check "$output_dir/source.sha256" > "$output_dir/source-check.log"
date -u '+%Y-%m-%dT%H:%M:%SZ' > "$output_dir/finished-utc.txt"
printf 'All M2 diagnostics passed. Restricted fixed-weight score checks only; no online unbiasedness or convergence claim.\n' > "$output_dir/PASSED"
printf 'Evidence: %s\n' "$output_dir"
