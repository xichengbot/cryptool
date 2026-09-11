#!/usr/bin/env bash
set -euo pipefail

script_dir=$(cd -- "${BASH_SOURCE[0]%/*}" && pwd)
source "$script_dir/../../lib/common.sh"

usage() {
  cat <<'EOF'
Usage: scripts/app/macos/install.sh [--source APP] [--target DIRECTORY]

Defaults:
  --source  dist/macos/ExchangeData.app
  --target  /Applications
EOF
}

source_app="$PROJECT_ROOT/dist/macos/ExchangeData.app"
target_dir=/Applications
while [[ $# -gt 0 ]]; do
  case "$1" in
    --source) [[ $# -ge 2 ]] || fail '--source requires a path'; source_app=$2; shift 2 ;;
    --target) [[ $# -ge 2 ]] || fail '--target requires a directory'; target_dir=$2; shift 2 ;;
    -h | --help) usage; exit 0 ;;
    *) fail "unknown argument: $1" ;;
  esac
done

[[ $(uname -s) == Darwin ]] || fail 'macOS application install requires Darwin'
for command in codesign ditto mktemp mv find pgrep touch; do
  require_command "$command"
done
[[ -d "$source_app" ]] || fail "source application not found: $source_app"
codesign --verify --deep --strict "$source_app"
[[ -d "$target_dir" && -w "$target_dir" ]] \
  || fail "target directory is not writable: $target_dir; rerun with sudo or choose --target"
if pgrep -x exchange-data >/dev/null 2>&1; then
  fail 'ExchangeData is running; quit it before installing'
fi

destination="$target_dir/ExchangeData.app"
staging_dir=$(mktemp -d "$target_dir/.exchange-data-install.XXXXXX")
backup_app="$target_dir/.ExchangeData.app.backup.$$"
cleanup() {
  [[ ! -e "$staging_dir" ]] || find "$staging_dir" -depth -delete
}
trap cleanup EXIT

step "Staging application in $target_dir"
ditto "$source_app" "$staging_dir/ExchangeData.app"
codesign --verify --deep --strict "$staging_dir/ExchangeData.app"

if [[ -e "$destination" ]]; then
  mv "$destination" "$backup_app"
fi
if ! mv "$staging_dir/ExchangeData.app" "$destination"; then
  [[ ! -e "$backup_app" ]] || mv "$backup_app" "$destination"
  fail "failed to install application: $destination"
fi
[[ ! -e "$backup_app" ]] || find "$backup_app" -depth -delete
touch "$destination"
codesign --verify --deep --strict --verbose=2 "$destination"
printf 'Installed macOS application: %s\n' "$destination"
