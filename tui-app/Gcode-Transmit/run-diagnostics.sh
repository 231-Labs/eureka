#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

exec ./diagnose-print.py \
  --stl test_cube_20mm.stl \
  --slice \
  --emit-control-gcode \
  "$@"
