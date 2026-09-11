#!/usr/bin/env bash
set -euo pipefail

script_dir=$(cd -- "${BASH_SOURCE[0]%/*}" && pwd)
if [[ -f "$script_dir/../../Cargo.toml" ]]; then
  project_root=$(cd -- "$script_dir/../.." && pwd)
  state_dir="$project_root/scripts"
else
  project_root="$script_dir"
  state_dir="$script_dir"
fi
if [[ ${CRYPTOOL_BIN+x} ]]; then
  binary_path=$CRYPTOOL_BIN
elif [[ -e "$script_dir/cryptool" ]]; then
  binary_path="$script_dir/cryptool"
elif [[ -e "$state_dir/cryptool" ]]; then
  binary_path="$state_dir/cryptool"
else
  binary_path="$project_root/target/release/cryptool"
fi
listen_address=${CRYPTOOL_LISTEN:-0.0.0.0:8080}
pid_file=${CRYPTOOL_PID_FILE:-$state_dir/cryptool.pid}
log_file=${CRYPTOOL_LOG_FILE:-$state_dir/cryptool.log}
action=${1:-start}
allowed_hosts=()

fail() {
  printf 'cryptool-web-server: %s\n' "$1" >&2
  exit 1
}

usage() {
  cat <<EOF
Usage: ${0##*/} [start|stop|restart|status|run]

  start    Start the Web server in background (default)
  stop     Stop the background Web server
  restart  Stop and start the background Web server
  status   Show whether the background Web server is running
  run      Run in foreground; Ctrl+C stops the server

PID file: $pid_file
Log file: $log_file
EOF
}

read_server_pid() {
  [[ -f "$pid_file" ]] || return 1
  IFS= read -r server_pid < "$pid_file" || return 1
  [[ "$server_pid" =~ ^[0-9]+$ ]] || return 1
}

server_is_running() {
  local process_state
  read_server_pid || return 1
  kill -0 "$server_pid" 2>/dev/null || return 1
  process_state=$(ps -p "$server_pid" -o stat= 2>/dev/null || true)
  [[ "$process_state" != Z* ]]
}

clear_pid_file() {
  [[ -e "$pid_file" ]] || return 0
  : > "$pid_file" || fail "cannot clear PID file: $pid_file"
}

remove_stale_pid_file() {
  if [[ -e "$pid_file" ]] && ! server_is_running; then
    clear_pid_file
  fi
}

show_status() {
  if server_is_running; then
    printf 'Exchange Data Web is running (PID %s)\n' "$server_pid"
    printf 'Log: %s\n' "$log_file"
    return 0
  fi
  remove_stale_pid_file
  printf 'Exchange Data Web is stopped\n'
  return 3
}

stop_server() {
  local attempt
  if ! server_is_running; then
    remove_stale_pid_file
    printf 'Exchange Data Web is already stopped\n'
    return 0
  fi

  printf 'Stopping Exchange Data Web (PID %s)\n' "$server_pid"
  kill "$server_pid" 2>/dev/null || fail "cannot send TERM to PID $server_pid"
  for attempt in {1..100}; do
    if ! server_is_running; then
      clear_pid_file
      printf 'Exchange Data Web stopped\n'
      return 0
    fi
    sleep 0.1
  done
  fail "server did not stop within 10 seconds (PID $server_pid)"
}

add_host() {
  local candidate=$1
  local existing
  [[ -n "$candidate" ]] || return 0
  if [[ ${#allowed_hosts[@]} -gt 0 ]]; then
    for existing in "${allowed_hosts[@]}"; do
      [[ "$existing" == "$candidate" ]] && return 0
    done
  fi
  allowed_hosts+=("$candidate")
}

is_ipv4() {
  local address=$1
  local old_ifs=$IFS
  local octet
  local octets

  [[ "$address" =~ ^[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+$ ]] || return 1
  IFS=.
  octets=($address)
  IFS=$old_ifs
  [[ ${#octets[@]} -eq 4 ]] || return 1
  for octet in "${octets[@]}"; do
    ((10#$octet <= 255)) || return 1
  done
}

add_discovered_ipv4() {
  local output=$1
  local candidate
  for candidate in $output; do
    candidate=${candidate%%/*}
    if is_ipv4 "$candidate" && [[ "$candidate" != 127.* ]]; then
      add_host "$candidate"
    fi
  done
}

add_ip_address_output() {
  local output=$1
  local candidate
  local previous=""
  for candidate in $output; do
    if [[ "$previous" == "inet" ]]; then
      add_discovered_ipv4 "$candidate"
    fi
    previous=$candidate
  done
}

discover_lan_hosts() {
  local output
  local interface_name

  if command -v hostname >/dev/null 2>&1; then
    output=$(hostname -I 2>/dev/null || true)
    add_discovered_ipv4 "$output"
  fi

  if command -v ip >/dev/null 2>&1; then
    output=$(ip -o -4 addr show scope global 2>/dev/null || true)
    add_ip_address_output "$output"
  fi

  if command -v ipconfig >/dev/null 2>&1; then
    for interface_name in en0 en1; do
      output=$(ipconfig getifaddr "$interface_name" 2>/dev/null || true)
      add_discovered_ipv4 "$output"
    done
  fi
}

is_loopback_listen() {
  case "$listen_address" in
    127.*:* | localhost:* | \[::1\]:*) return 0 ;;
    *) return 1 ;;
  esac
}

[[ $# -le 1 ]] || {
  usage >&2
  fail "only one action may be specified"
}

case "$action" in
  status)
    if show_status; then
      exit 0
    else
      exit 3
    fi
    ;;
  stop)
    stop_server
    exit 0
    ;;
  restart)
    stop_server
    ;;
  start | run)
    ;;
  -h | --help | help)
    usage
    exit 0
    ;;
  *)
    usage >&2
    fail "unknown action: $action"
    ;;
esac

[[ -f "$binary_path" && -x "$binary_path" ]] \
  || fail "cryptool binary is not an executable file: $binary_path; run scripts/web/build-local.sh first or set CRYPTOOL_BIN"

if command -v file >/dev/null 2>&1 && command -v uname >/dev/null 2>&1; then
  binary_description=$(file -b "$binary_path" 2>/dev/null || true)
  host_system=$(uname -s 2>/dev/null || true)
  case "$host_system:$binary_description" in
    Linux:*Mach-O* | Linux:*PE32*)
      fail "binary format does not match Linux: $binary_description; rebuild cryptool on a compatible Linux host"
      ;;
    Darwin:*ELF* | Darwin:*PE32*)
      fail "binary format does not match macOS: $binary_description; rebuild cryptool on macOS"
      ;;
  esac
fi

config_path=""
if [[ ${CRYPTOOL_CONFIG+x} ]]; then
  config_path=$CRYPTOOL_CONFIG
elif [[ -f "$script_dir/config.toml" ]]; then
  config_path="$script_dir/config.toml"
elif [[ -f "$state_dir/config.toml" ]]; then
  config_path="$state_dir/config.toml"
elif [[ -f "$project_root/config.toml" ]]; then
  config_path="$project_root/config.toml"
fi

if [[ -n "$config_path" && (! -f "$config_path" || ! -r "$config_path") ]]; then
  fail "configuration file is not a readable file: $config_path"
fi

if [[ -n ${CRYPTOOL_ALLOW_HOSTS:-} ]]; then
  explicit_hosts=${CRYPTOOL_ALLOW_HOSTS//,/ }
  for host in $explicit_hosts; do
    add_host "$host"
  done
elif ! is_loopback_listen; then
  discover_lan_hosts
fi

if ! is_loopback_listen && [[ ${#allowed_hosts[@]} -eq 0 ]]; then
  fail "no LAN IPv4 address was discovered; set CRYPTOOL_ALLOW_HOSTS to a comma-separated IP/domain list"
fi

command_args=()
if [[ -n "$config_path" ]]; then
  command_args+=(--config "$config_path")
fi
command_args+=(serve --listen "$listen_address")

if [[ ${#allowed_hosts[@]} -gt 0 ]]; then
  for host in "${allowed_hosts[@]}"; do
    command_args+=(--allow-host "$host")
  done
fi

printf 'Starting Exchange Data Web on %s\n' "$listen_address"
if [[ ${#allowed_hosts[@]} -gt 0 ]]; then
  listen_port=${listen_address##*:}
  for host in "${allowed_hosts[@]}"; do
    printf 'LAN URL: http://%s:%s/\n' "$host" "$listen_port"
  done
fi

if [[ "$action" == "run" ]]; then
  exec "$binary_path" "${command_args[@]}"
fi

if server_is_running; then
  fail "Exchange Data Web is already running (PID $server_pid)"
fi
remove_stale_pid_file

if ! : >> "$log_file"; then
  fail "cannot write log file: $log_file"
fi

nohup "$binary_path" "${command_args[@]}" >> "$log_file" 2>&1 </dev/null &
server_pid=$!
if ! printf '%s\n' "$server_pid" > "$pid_file"; then
  kill "$server_pid" 2>/dev/null || true
  fail "cannot write PID file: $pid_file"
fi

sleep 1
if ! server_is_running; then
  wait "$server_pid" 2>/dev/null || true
  clear_pid_file
  fail "server exited during startup; inspect log: $log_file"
fi

printf 'Exchange Data Web started in background (PID %s)\n' "$server_pid"
printf 'Log: %s\n' "$log_file"
