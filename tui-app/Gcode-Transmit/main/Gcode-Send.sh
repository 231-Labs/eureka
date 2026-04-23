#!/bin/bash
# Get the directory of the script
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# shellcheck source=/dev/null
source "$SCRIPT_DIR/../common-device.sh"

# Get the parent directory path
PARENT_DIR="$(dirname "$SCRIPT_DIR")"

prusa-slicer --export-gcode --load Ender-3_set.ini --output test.gcode "$PARENT_DIR/test.stl" &
echo $! > "$SCRIPT_DIR/Gcode-Send-PID.pid"
# echo $! > Gcode-Send-PID.pid
wait $!
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
echo $! > "$SCRIPT_DIR/Gcode-Send-PID.pid"
# echo $! > Gcode-Send-PID.pid
wait $!
rm -rf test.gcode

