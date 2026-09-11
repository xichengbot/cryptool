#!/usr/bin/env bash
set -euo pipefail

script_dir=$(cd -- "${BASH_SOURCE[0]%/*}" && pwd)
source "$script_dir/../lib/common.sh"
target=x86_64-unknown-linux-gnu.2.17
package=true

usage() {
  cat <<'EOF'
Usage: build-ubuntu.sh [--target TARGET] [--no-package]

Cross-build the embedded Web server for Ubuntu x86_64 with cargo-zigbuild.
Default target: x86_64-unknown-linux-gnu.2.17
EOF
}

while (($#)); do
  case "$1" in
    --target) [[ $# -ge 2 ]] || fail "--target requires a value"; target=$2; shift 2 ;;
    --no-package) package=false; shift ;;
    -h|--help|help) usage; exit 0 ;;
    *) usage >&2; fail "unknown argument: $1" ;;
  esac
done

for command in cargo zig file strings sort; do require_command "$command"; done
cargo zigbuild --help >/dev/null 2>&1 || fail "cargo-zigbuild is not installed"
"$script_dir/build-local.sh" --assets-only

step "Cross-building Ubuntu Web server ($target)"
(cd "$PROJECT_ROOT" && cargo zigbuild -p cryptool --release --locked --features embed-web --target "$target")
cargo_target=${target%%.*}
target_dir=$(cargo_target_dir)
export CARGO_TARGET_DIR="$target_dir"
binary="$target_dir/$cargo_target/release/cryptool"
[[ -x "$binary" ]] || fail "Ubuntu binary is missing or not executable: $binary"
description=$(file -b "$binary")
[[ "$description" == *ELF\ 64-bit* && "$description" == *x86-64* ]] || fail "unexpected binary format: $description"
[[ "$description" == *interpreter* ]] || fail "ELF interpreter is missing: $description"

highest=$(strings "$binary" | grep -Eo 'GLIBC_[0-9]+(\.[0-9]+)*' | sed 's/GLIBC_//' | sort -Vu | tail -1)
[[ -n "$highest" ]] || fail "could not determine the required GLIBC version"
[[ $(printf '%s\n%s\n' "$highest" 2.17 | sort -V | tail -1) == 2.17 ]] \
  || fail "binary requires GLIBC_$highest; expected at most GLIBC_2.17"
printf 'Ubuntu binary: %s\nRequired GLIBC: %s\n' "$binary" "$highest"

if $package; then
  "$script_dir/package-ubuntu.sh" --binary "$binary"
fi
