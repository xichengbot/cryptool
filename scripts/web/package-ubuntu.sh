#!/usr/bin/env bash
set -euo pipefail

script_dir=$(cd -- "${BASH_SOURCE[0]%/*}" && pwd)
source "$script_dir/../lib/common.sh"
target_dir=$(cargo_target_dir)
binary="$target_dir/x86_64-unknown-linux-gnu/release/cryptool"

usage() {
  cat <<'EOF'
Usage: package-ubuntu.sh [--binary PATH]

Create dist/cryptool-web-linux-x86_64.tar.gz and its SHA-256 checksum.
EOF
}

while (($#)); do
  case "$1" in
    --binary) [[ $# -ge 2 ]] || fail "--binary requires a path"; binary=$2; shift 2 ;;
    -h|--help|help) usage; exit 0 ;;
    *) usage >&2; fail "unknown argument: $1" ;;
  esac
done

for command in file tar gzip; do require_command "$command"; done
[[ -x "$binary" ]] || fail "binary is missing or not executable: $binary"
file -b "$binary" | grep -q 'ELF 64-bit.*x86-64' || fail "binary is not Linux x86-64"

release_dir="$PROJECT_ROOT/dist/linux-x86_64"
archive="$PROJECT_ROOT/dist/cryptool-web-linux-x86_64.tar.gz"
checksum="$archive.sha256"
mkdir -p "$PROJECT_ROOT/dist"
staged_binary=$(mktemp "$PROJECT_ROOT/dist/.cryptool-linux.XXXXXX")
cleanup() { [[ ! -e "$staged_binary" ]] || find "$staged_binary" -delete; }
trap cleanup EXIT
install -m 0755 "$binary" "$staged_binary"
clean_generated_path "$release_dir"
rm -f -- "$archive" "$checksum"
mkdir -p "$release_dir"
install -m 0755 "$staged_binary" "$release_dir/cryptool"
install -m 0755 "$script_dir/server.sh" "$release_dir/cryptool-web-server.sh"
install -m 0644 "$PROJECT_ROOT/config.example.toml" "$release_dir/config.example.toml"
install -m 0644 "$PROJECT_ROOT/docs/build-deploy/web-ubuntu.md" "$release_dir/WEB-UBUNTU.md"
cat > "$release_dir/README.md" <<'EOF'
# Exchange Data Web — Ubuntu x86_64

1. `cp config.example.toml config.toml`，按需配置代理和 API Key。
2. `./cryptool-web-server.sh start` 后台启动。
3. 使用 `status`、`restart`、`stop` 管理服务，日志位于 `cryptool.log`。

完整说明见 `WEB-UBUNTU.md`。
EOF

(cd "$PROJECT_ROOT/dist" && COPYFILE_DISABLE=1 tar -czf "${archive##*/}" linux-x86_64)
(cd "$PROJECT_ROOT/dist" && sha256_file "${archive##*/}" > "${checksum##*/}")
gzip -t "$archive"
(cd "$PROJECT_ROOT/dist" && if command -v sha256sum >/dev/null 2>&1; then sha256sum -c "${checksum##*/}"; else shasum -a 256 -c "${checksum##*/}"; fi)
tar -tzf "$archive" | grep -q 'linux-x86_64/cryptool-web-server.sh'
printf 'Ubuntu package: %s\nChecksum: %s\n' "$archive" "$checksum"
