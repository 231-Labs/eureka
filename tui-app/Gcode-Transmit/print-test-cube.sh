#!/usr/bin/env bash
# One-shot: slice test_cube_20mm.stl (no chain / Walrus) and send G-code via eureka-serial.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=/dev/null
source "$SCRIPT_DIR/common-device.sh"

MAIN_DIR="$SCRIPT_DIR/main"
STL="$SCRIPT_DIR/test_cube_20mm.stl"
GCODE_NAME="test_cube_20mm_print.gcode"

if [[ ! -f "$STL" ]]; then
  echo "error: missing STL: $STL"
  exit 1
fi

cd "$MAIN_DIR"
echo "Slicing: $STL"
prusa-slicer --export-gcode --load Ender-3_set.ini --output "$GCODE_NAME" "$STL"
if [[ ! -f "$GCODE_NAME" ]]; then
  echo "error: slicer did not produce $MAIN_DIR/$GCODE_NAME (check prusa-slicer and Ender-3_set.ini)"
  exit 2
fi

if SERIAL_BIN="$(eureka_find_serial_bin "$MAIN_DIR")"; then
  if [[ -z "${EUREKA_PRINTER_DEVICE:-}" ]]; then
    EUREKA_PRINTER_DEVICE="$(eureka_resolve_printer_device)" || true
    export EUREKA_PRINTER_DEVICE
  fi
  if [[ -z "${EUREKA_PRINTER_DEVICE:-}" ]]; then
    echo "No serial device found (set EUREKA_PRINTER_DEVICE or add /dev/3Dprinter — see README.md)"
    exit 1
  fi
  echo "Sending G-code with: $SERIAL_BIN (device: $EUREKA_PRINTER_DEVICE)"
  "$SERIAL_BIN" "$GCODE_NAME"
else
  if [[ ! -e /dev/3Dprinter ]]; then
    echo "Legacy ./serial needs /dev/3Dprinter. Build eureka-serial: (cd tui-app && cargo build --release)"
    exit 1
  fi
  # Legacy sender expects ./test.gcode in this directory.
  cp -f "$GCODE_NAME" test.gcode
  echo "Sending G-code with: ./serial (test.gcode)"
  ./serial
fi

if [[ -z "${EUREKA_KEEP_GCODE:-}" ]]; then
  rm -f "$MAIN_DIR/$GCODE_NAME"
fi

echo "Done."
