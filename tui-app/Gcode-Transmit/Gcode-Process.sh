#!/bin/bash

Options_control="$1"
# Get the directory of the script
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=/dev/null
source "$SCRIPT_DIR/common-device.sh"

if [ "$Options_control" == "--print" ]; then

  USB_Device="$(eureka_resolve_printer_device)" || USB_Device=""
  if [ -z "$USB_Device" ]; then
    echo "Printer not connected!"
    exit 1
  fi
  export EUREKA_PRINTER_DEVICE="$USB_Device"

  # Propagate Gcode-Send.sh's exit code; previously we wrote `$?` to a file which
  # silently swallowed failures (slicer/serial) and made the caller think the
  # print succeeded — triggering an on-chain `complete_print_job` for a job that
  # never actually printed.
  "$SCRIPT_DIR/main/Gcode-Send.sh" &
  send_pid=$!
  wait "$send_pid"
  send_status=$?
  echo "$send_status" > "$SCRIPT_DIR/Gcode-Send-Status"
  exit "$send_status"

elif [ "$Options_control" == "--stop" ]; then

  "$SCRIPT_DIR/main/Gcode-Stop.sh"
  stop_status=$?
  echo "$stop_status" > "$SCRIPT_DIR/Gcode-Stop-Status"
  exit "$stop_status"


elif [ "$Options_control" == "--help" ]; then

  echo "Usage: Gcode-Process.sh [option]"
  echo ""
  echo "Options:"
  echo "  --print    Send G-code to the printer"
  echo "  --stop     Stop G-code transfer"
  echo "  --help     Show this help"

else

  echo "$Options_control is not a valid Gcode-Process option; use '--help'"

fi
