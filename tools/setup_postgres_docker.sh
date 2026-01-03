#!/usr/bin/env bash
# Run: ./tools/setup_postgres_docker.sh
# Reset DB: ./tools/setup_postgres_docker.sh --reset
# Write env file: ./tools/setup_postgres_docker.sh --write-env .env.local
# Export into current shell: source ./tools/setup_postgres_docker.sh
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  ./tools/setup_postgres_docker.sh [--reset] [--write-env FILE]

Creates (or starts) a local Postgres container for SPAI development.

Environment overrides:
  CONTAINER_NAME   (default: spai-postgres)
  POSTGRES_IMAGE   (default: postgres:16)
  POSTGRES_USER    (default: spai)
  POSTGRES_PASSWORD(default: spai)
  POSTGRES_DB      (default: spai)
  POSTGRES_PORT    (default: 5432)
  POSTGRES_VOLUME  (default: spai-postgres-data)

Notes:
  - To export DATABASE_URL into your current shell, source this script:
      source ./tools/setup_postgres_docker.sh
EOF
}

RESET=0
WRITE_ENV_FILE=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    -h|--help) usage; exit 0 ;;
    --reset) RESET=1; shift ;;
    --write-env)
      WRITE_ENV_FILE="${2:-}"
      if [[ -z "${WRITE_ENV_FILE}" ]]; then
        echo "error: --write-env requires a file path" >&2
        exit 2
      fi
      shift 2
      ;;
    *)
      echo "error: unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if ! command -v docker >/dev/null 2>&1; then
  echo "error: docker is not installed (or not on PATH)" >&2
  exit 1
fi

if ! docker info >/dev/null 2>&1; then
  echo "error: docker daemon not reachable (is Docker running?)" >&2
  exit 1
fi

CONTAINER_NAME="${CONTAINER_NAME:-spai-postgres}"
POSTGRES_IMAGE="${POSTGRES_IMAGE:-postgres:16}"
POSTGRES_USER="${POSTGRES_USER:-spai}"
POSTGRES_PASSWORD="${POSTGRES_PASSWORD:-spai}"
POSTGRES_DB="${POSTGRES_DB:-spai}"
POSTGRES_PORT="${POSTGRES_PORT:-5432}"
POSTGRES_VOLUME="${POSTGRES_VOLUME:-spai-postgres-data}"

if [[ "${RESET}" -eq 1 ]]; then
  if docker ps -a --format '{{.Names}}' | grep -qx "${CONTAINER_NAME}"; then
    docker rm -f "${CONTAINER_NAME}" >/dev/null
  fi
  if docker volume ls --format '{{.Name}}' | grep -qx "${POSTGRES_VOLUME}"; then
    docker volume rm "${POSTGRES_VOLUME}" >/dev/null
  fi
fi

if docker ps -a --format '{{.Names}}' | grep -qx "${CONTAINER_NAME}"; then
  if docker ps --format '{{.Names}}' | grep -qx "${CONTAINER_NAME}"; then
    echo "Postgres container already running: ${CONTAINER_NAME}"
  else
    echo "Starting existing Postgres container: ${CONTAINER_NAME}"
    docker start "${CONTAINER_NAME}" >/dev/null
  fi
else
  echo "Creating Postgres container: ${CONTAINER_NAME}"
  docker run \
    --name "${CONTAINER_NAME}" \
    -e "POSTGRES_USER=${POSTGRES_USER}" \
    -e "POSTGRES_PASSWORD=${POSTGRES_PASSWORD}" \
    -e "POSTGRES_DB=${POSTGRES_DB}" \
    -p "${POSTGRES_PORT}:5432" \
    -v "${POSTGRES_VOLUME}:/var/lib/postgresql/data" \
    -d "${POSTGRES_IMAGE}" >/dev/null
fi

echo "Waiting for Postgres to become ready..."
for _ in $(seq 1 60); do
  if docker exec "${CONTAINER_NAME}" pg_isready -U "${POSTGRES_USER}" -d "${POSTGRES_DB}" >/dev/null 2>&1; then
    break
  fi
  sleep 0.5
done

if ! docker exec "${CONTAINER_NAME}" pg_isready -U "${POSTGRES_USER}" -d "${POSTGRES_DB}" >/dev/null 2>&1; then
  echo "error: Postgres did not become ready; check logs with: docker logs ${CONTAINER_NAME}" >&2
  exit 1
fi

DATABASE_URL="postgres://${POSTGRES_USER}:${POSTGRES_PASSWORD}@localhost:${POSTGRES_PORT}/${POSTGRES_DB}"

if [[ -n "${WRITE_ENV_FILE}" ]]; then
  cat >"${WRITE_ENV_FILE}" <<EOF
DATABASE_URL=${DATABASE_URL}
SPAI_DATABASE_URL=${DATABASE_URL}
EOF
  echo "Wrote env file: ${WRITE_ENV_FILE}"
fi

if [[ "${BASH_SOURCE[0]}" != "${0}" ]]; then
  export DATABASE_URL
  export SPAI_DATABASE_URL="${DATABASE_URL}"
  echo "Exported DATABASE_URL and SPAI_DATABASE_URL into current shell."
else
  echo
  echo "Set env vars:"
  echo "  export DATABASE_URL='${DATABASE_URL}'"
  echo "  export SPAI_DATABASE_URL='${DATABASE_URL}'"
fi
