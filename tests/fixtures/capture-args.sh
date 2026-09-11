#!/bin/bash
set -euo pipefail

: "${CRYPTOOL_TEST_ARGS_FILE:?CRYPTOOL_TEST_ARGS_FILE is required}"
printf '%s\n' "$@" > "$CRYPTOOL_TEST_ARGS_FILE"
