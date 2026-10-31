#!/bin/bash
# Env parity preflight (Phase 21 Decision 14). Run on the server from
# /opt/zainstreat, after _deploy.yml uploads .env.next next to the live
# .env. Resolves both through the (new) compose files and prints the
# service.KEY names whose resolved values differ — hashes are compared,
# values are never printed.
#
# Usage: env-parity.sh <enforce:true|false> [allowed,KEYS,...]
# Exits 1 when enforce=true and a differing key isn't in the allow list.
set -euo pipefail
cd /opt/zainstreat

ENFORCE="${1:-true}"
ALLOWED=",${2:-},"

if [ ! -f .env ]; then
  echo "No live .env yet (fresh box): skipping parity check."
  exit 0
fi

# python3 ships with Amazon Linux 2023 (dnf depends on it); jq doesn't.
hashes() {
  # The live .env predates API_IMAGE/WEB_IMAGE (required by
  # docker-compose.prod.yml); shell env wins over --env-file, so pin both.
  API_IMAGE=parity WEB_IMAGE=parity docker compose --env-file "$1" -f docker-compose.yml -f docker-compose.prod.yml config --format json \
    | python3 -c '
import hashlib, json, sys
for name, svc in json.load(sys.stdin)["services"].items():
    for key, value in (svc.get("environment") or {}).items():
        digest = hashlib.sha256(str(value).encode()).hexdigest()[:16]
        print(f"{name}.{key} {digest}")
' | sort
}

# Image refs change on every deploy by design.
CHANGED=$(diff <(hashes .env) <(hashes .env.next) \
  | awk '/^[<>]/ {print $2}' | sort -u \
  | grep -vE '\.(API_IMAGE|WEB_IMAGE)$' || true)

if [ -z "$CHANGED" ]; then
  echo "Env parity: no changes."
  exit 0
fi

echo "Env parity: resolved values differ for:"
BLOCKED=0
for k in $CHANGED; do
  key="${k#*.}"
  if [[ "$ALLOWED" == *",$key,"* ]]; then
    echo "  $k (allowed)"
  else
    echo "  $k"
    BLOCKED=1
  fi
done

if [ "$ENFORCE" = "true" ] && [ "$BLOCKED" = "1" ]; then
  echo "::error::Env parity check failed. Fix the Environment secret/variable, or list the key in allow_env_changes if the change is intended." >&2
  exit 1
fi
