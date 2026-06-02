#!/usr/bin/env sh
set -eu

FLOWBASE_REPO="${FLOWBASE_REPO:-taichuy/1flowbase}"
FLOWBASE_REF="${FLOWBASE_REF:-main}"
FLOWBASE_ARCHIVE_URL="${FLOWBASE_ARCHIVE_URL:-https://codeload.github.com/${FLOWBASE_REPO}/tar.gz/refs/heads/${FLOWBASE_REF}}"
FLOWBASE_ARCHIVE_DOCKER_DIR="1flowbase-${FLOWBASE_REF}/docker"
DEFAULT_OFFICIAL_PLUGIN_GITHUB_PROXY_URL="https://gh-proxy.com/"
DB_PASSWORD="${FLOWBASE_DB_PASSWORD:-}"
ROOT_ACCOUNT="${FLOWBASE_ROOT_ACCOUNT:-}"
ROOT_PASSWORD="${FLOWBASE_ROOT_PASSWORD:-}"
PROVIDER_SECRET="${FLOWBASE_PROVIDER_SECRET:-}"
WEB_PORT="${FLOWBASE_WEB_PORT:-}"
PLUGIN_GITHUB_PROXY_URL="${FLOWBASE_OFFICIAL_PLUGIN_GITHUB_PROXY_URL:-${API_OFFICIAL_PLUGIN_GITHUB_PROXY_URL:-}}"
PULL_IMAGES="${FLOWBASE_PULL_IMAGES:-}"
START_CONTAINERS="${FLOWBASE_START_CONTAINERS:-}"
INTERACTIVE=1

if [ "${FLOWBASE_NON_INTERACTIVE:-}" = "1" ] || [ "${FLOWBASE_NON_INTERACTIVE:-}" = "true" ]; then
  INTERACTIVE=0
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

read_from_tty() {
  if [ "$INTERACTIVE" -eq 1 ] && [ -r /dev/tty ]; then
    IFS= read -r value < /dev/tty || value=""
    printf '%s\n' "$value"
  else
    printf '%s\n' ""
  fi
}

prompt_env_value() {
  key="$1"
  label="$2"
  current_value="$(read_env_value "$key" ./docker/.env)"

  if [ -n "$current_value" ]; then
    printf '%s [%s]: ' "$label" "$current_value" > /dev/tty 2>/dev/null || true
  else
    printf '%s: ' "$label" > /dev/tty 2>/dev/null || true
  fi

  input="$(read_from_tty)"
  if [ -n "$input" ]; then
    set_env_value "$key" "$input" ./docker/.env
    echo "Updated ${key} in docker/.env."
  else
    echo "Keeping ${key}: ${current_value:-empty}"
  fi
}

normalize_yes_no() {
  case "$1" in
    y|Y|yes|YES|Yes|true|TRUE|1)
      printf '%s\n' "yes"
      ;;
    *)
      printf '%s\n' "no"
      ;;
  esac
}

prompt_yes_no() {
  question="$1"
  default_answer="$2"
  if [ "$default_answer" = "yes" ]; then
    suffix="[Y/n]"
  else
    suffix="[y/N]"
  fi

  printf '%s %s: ' "$question" "$suffix" > /dev/tty 2>/dev/null || true
  input="$(read_from_tty)"
  if [ -z "$input" ]; then
    printf '%s\n' "$default_answer"
  else
    normalize_yes_no "$input"
  fi
}

prompt_official_plugin_github_proxy_url() {
  current_value="$(read_env_value API_OFFICIAL_PLUGIN_GITHUB_PROXY_URL ./docker/.env)"
  default_answer="no"
  if [ -n "$current_value" ]; then
    default_answer="yes"
  fi

  enabled="$(prompt_yes_no "Use CN GitHub plugin download accelerator?" "$default_answer")"
  if [ "$enabled" = "yes" ]; then
    default_value="$current_value"
    if [ -z "$default_value" ]; then
      default_value="$DEFAULT_OFFICIAL_PLUGIN_GITHUB_PROXY_URL"
    fi

    printf 'Official plugin GitHub raw proxy URL [%s]: ' "$default_value" > /dev/tty 2>/dev/null || true
    input="$(read_from_tty)"
    if [ -n "$input" ]; then
      set_env_value API_OFFICIAL_PLUGIN_GITHUB_PROXY_URL "$input" ./docker/.env
    else
      set_env_value API_OFFICIAL_PLUGIN_GITHUB_PROXY_URL "$default_value" ./docker/.env
    fi
    echo "Updated API_OFFICIAL_PLUGIN_GITHUB_PROXY_URL in docker/.env."
  else
    set_env_value API_OFFICIAL_PLUGIN_GITHUB_PROXY_URL "" ./docker/.env
    echo "Disabled API_OFFICIAL_PLUGIN_GITHUB_PROXY_URL in docker/.env."
  fi
}

usage() {
  cat <<'EOF'
Usage: docker-deploy.sh [options]

Options:
  --db-password VALUE       Pre-fill POSTGRES_PASSWORD before the interactive prompt.
  --root-account VALUE      Pre-fill BOOTSTRAP_ROOT_ACCOUNT before the interactive prompt.
  --root-password VALUE     Pre-fill BOOTSTRAP_ROOT_PASSWORD before the interactive prompt.
  --provider-secret VALUE   Pre-fill API_PROVIDER_SECRET_MASTER_KEY before the interactive prompt.
  --web-port VALUE          Pre-fill WEB_PORT before the interactive prompt.
  --plugin-github-proxy-url VALUE
                            Pre-fill API_OFFICIAL_PLUGIN_GITHUB_PROXY_URL before the interactive prompt.
  --pull                    Pull images without asking.
  --no-pull                 Do not pull images without asking.
  --start                   Start containers without asking.
  --no-start                Do not start containers without asking.
  --non-interactive         Do not prompt. Defaults to no pull and no start unless --pull/--start are set.
  -h, --help                Show this help.

Environment variables with the same effect:
  FLOWBASE_DB_PASSWORD
  FLOWBASE_ROOT_ACCOUNT
  FLOWBASE_ROOT_PASSWORD
  FLOWBASE_PROVIDER_SECRET
  FLOWBASE_WEB_PORT
  FLOWBASE_OFFICIAL_PLUGIN_GITHUB_PROXY_URL
  FLOWBASE_PULL_IMAGES=1
  FLOWBASE_START_CONTAINERS=1
  FLOWBASE_NON_INTERACTIVE=1
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
    --plugin-github-proxy-url)
      PLUGIN_GITHUB_PROXY_URL="$(require_value "$1" "${2-}")"
      shift 2
      ;;
    --plugin-github-proxy-url=*)
      PLUGIN_GITHUB_PROXY_URL="${1#*=}"
      shift
      ;;
    --pull)
      PULL_IMAGES=1
      shift
      ;;
    --no-pull)
      PULL_IMAGES=0
      shift
      ;;
    --start)
      START_CONTAINERS=1
      shift
      ;;
    --no-start|--prepare-only)
      START_CONTAINERS=0
      shift
      ;;
    --non-interactive)
      INTERACTIVE=0
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
if [ -n "$PLUGIN_GITHUB_PROXY_URL" ]; then
  set_env_value API_OFFICIAL_PLUGIN_GITHUB_PROXY_URL "$PLUGIN_GITHUB_PROXY_URL" ./docker/.env
  echo "Updated API_OFFICIAL_PLUGIN_GITHUB_PROXY_URL in docker/.env."
fi

if [ "$INTERACTIVE" -eq 1 ] && [ -r /dev/tty ]; then
  echo "Configure docker/.env. Press Enter to keep the value shown in brackets."
  prompt_env_value POSTGRES_PASSWORD "Database password"
  prompt_env_value BOOTSTRAP_ROOT_ACCOUNT "Root account"
  prompt_env_value BOOTSTRAP_ROOT_PASSWORD "Root password"
  prompt_env_value API_PROVIDER_SECRET_MASTER_KEY "API provider secret master key"
  prompt_env_value WEB_PORT "Web port"
  prompt_official_plugin_github_proxy_url
elif [ "$INTERACTIVE" -eq 1 ]; then
  echo "No interactive terminal was found. Keeping docker/.env values."
fi

if [ -z "$PULL_IMAGES" ]; then
  if [ "$INTERACTIVE" -eq 1 ] && [ -r /dev/tty ]; then
    PULL_IMAGES="$(prompt_yes_no "Pull Docker images?" "no")"
  else
    PULL_IMAGES="no"
  fi
else
  PULL_IMAGES="$(normalize_yes_no "$PULL_IMAGES")"
fi

if [ -z "$START_CONTAINERS" ]; then
  if [ "$INTERACTIVE" -eq 1 ] && [ -r /dev/tty ]; then
    START_CONTAINERS="$(prompt_yes_no "Start 1flowbase now?" "no")"
  else
    START_CONTAINERS="no"
  fi
else
  START_CONTAINERS="$(normalize_yes_no "$START_CONTAINERS")"
fi

if [ "$PULL_IMAGES" = "no" ] && [ "$START_CONTAINERS" = "no" ]; then
  echo "Docker files are ready in ./docker."
  echo "No images were pulled and no containers were started."
  echo "To start later, run: cd docker && docker compose pull && docker compose up -d"
  exit 0
fi

docker info >/dev/null 2>&1 || fail "Docker is installed but the daemon is not reachable. Start Docker and try again."

cd docker
if [ "$PULL_IMAGES" = "yes" ]; then
  compose pull
else
  echo "Skipping image pull."
fi

if [ "$START_CONTAINERS" = "yes" ]; then
  compose up -d
else
  echo "Skipping container startup."
  echo "To start later, run: cd docker && docker compose up -d"
  exit 0
fi

web_port="$(read_env_value WEB_PORT .env)"
root_account="$(read_env_value BOOTSTRAP_ROOT_ACCOUNT .env)"
root_password="$(read_env_value BOOTSTRAP_ROOT_PASSWORD .env)"

echo "1flowbase is starting. Web: http://127.0.0.1:${web_port:-3100}"
echo "Initial root account: ${root_account:-root}"
echo "Initial root password: ${root_password:-1flowbase}"
