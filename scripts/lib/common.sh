#!/usr/bin/env bash

if [[ -z ${PROJECT_ROOT:-} ]]; then
  PROJECT_ROOT=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
fi

fail() {
  printf 'error: %s\n' "$1" >&2
  exit 1
}

step() {
  printf '==> %s\n' "$1"
}

require_command() {
  command -v "$1" >/dev/null 2>&1 || fail "required command not found: $1"
}

clean_generated_path() {
  local path=$1
  local cargo_root=${2:-$PROJECT_ROOT/target}
  case "$path" in
    "$cargo_root/dx/exchange-data/"* | "$PROJECT_ROOT/web-dist/"* | "$PROJECT_ROOT/dist/"*) ;;
    *) fail "refusing to clean path outside generated directories: $path" ;;
  esac
  [[ ! -e "$path" ]] || find "$path" -depth -delete
}

cargo_target_dir() {
  local configured=${CARGO_TARGET_DIR:-$PROJECT_ROOT/target}
  case "$configured" in
    /*) ;;
    *) configured="$PROJECT_ROOT/$configured" ;;
  esac
  mkdir -p "$configured"
  (cd -- "$configured" && pwd -P)
}

sha256_file() {
  local path=$1
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$path"
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$path"
  else
    fail 'required command not found: sha256sum or shasum'
  fi
}
