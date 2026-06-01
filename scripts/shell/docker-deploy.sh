#!/usr/bin/env sh
set -eu

FLOWBASE_REPO="${FLOWBASE_REPO:-taichuy/1flowbase}"
FLOWBASE_REF="${FLOWBASE_REF:-main}"
FLOWBASE_ARCHIVE_URL="${FLOWBASE_ARCHIVE_URL:-https://codeload.github.com/${FLOWBASE_REPO}/tar.gz/refs/heads/${FLOWBASE_REF}}"
FLOWBASE_ARCHIVE_DOCKER_DIR="1flowbase-${FLOWBASE_REF}/docker"
DB_PASSWORD="${FLOWBASE_DB_PASSWORD:-}"
ROOT_ACCOUNT="${FLOWBASE_ROOT_ACCOUNT:-}"
ROOT_PASSWORD="${FLOWBASE_ROOT_PASSWORD:-}"
PROVIDER_SECRET="${FLOWBASE_PROVIDER_SECRET:-}"
WEB_PORT="${FLOWBASE_WEB_PORT:-}"
START_CONTAINERS=1

if [ "${FLOWBASE_NO_START:-}" = "1" ] || [ "${FLOWBASE_NO_START:-}" = "true" ]; then
  START_CONTAINERS=0
fi

fail() {
  printf '%s\n' "$1" >&2
  exit 1
}

read_env_value() {
  key="$1"
  file="$2"
  if [ -f "$file" ]; then
    grep -E "^${key}=" "$file" | tail -n 1 | cut -d= -f2- || true
  fi
}

set_env_value() {
  key="$1"
  value="$2"
  file="$3"
  tmp="${file}.tmp.$$"
  awk -v key="$key" -v value="$value" '
    BEGIN { found = 0 }
    $0 ~ "^" key "=" {
      print key "=" value
      found = 1
      next
    }
    { print }
    END {
      if (!found) {
        print key "=" value
      }
    }
  ' "$file" > "$tmp" && mv "$tmp" "$file"
}

require_value() {
  option="$1"
  [ -n "${2-}" ] || fail "Missing value for ${option}."
  printf '%s\n' "$2"
}

usage() {
  cat <<'EOF'
Usage: docker-deploy.sh [options]

Options:
  --db-password VALUE       Set POSTGRES_PASSWORD in docker/.env.
  --root-account VALUE      Set BOOTSTRAP_ROOT_ACCOUNT in docker/.env.
  --root-password VALUE     Set BOOTSTRAP_ROOT_PASSWORD in docker/.env.
  --provider-secret VALUE   Set API_PROVIDER_SECRET_MASTER_KEY in docker/.env.
  --web-port VALUE          Set WEB_PORT in docker/.env.
  --no-start                Prepare docker files and docker/.env only. Do not pull images or start containers.
  -h, --help                Show this help.

Environment variables with the same effect:
  FLOWBASE_DB_PASSWORD
  FLOWBASE_ROOT_ACCOUNT
  FLOWBASE_ROOT_PASSWORD
  FLOWBASE_PROVIDER_SECRET
  FLOWBASE_WEB_PORT
  FLOWBASE_NO_START=1
EOF
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --db-password)
      DB_PASSWORD="$(require_value "$1" "${2-}")"
      shift 2
      ;;
    --db-password=*)
      DB_PASSWORD="${1#*=}"
      shift
      ;;
    --root-account)
      ROOT_ACCOUNT="$(require_value "$1" "${2-}")"
      shift 2
      ;;
    --root-account=*)
      ROOT_ACCOUNT="${1#*=}"
      shift
      ;;
    --root-password)
      ROOT_PASSWORD="$(require_value "$1" "${2-}")"
      shift 2
      ;;
    --root-password=*)
      ROOT_PASSWORD="${1#*=}"
      shift
      ;;
    --provider-secret)
      PROVIDER_SECRET="$(require_value "$1" "${2-}")"
      shift 2
      ;;
    --provider-secret=*)
      PROVIDER_SECRET="${1#*=}"
      shift
      ;;
    --web-port)
      WEB_PORT="$(require_value "$1" "${2-}")"
      shift 2
      ;;
    --web-port=*)
      WEB_PORT="${1#*=}"
      shift
      ;;
    --no-start|--prepare-only)
      START_CONTAINERS=0
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      fail "Unknown option: $1"
      ;;
  esac
done

command -v docker >/dev/null 2>&1 || fail "Docker is required. Install Docker Engine or Docker Desktop first."

if docker compose version >/dev/null 2>&1; then
  compose() { docker compose "$@"; }
elif command -v docker-compose >/dev/null 2>&1; then
  compose() { docker-compose "$@"; }
else
  fail "Docker Compose is required. Install the Docker Compose plugin or docker-compose first."
fi

if [ -d ./docker ]; then
  echo "Using existing ./docker directory."
else
  command -v tar >/dev/null 2>&1 || fail "tar is required to unpack the 1flowbase archive."
  if command -v curl >/dev/null 2>&1; then
    download() { curl -fsSL "$1" -o "$2"; }
  elif command -v wget >/dev/null 2>&1; then
    download() { wget -qO "$2" "$1"; }
  else
    fail "curl or wget is required to download the 1flowbase docker files."
  fi

  tmpdir="$(mktemp -d 2>/dev/null || mktemp -d -t 1flowbase)"
  trap 'rm -rf "$tmpdir"' EXIT HUP INT TERM
  archive="$tmpdir/1flowbase.tar.gz"
  echo "Downloading 1flowbase docker files."
  download "$FLOWBASE_ARCHIVE_URL" "$archive"
  tar -xzf "$archive" -C "$tmpdir" "$FLOWBASE_ARCHIVE_DOCKER_DIR"
  mv "$tmpdir/$FLOWBASE_ARCHIVE_DOCKER_DIR" ./docker
  echo "Downloaded ./docker."
fi

if [ ! -f ./docker/.env ]; then
  cp ./docker/.env.example ./docker/.env
  echo "Created docker/.env from docker/.env.example."
else
  echo "Using existing docker/.env."
fi

if [ -n "$DB_PASSWORD" ]; then
  set_env_value POSTGRES_PASSWORD "$DB_PASSWORD" ./docker/.env
  echo "Updated POSTGRES_PASSWORD in docker/.env."
fi
if [ -n "$ROOT_ACCOUNT" ]; then
  set_env_value BOOTSTRAP_ROOT_ACCOUNT "$ROOT_ACCOUNT" ./docker/.env
  echo "Updated BOOTSTRAP_ROOT_ACCOUNT in docker/.env."
fi
if [ -n "$ROOT_PASSWORD" ]; then
  set_env_value BOOTSTRAP_ROOT_PASSWORD "$ROOT_PASSWORD" ./docker/.env
  echo "Updated BOOTSTRAP_ROOT_PASSWORD in docker/.env."
fi
if [ -n "$PROVIDER_SECRET" ]; then
  set_env_value API_PROVIDER_SECRET_MASTER_KEY "$PROVIDER_SECRET" ./docker/.env
  echo "Updated API_PROVIDER_SECRET_MASTER_KEY in docker/.env."
fi
if [ -n "$WEB_PORT" ]; then
  set_env_value WEB_PORT "$WEB_PORT" ./docker/.env
  echo "Updated WEB_PORT in docker/.env."
fi

if [ "$START_CONTAINERS" -eq 0 ]; then
  echo "Docker files are ready in ./docker."
  echo "No containers were started because --no-start was used."
  echo "To start later, run: cd docker && docker compose pull && docker compose up -d"
  exit 0
fi

docker info >/dev/null 2>&1 || fail "Docker is installed but the daemon is not reachable. Start Docker and try again."

cd docker
compose pull
compose up -d

web_port="$(read_env_value WEB_PORT .env)"
root_account="$(read_env_value BOOTSTRAP_ROOT_ACCOUNT .env)"
root_password="$(read_env_value BOOTSTRAP_ROOT_PASSWORD .env)"

echo "1flowbase is starting. Web: http://127.0.0.1:${web_port:-3100}"
echo "Initial root account: ${root_account:-root}"
echo "Initial root password: ${root_password:-1flowbase}"
