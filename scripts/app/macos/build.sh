#!/usr/bin/env bash
set -euo pipefail

script_dir=$(cd -- "${BASH_SOURCE[0]%/*}" && pwd)
source "$script_dir/../../lib/common.sh"

usage() {
  cat <<'EOF'
Usage: scripts/app/macos/build.sh

Build, ad-hoc sign, and verify dist/macos/ExchangeData.app.
Set DX to override the Dioxus CLI executable.
EOF
}

if [[ ${1:-} == -h || ${1:-} == --help ]]; then
  usage
  exit 0
fi
[[ $# -eq 0 ]] || fail 'macOS build does not accept positional arguments'
[[ $(uname -s) == Darwin ]] || fail 'macOS application build requires Darwin'

dx_tool=${DX:-dx}
for command in cargo "$dx_tool" codesign plutil file find; do
  require_command "$command"
done

cargo_target=$(cargo_target_dir)
export CARGO_TARGET_DIR="$cargo_target"
release_bundle="$cargo_target/dx/exchange-data/release/macos"
package_bundle="$cargo_target/dx/exchange-data/bundle/macos"
output_app="$PROJECT_ROOT/dist/macos/ExchangeData.app"

step 'Cleaning previous macOS bundles'
clean_generated_path "$release_bundle" "$cargo_target"
clean_generated_path "$package_bundle" "$cargo_target"
clean_generated_path "$output_app"

step 'Building macOS application'
(
  cd "$PROJECT_ROOT/apps/exchange-ui"
  "$dx_tool" bundle --macos --release --no-default-features --features desktop \
    --cargo-args=--locked --package-types macos --out-dir "$PROJECT_ROOT/dist/macos"
)

[[ -x "$output_app/Contents/MacOS/exchange-data" ]] \
  || fail "application executable not found: $output_app"
[[ $(file -b "$output_app/Contents/MacOS/exchange-data") == *Mach-O* ]] \
  || fail 'macOS application executable is not Mach-O'

step 'Signing and verifying macOS application'
codesign --force --deep --sign - "$output_app"
plutil -lint "$output_app/Contents/Info.plist"
icon_file=$(plutil -extract CFBundleIconFile raw -o - "$output_app/Contents/Info.plist")
[[ -n "$icon_file" && -f "$output_app/Contents/Resources/$icon_file" ]] \
  || fail 'CFBundleIconFile does not reference a bundled icon'
codesign --verify --deep --strict --verbose=2 "$output_app"

printf 'macOS application: %s\n' "$output_app"
du -sh "$output_app"
