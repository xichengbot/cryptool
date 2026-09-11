#!/usr/bin/env bash
set -euo pipefail
script_dir=$(cd -- "${BASH_SOURCE[0]%/*}" && pwd)
exec "$script_dir/app/macos/build.sh" "$@"
