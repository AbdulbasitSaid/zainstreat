#!/bin/bash
# Forced command for the backup SSH key (Phase 21 Decision 15). Installed
# as /opt/zainstreat/bin/backup-stream and pinned in the deploy user's
# authorized_keys with `restrict,command="..."`, so this is the only thing
# that key can run. Streams one backup to stdout; refuses everything else.
set -euo pipefail
cd /opt/zainstreat
C="docker compose -f docker-compose.yml -f docker-compose.prod.yml"

case "${SSH_ORIGINAL_COMMAND:-}" in
  # The postgres container already has POSTGRES_USER/DB in its env, so the
  # host never parses .env (a hand-written one isn't necessarily quoted).
  db)    exec $C exec -T postgres sh -c 'pg_dump -U "$POSTGRES_USER" -Fc "$POSTGRES_DB"' ;;
  minio) exec $C exec -T minio tar -C /data -czf - . ;;
  *)     echo "refused: only 'db' or 'minio'" >&2; exit 1 ;;
esac
