#!/bin/bash

Options_control="$1"
# Get the directory of the script
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

if [ "$Options_control" == "--print" ]; then

  USB_Device="/dev/3Dprinter"
  if [ ! -e "$USB_Device" ]; then
    echo "Printer not connected!"
    exit 1
  fi

  # Use relative path to execute sub-script
  "$SCRIPT_DIR/main/Gcode-Send.sh" &
  wait $!
  echo $? > "$SCRIPT_DIR/Gcode-Send-Status"

elif [ "$Options_control" == "--stop" ]; then

  # Use relative path to execute stop script
  "$SCRIPT_DIR/main/Gcode-Stop.sh"
  wait $!
  echo $? > "$SCRIPT_DIR/Gcode-Stop-Status"


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
