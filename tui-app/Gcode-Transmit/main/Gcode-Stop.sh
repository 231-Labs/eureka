#!/bin/bash
# Get the directory of the script
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# shellcheck source=/dev/null
source "$SCRIPT_DIR/../common-device.sh"

# Use relative path to get PID file
PID_File_Path="$SCRIPT_DIR/Gcode-Send-PID.pid"
USB_Device="$(eureka_resolve_printer_device)" || USB_Device=""

# Check if Printer is connected
if [ -z "$USB_Device" ]; then
  echo "Printer not connected!"
  exit 1
fi
export EUREKA_PRINTER_DEVICE="$USB_Device"

if [ -f "$PID_File_Path" ]; then
    PID=$(cat "$PID_File_Path")

    if ps -p "$PID" > /dev/null ; then

        kill "$PID" > /dev/null
        echo "Print job terminated successfully!"
        stty -F "$USB_Device" 115200
        if [ -e "$USB_Device" ]; then

           echo "G21" > "$USB_Device"
           echo "M104 S0" > "$USB_Device"
           echo "M140 S0" > "$USB_Device"
           echo "G1 Z180 F1500" > "$USB_Device"
           echo "G1 Y220 F1500" > "$USB_Device"

        else
            echo "Error: Unable to access $USB_Device"
        fi
    else 
        echo "PID exists but process is not running: $PID"
    fi
else
    echo "PID file does not exist. Print job may not have started."
fi
