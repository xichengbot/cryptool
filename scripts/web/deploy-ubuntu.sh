#!/usr/bin/env bash
set -euo pipefail

script_dir=$(cd -- "${BASH_SOURCE[0]%/*}" && pwd)
source "$script_dir/../lib/common.sh"

source_dir="$PROJECT_ROOT/dist/linux-x86_64"
target_dir=/data/cryptool
service_user=cryptool
start_after=false

usage() {
  cat <<'EOF'
Usage: deploy-ubuntu.sh [--source DIR] [--target DIR] [--user USER] [--start]

Install an Ubuntu Web release directory. Must run as root on Linux x86_64.
Existing config.toml is preserved. The service is started only with --start.
EOF
}

while (($#)); do
  case "$1" in
    --source) [[ $# -ge 2 ]] || fail "--source requires a directory"; source_dir=$2; shift 2 ;;
    --target) [[ $# -ge 2 ]] || fail "--target requires a directory"; target_dir=$2; shift 2 ;;
    --user) [[ $# -ge 2 ]] || fail "--user requires a name"; service_user=$2; shift 2 ;;
    --start) start_after=true; shift ;;
    -h|--help|help) usage; exit 0 ;;
    *) usage >&2; fail "unknown argument: $1" ;;
  esac
done

[[ $(uname -s) == Linux && $(uname -m) == x86_64 ]] || fail "deployment requires Linux x86_64"
[[ $(id -u) -eq 0 ]] || fail "deployment must run as root"
for command in file install id useradd runuser touch chown chmod; do require_command "$command"; done
source_dir=$(cd -- "$source_dir" && pwd)
binary="$source_dir/cryptool"
server="$source_dir/cryptool-web-server.sh"
[[ -x "$binary" && -x "$server" ]] || fail "release directory is incomplete: $source_dir"
file -b "$binary" | grep -q 'ELF 64-bit.*x86-64' || fail "cryptool is not a Linux x86-64 binary"
[[ ! -L "$target_dir" ]] || fail "refusing symbolic-link target directory: $target_dir"

if ! id "$service_user" >/dev/null 2>&1; then
  useradd --system --user-group --home-dir "$target_dir" --shell /usr/sbin/nologin "$service_user"
fi
service_group=$(id -gn "$service_user")

if [[ -x "$target_dir/cryptool-web-server.sh" ]]; then
  runuser -u "$service_user" -- "$target_dir/cryptool-web-server.sh" stop
fi

install -d -m 0755 -o root -g root "$target_dir"
for managed_file in cryptool cryptool-web-server.sh README.md WEB-UBUNTU.md; do
  [[ ! -L "$target_dir/$managed_file" ]] \
    || fail "refusing symbolic-link managed file: $target_dir/$managed_file"
done
install -m 0755 -o root -g root "$binary" "$target_dir/cryptool"
install -m 0755 -o root -g root "$server" "$target_dir/cryptool-web-server.sh"
for name in README.md WEB-UBUNTU.md; do
  [[ ! -f "$source_dir/$name" ]] || install -m 0644 -o root -g root "$source_dir/$name" "$target_dir/$name"
done
[[ ! -L "$target_dir/config.toml" ]] || fail "refusing symbolic-link configuration: $target_dir/config.toml"
if [[ ! -f "$target_dir/config.toml" ]]; then
  [[ -f "$source_dir/config.example.toml" ]] || fail "missing configuration template"
  install -m 0640 -o root -g "$service_group" "$source_dir/config.example.toml" "$target_dir/config.toml"
else
  chown root:"$service_group" "$target_dir/config.toml"
  chmod 0640 "$target_dir/config.toml"
fi
for state_file in cryptool.log cryptool.pid; do
  state_path="$target_dir/$state_file"
  [[ ! -L "$state_path" ]] || fail "refusing symbolic-link state file: $state_path"
  if [[ ! -e "$state_path" ]]; then
    install -m 0644 -o "$service_user" -g "$service_group" /dev/null "$state_path"
  else
    [[ -f "$state_path" ]] || fail "state path is not a regular file: $state_path"
    chown "$service_user:$service_group" "$state_path"
    chmod 0644 "$state_path"
  fi
done

if $start_after; then
  runuser -u "$service_user" -- "$target_dir/cryptool-web-server.sh" start
  runuser -u "$service_user" -- "$target_dir/cryptool-web-server.sh" status
fi
printf 'Deployed to %s (user: %s)\n' "$target_dir" "$service_user"
