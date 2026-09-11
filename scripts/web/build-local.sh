#!/usr/bin/env bash
set -euo pipefail

script_dir=$(cd -- "${BASH_SOURCE[0]%/*}" && pwd)
source "$script_dir/../lib/common.sh"
assets_only=false

usage() {
  cat <<'EOF'
Usage: build-local.sh [--assets-only]

Build Dioxus Web assets. By default also build the host Web server binary and
copy it to dist/web-local/cryptool.
EOF
}

while (($#)); do
  case "$1" in
    --assets-only) assets_only=true; shift ;;
    -h|--help|help) usage; exit 0 ;;
    *) usage >&2; fail "unknown argument: $1" ;;
  esac
done

dx_tool=${DX:-dx}
require_command "$dx_tool"
require_command cargo
require_command find

target_dir=$(cargo_target_dir)
export CARGO_TARGET_DIR="$target_dir"
clean_generated_path "$PROJECT_ROOT/web-dist/public"
clean_generated_path "$target_dir/dx/exchange-data/release/web/public" "$target_dir"

step "Building Dioxus Web assets"
(cd "$PROJECT_ROOT/apps/exchange-ui" && "$dx_tool" bundle --web --release \
  --debug-symbols=false --no-default-features --features web --cargo-args=--locked \
  --out-dir "$PROJECT_ROOT/web-dist")

public="$PROJECT_ROOT/web-dist/public"
[[ -s "$public/index.html" ]] || fail "missing Web index: $public/index.html"
for pattern in '*.wasm' '*.js' '*.css' '*favicon*'; do
  find "$public" -type f -name "$pattern" -size +0c -print -quit | grep -q . \
    || fail "missing generated Web asset: $pattern"
done

$assets_only && exit 0
step "Building local embedded Web server"
(cd "$PROJECT_ROOT" && cargo build -p cryptool --release --locked --features embed-web)
clean_generated_path "$PROJECT_ROOT/dist/web-local"
mkdir -p "$PROJECT_ROOT/dist/web-local"
install -m 0755 "$target_dir/release/cryptool" "$PROJECT_ROOT/dist/web-local/cryptool"
printf 'Local Web binary: %s\n' "$PROJECT_ROOT/dist/web-local/cryptool"
