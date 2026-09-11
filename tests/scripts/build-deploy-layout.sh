#!/usr/bin/env bash
set -euo pipefail

project_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)

required_files=(
  scripts/lib/common.sh
  scripts/app/macos/build.sh
  scripts/app/macos/install.sh
  scripts/app/windows/build.ps1
  scripts/app/windows/install.ps1
  scripts/web/build-local.sh
  scripts/web/build-ubuntu.sh
  scripts/web/package-ubuntu.sh
  scripts/web/deploy-ubuntu.sh
  scripts/web/server.sh
  scripts/build-desktop.sh
  scripts/build-desktop.ps1
  scripts/build-web.sh
  scripts/cryptool-web-server.sh
  docs/build-deploy/README.md
  docs/build-deploy/app-macos.md
  docs/build-deploy/app-windows.md
  docs/build-deploy/web-ubuntu.md
)

for relative_path in "${required_files[@]}"; do
  [[ -f "$project_root/$relative_path" ]] || {
    printf 'missing: %s\n' "$relative_path" >&2
    exit 1
  }
done

for relative_path in scripts/app/macos/*.sh scripts/web/*.sh scripts/build-desktop.sh scripts/build-web.sh scripts/cryptool-web-server.sh; do
  bash -n "$project_root/$relative_path"
done

grep -q 'start|stop|restart|status|run' <("$project_root/scripts/web/server.sh" --help)
deploy_help=$("$project_root/scripts/web/deploy-ubuntu.sh" --help)
for option in --source --target --user --start; do
  grep -q -- "$option" <<<"$deploy_help"
done

"$project_root/scripts/build-desktop.sh" --help >/dev/null
"$project_root/scripts/build-web.sh" --help >/dev/null
"$project_root/scripts/cryptool-web-server.sh" --help >/dev/null
[[ $(wc -l < "$project_root/scripts/build-desktop.sh") -le 6 ]]
[[ $(wc -l < "$project_root/scripts/build-web.sh") -le 6 ]]
[[ $(wc -l < "$project_root/scripts/cryptool-web-server.sh") -le 6 ]]

help_scripts=(
  scripts/app/macos/build.sh
  scripts/app/macos/install.sh
  scripts/web/build-local.sh
  scripts/web/build-ubuntu.sh
  scripts/web/package-ubuntu.sh
  scripts/web/deploy-ubuntu.sh
  scripts/web/server.sh
)

for relative_path in "${help_scripts[@]}"; do
  "$project_root/$relative_path" --help >/dev/null
done
