#!/bin/bash
# Reads or writes one uploaded image in this server's MinIO bucket, for
# copy-staging-event.yml (Phase 34 Decision 8). The workflow copies it to
# the server and runs it over SSH; it isn't installed. The MinIO image has
# no client, so this runs a pinned curl container on the Compose network
# and signs the request with SigV4.
#
#   minio-object.sh get <key> > file
#   minio-object.sh put <key> < file
set -euo pipefail
cd /opt/zainstreat
C="docker compose -f docker-compose.yml -f docker-compose.prod.yml"
CURL_IMAGE=curlimages/curl:8.11.1

action=${1:-}
key=${2:-}
# The API names uploads <uuid>.<ext> (routes/admin/media.rs).
[[ $key =~ ^[0-9a-f-]{36}\.(jpg|png|webp)$ ]] || { echo "refused: not an upload key: $key" >&2; exit 1; }
case "${key##*.}" in
  jpg)  type=image/jpeg ;;
  png)  type=image/png ;;
  webp) type=image/webp ;;
esac

minio=$($C ps -q minio)
test -n "$minio" || { echo "MinIO isn't running." >&2; exit 1; }
network=$(docker inspect -f '{{range $name, $_ := .NetworkSettings.Networks}}{{$name}} {{end}}' "$minio" | awk '{print $1}')
# `exec` forwards stdin, so these read /dev/null and leave the upload's body alone.
bucket=$($C exec -T api printenv MINIO_BUCKET < /dev/null 2>/dev/null || true)
export URL="http://minio:9000/${bucket:-menu-images}/$key" TYPE="$type"
MINIO_USER=$($C exec -T minio printenv MINIO_ROOT_USER < /dev/null)
MINIO_PASS=$($C exec -T minio printenv MINIO_ROOT_PASSWORD < /dev/null)
export MINIO_USER MINIO_PASS

run() {
  docker run --rm -i --network "$network" -e URL -e TYPE -e MINIO_USER -e MINIO_PASS \
    --entrypoint sh "$CURL_IMAGE" -c "$1"
}

case "$action" in
  get) run 'curl -fsS --aws-sigv4 "aws:amz:us-east-1:s3" --user "$MINIO_USER:$MINIO_PASS" "$URL"' ;;
  # MinIO wants a Content-Length, so the body is buffered before upload.
  put) run 'cat > /tmp/object && curl -fsS --aws-sigv4 "aws:amz:us-east-1:s3" --user "$MINIO_USER:$MINIO_PASS" -H "Content-Type: $TYPE" -T /tmp/object "$URL"' ;;
  *)   echo "usage: minio-object.sh get|put <key>" >&2; exit 1 ;;
esac
