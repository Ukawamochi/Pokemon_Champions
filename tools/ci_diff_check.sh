#!/usr/bin/env bash
set -euo pipefail

FAIL_ON_DIFF=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --fail-on-diff)
      FAIL_ON_DIFF=1
      shift
      ;;
    --help|-h)
      echo "Usage: tools/ci_diff_check.sh [--fail-on-diff]"
      exit 0
      ;;
    *)
      echo "Unknown arg: $1" >&2
      exit 2
      ;;
  esac
done

mkdir -p tmp reports

EXIT_CODE=0

shopt -s nullglob
for CASE in tests/showdown_compat/cases/*.json; do
  CASE_NAME="$(basename "$CASE" .json)"
  RUST_JSON="tmp/rust_${CASE_NAME}.json"
  REPORT_HTML="reports/${CASE_NAME}_report.html"

  cargo run --release --bin pokemon-battle-cli -- \
    run-case --case "$CASE" --log-json "$RUST_JSON"

  if [[ "$FAIL_ON_DIFF" -eq 1 ]]; then
    if ! cargo run --release --bin diff_analyzer -- \
      --showdown "$CASE" --rust "$RUST_JSON" --out "$REPORT_HTML" --fail-on-diff; then
      EXIT_CODE=1
    fi
  else
    cargo run --release --bin diff_analyzer -- \
      --showdown "$CASE" --rust "$RUST_JSON" --out "$REPORT_HTML" || true
  fi
done

exit "$EXIT_CODE"

