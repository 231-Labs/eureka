#!/bin/bash
# Get the directory of the script
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# shellcheck source=/dev/null
source "$SCRIPT_DIR/../common-device.sh"

# Get the parent directory path
PARENT_DIR="$(dirname "$SCRIPT_DIR")"

# Pre-flight: make sure the STL exists and is non-empty. A 0-byte file (e.g. a
# failed Walrus download) would otherwise make prusa-slicer fail in a way that
# the caller currently misinterprets as success.
if [ ! -s "$PARENT_DIR/test.stl" ]; then
  echo "Missing or empty STL at $PARENT_DIR/test.stl — Walrus download likely failed or the model is encrypted but was not decrypted before slicing."
  exit 2
fi

prusa-slicer --export-gcode --load Ender-3_set.ini --output test.gcode "$PARENT_DIR/test.stl" &
slicer_pid=$!
echo "$slicer_pid" > "$SCRIPT_DIR/Gcode-Send-PID.pid"
wait "$slicer_pid"
slicer_status=$?
if [ "$slicer_status" -ne 0 ] || [ ! -s test.gcode ]; then
  echo "Slicer failed (exit=$slicer_status) — no usable test.gcode. Check prusa-slicer, Ender-3_set.ini, and $PARENT_DIR/test.stl (must be a valid STL)."
  exit 2
fi
rm -rf "$PARENT_DIR/test.stl"

if SERIAL_BIN="$(eureka_find_serial_bin "$SCRIPT_DIR")"; then
  if [ -z "${EUREKA_PRINTER_DEVICE:-}" ]; then
    EUREKA_PRINTER_DEVICE="$(eureka_resolve_printer_device)" || true
    export EUREKA_PRINTER_DEVICE
  fi
  if [ -z "${EUREKA_PRINTER_DEVICE:-}" ]; then
    echo "No serial device found (set EUREKA_PRINTER_DEVICE or add udev symlink /dev/3Dprinter — see README.md)"
    exit 1
  fi
  "$SERIAL_BIN" test.gcode &
else
  if [ ! -e /dev/3Dprinter ]; then
    echo "Legacy ./serial needs /dev/3Dprinter. Build eureka-serial: (cd tui-app && cargo build --release)"
    exit 1
  fi
  ./serial &
fi
serial_pid=$!
echo "$serial_pid" > "$SCRIPT_DIR/Gcode-Send-PID.pid"
wait "$serial_pid"
serial_status=$?

if [ -z "${EUREKA_KEEP_GCODE:-}" ]; then
  rm -rf test.gcode
fi

if [ "$serial_status" -ne 0 ]; then
  echo "Serial transmission failed (exit=$serial_status)."
  exit 3
fi
exit 0

