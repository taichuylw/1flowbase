#!/usr/bin/env sh
set -eu

FLOWBASE_REPO="${FLOWBASE_REPO:-taichuy/1flowbase}"
FLOWBASE_REF="${FLOWBASE_REF:-main}"
FLOWBASE_ARCHIVE_URL="${FLOWBASE_ARCHIVE_URL:-https://codeload.github.com/${FLOWBASE_REPO}/tar.gz/refs/heads/${FLOWBASE_REF}}"
FLOWBASE_ARCHIVE_DOCKER_DIR="1flowbase-${FLOWBASE_REF}/docker"

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

command -v docker >/dev/null 2>&1 || fail "Docker is required. Install Docker Engine or Docker Desktop first."
docker info >/dev/null 2>&1 || fail "Docker is installed but the daemon is not reachable. Start Docker and try again."

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

cd docker
compose pull
compose up -d

web_port="$(read_env_value WEB_PORT .env)"
root_account="$(read_env_value BOOTSTRAP_ROOT_ACCOUNT .env)"
root_password="$(read_env_value BOOTSTRAP_ROOT_PASSWORD .env)"

echo "1flowbase is starting. Web: http://127.0.0.1:${web_port:-3100}"
echo "Initial root account: ${root_account:-root}"
echo "Initial root password: ${root_password:-1flowbase}"
