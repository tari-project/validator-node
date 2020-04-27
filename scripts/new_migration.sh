#!/bin/bash

set -e
if [[ -z "$1" ]]; then
  echo "Usage: $0 migration_name"
  exit 1
fi
NAME="$1"
CURRENT_DIR="$(cd "$(dirname "$0")" && pwd)"
MIGRATIONS_DIR="$(cd "${CURRENT_DIR}" && cd ../node/migrations && pwd)"
DATE=$(date +%s)
FILENAME="${MIGRATIONS_DIR}/V${DATE}__${NAME}.sql"
touch "$FILENAME" && echo "Created migration file ${FILENAME}"