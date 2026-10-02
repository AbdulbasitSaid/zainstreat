#!/usr/bin/env bash
# PostToolUse hook (Edit|Write): lints a single just-edited apps/web TS/TSX
# file and surfaces ESLint's output back to Claude on failure.
set -euo pipefail

payload="$(cat)"
file_path="$(printf '%s' "$payload" | jq -r '.tool_input.file_path // empty')"

if [[ -z "$file_path" ]]; then
  exit 0
fi

case "$file_path" in
  *apps/web/*.ts | *apps/web/*.tsx)
    case "$file_path" in
      *node_modules/* | */.next/*) exit 0 ;;
    esac
    ;;
  *)
    exit 0
    ;;
esac

web_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)/apps/web"
cd "$web_dir"

if ! output="$(pnpm exec eslint "$file_path" 2>&1)"; then
  echo "$output" >&2
  exit 2
fi

exit 0
