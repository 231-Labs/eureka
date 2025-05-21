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

  echo "用法：Gcode-Process.sh [選項]"
  echo ""
  echo "可用選項："
  echo "  --print    傳送 G-code 指令"
  echo "  --stop     停止 G-code 傳送"
  echo "  --help     顯示說明"

else

  echo "$Options_control is not Gcode-Process comment pless use '--help'"

fi
