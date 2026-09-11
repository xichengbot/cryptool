#!/usr/bin/env bash
set -euo pipefail

project_root=$(cd -- "${BASH_SOURCE[0]%/*}/.." && pwd)
launcher="$project_root/scripts/cryptool-web-server.sh"
capture_binary="$project_root/tests/fixtures/capture-args.sh"
temp_root=${TMPDIR:-/tmp}
test_dir=$(mktemp -d "${temp_root%/}/cryptool-launcher-test.XXXXXX")
cleanup() {
  if [[ -f "$test_dir/server.pid" ]]; then
    kill "$(cat "$test_dir/server.pid")" 2>/dev/null || true
  fi
  rm -rf -- "$test_dir"
}
trap cleanup EXIT

passed=0

fail() {
  printf 'FAIL: %s\n' "$1" >&2
  exit 1
}

assert_arguments() {
  local expected=$1
  local actual
  actual=$(cat "$CRYPTOOL_TEST_ARGS_FILE")
  if [[ "$actual" != "$expected" ]]; then
    printf 'Expected arguments:\n%s\nActual arguments:\n%s\n' "$expected" "$actual" >&2
    fail "$2"
  fi
  passed=$((passed + 1))
}

export CRYPTOOL_TEST_ARGS_FILE="$test_dir/args"

bundle_dir="$test_dir/server-bundle"
mkdir -p "$bundle_dir"
cp "$launcher" "$bundle_dir/cryptool-web-server.sh"
cp "$capture_binary" "$bundle_dir/cryptool"
cp "$project_root/config.example.toml" "$bundle_dir/config.toml"
chmod +x "$bundle_dir/cryptool-web-server.sh" "$bundle_dir/cryptool"

CRYPTOOL_LISTEN="0.0.0.0:8080" \
CRYPTOOL_ALLOW_HOSTS="192.168.50.10" \
  "$bundle_dir/cryptool-web-server.sh" run >/dev/null
expected=$(printf '%s\n' \
  --config "$bundle_dir/config.toml" \
  serve --listen 0.0.0.0:8080 \
  --allow-host 192.168.50.10)
assert_arguments "$expected" "launcher should find cryptool and config.toml beside itself"

CRYPTOOL_BIN="$capture_binary" \
CRYPTOOL_CONFIG="$project_root/config.example.toml" \
CRYPTOOL_LISTEN="0.0.0.0:9090" \
CRYPTOOL_ALLOW_HOSTS="192.168.50.10, exchange.local" \
  "$launcher" run >/dev/null
expected=$(printf '%s\n' \
  --config "$project_root/config.example.toml" \
  serve --listen 0.0.0.0:9090 \
  --allow-host 192.168.50.10 \
  --allow-host exchange.local)
assert_arguments "$expected" "explicit settings were not forwarded exactly"

CRYPTOOL_BIN="$capture_binary" \
CRYPTOOL_CONFIG="" \
CRYPTOOL_LISTEN="127.0.0.1:8081" \
CRYPTOOL_ALLOW_HOSTS="" \
  "$launcher" run >/dev/null
expected=$(printf '%s\n' serve --listen 127.0.0.1:8081)
assert_arguments "$expected" "empty config should not add --config"

format_stub_dir="$test_dir/format-stub-bin"
mkdir -p "$format_stub_dir"
printf '%s\n' '#!/bin/bash' 'printf "%s\n" "Mach-O 64-bit executable x86_64"' \
  > "$format_stub_dir/file"
printf '%s\n' '#!/bin/bash' 'printf "%s\n" "Linux"' \
  > "$format_stub_dir/uname"
chmod +x "$format_stub_dir/file" "$format_stub_dir/uname"

if output=$(PATH="$format_stub_dir" \
  CRYPTOOL_BIN="$capture_binary" \
  CRYPTOOL_CONFIG="" \
  CRYPTOOL_ALLOW_HOSTS="192.168.50.10" \
  /bin/bash "$launcher" run 2>&1); then
  fail "a macOS binary should be rejected on Linux before exec"
fi
[[ "$output" == *"binary format does not match Linux"* ]] \
  || fail "binary format mismatch error is not actionable"
passed=$((passed + 1))

stub_dir="$test_dir/stub-bin"
mkdir -p "$stub_dir"
printf '%s\n' \
  '#!/bin/bash' \
  'if [[ ${1:-} == "-I" ]]; then' \
  '  printf "%s\n" "192.168.50.10 127.0.0.1 10.0.0.2 192.168.50.10 999.1.1.1"' \
  'else' \
  '  exit 1' \
  'fi' > "$stub_dir/hostname"
chmod +x "$stub_dir/hostname"

PATH="$stub_dir" \
CRYPTOOL_BIN="$capture_binary" \
CRYPTOOL_CONFIG="" \
CRYPTOOL_LISTEN="0.0.0.0:8080" \
CRYPTOOL_ALLOW_HOSTS="" \
  /bin/bash "$launcher" run >/dev/null
expected=$(printf '%s\n' \
  serve --listen 0.0.0.0:8080 \
  --allow-host 192.168.50.10 \
  --allow-host 10.0.0.2)
assert_arguments "$expected" "automatic discovery should add unique LAN IPv4 hosts"

ip_stub_dir="$test_dir/ip-stub-bin"
mkdir -p "$ip_stub_dir"
printf '%s\n' \
  '#!/bin/bash' \
  'printf "%s\n" "2: eth0 inet 192.168.1.129/24 brd 192.168.1.255 scope global dynamic eth0"' \
  > "$ip_stub_dir/ip"
chmod +x "$ip_stub_dir/ip"

PATH="$ip_stub_dir" \
CRYPTOOL_BIN="$capture_binary" \
CRYPTOOL_CONFIG="" \
CRYPTOOL_LISTEN="0.0.0.0:8080" \
CRYPTOOL_ALLOW_HOSTS="" \
  /bin/bash "$launcher" run >/dev/null
expected=$(printf '%s\n' \
  serve --listen 0.0.0.0:8080 \
  --allow-host 192.168.1.129)
assert_arguments "$expected" "ip discovery should ignore the interface broadcast address"

empty_path="$test_dir/empty-bin"
mkdir -p "$empty_path"
if output=$(PATH="$empty_path" \
  CRYPTOOL_BIN="$capture_binary" \
  CRYPTOOL_CONFIG="" \
  CRYPTOOL_LISTEN="0.0.0.0:8080" \
  CRYPTOOL_ALLOW_HOSTS="" \
  /bin/bash "$launcher" 2>&1); then
  fail "non-loopback listen without a discoverable host should fail"
fi
[[ "$output" == *"CRYPTOOL_ALLOW_HOSTS"* ]] \
  || fail "discovery failure should explain CRYPTOOL_ALLOW_HOSTS"
passed=$((passed + 1))

if output=$(CRYPTOOL_BIN="$test_dir/missing-cryptool" \
  CRYPTOOL_CONFIG="" \
  CRYPTOOL_ALLOW_HOSTS="192.168.50.10" \
  "$launcher" 2>&1); then
  fail "missing binary should fail"
fi
[[ "$output" == *"not an executable file"* ]] || fail "missing binary error is not actionable"
passed=$((passed + 1))

if output=$(CRYPTOOL_BIN="$test_dir" \
  CRYPTOOL_CONFIG="" \
  CRYPTOOL_ALLOW_HOSTS="192.168.50.10" \
  "$launcher" 2>&1); then
  fail "a directory should not be accepted as the binary"
fi
[[ "$output" == *"not an executable file"* ]] \
  || fail "invalid binary type error is not actionable"
passed=$((passed + 1))

if output=$(CRYPTOOL_BIN="$capture_binary" \
  CRYPTOOL_CONFIG="$test_dir/missing-config.toml" \
  CRYPTOOL_ALLOW_HOSTS="192.168.50.10" \
  "$launcher" 2>&1); then
  fail "missing explicit config should fail"
fi
[[ "$output" == *"configuration file is not a readable file"* ]] \
  || fail "missing config error is not actionable"
passed=$((passed + 1))

config_directory="$test_dir/config-directory"
mkdir -p "$config_directory"
if output=$(CRYPTOOL_BIN="$capture_binary" \
  CRYPTOOL_CONFIG="$config_directory" \
  CRYPTOOL_ALLOW_HOSTS="192.168.50.10" \
  "$launcher" 2>&1); then
  fail "a directory should not be accepted as the configuration file"
fi
[[ "$output" == *"configuration file is not a readable file"* ]] \
  || fail "invalid configuration type error is not actionable"
passed=$((passed + 1))

managed_binary="$test_dir/managed-cryptool"
cat > "$managed_binary" <<'EOF'
#!/usr/bin/env bash
printf '%s\n' "$@" > "$CRYPTOOL_TEST_ARGS_FILE"
trap 'exit 0' TERM INT
while :; do
  sleep 1
done
EOF
chmod +x "$managed_binary"
pid_file="$test_dir/server.pid"
log_file="$test_dir/server.log"

CRYPTOOL_BIN="$managed_binary" \
CRYPTOOL_CONFIG="$project_root/config.example.toml" \
CRYPTOOL_ALLOW_HOSTS="192.168.50.10" \
CRYPTOOL_PID_FILE="$pid_file" \
CRYPTOOL_LOG_FILE="$log_file" \
  "$launcher" >/dev/null
[[ -s "$pid_file" ]] || fail "start should write a PID file"
kill -0 "$(cat "$pid_file")" 2>/dev/null || fail "start should leave the server running in background"
passed=$((passed + 1))

output=$(CRYPTOOL_PID_FILE="$pid_file" CRYPTOOL_LOG_FILE="$log_file" "$launcher" status)
[[ "$output" == *"running"* ]] || fail "status should report a running server"
passed=$((passed + 1))

if output=$(CRYPTOOL_PID_FILE="$pid_file" CRYPTOOL_LOG_FILE="$log_file" "$launcher" start 2>&1); then
  fail "starting an already running server should fail"
fi
[[ "$output" == *"already running"* ]] || fail "duplicate start error should be actionable"
passed=$((passed + 1))

old_pid=$(cat "$pid_file")
CRYPTOOL_BIN="$managed_binary" \
CRYPTOOL_CONFIG="$project_root/config.example.toml" \
CRYPTOOL_ALLOW_HOSTS="192.168.50.10" \
CRYPTOOL_PID_FILE="$pid_file" \
CRYPTOOL_LOG_FILE="$log_file" \
  "$launcher" restart >/dev/null
new_pid=$(cat "$pid_file")
[[ "$new_pid" != "$old_pid" ]] || fail "restart should replace the server process"
kill -0 "$new_pid" 2>/dev/null || fail "restart should leave the server running"
passed=$((passed + 1))

CRYPTOOL_PID_FILE="$pid_file" CRYPTOOL_LOG_FILE="$log_file" "$launcher" stop >/dev/null
[[ ! -s "$pid_file" ]] || fail "stop should clear the PID file"
if output=$(CRYPTOOL_PID_FILE="$pid_file" CRYPTOOL_LOG_FILE="$log_file" "$launcher" status 2>&1); then
  fail "status should return non-zero for a stopped server"
fi
[[ "$output" == *"stopped"* ]] || fail "status should report a stopped server"
passed=$((passed + 1))

printf 'PASS: %s launcher contract checks\n' "$passed"
