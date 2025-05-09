#!/bin/bash

Options_control="$1"

if [ "$Options_control" == "--print" ]; then

  USB_Device="/dev/3Dprinter"
  if [ ! -e "$USB_Device" ]; then
    echo "Printer not connected!"
    exit 1
  fi
  # /home/ubuntu/eureka/tui-app/Gcode-Transmit/main/Gcode-Send.sh
  ./main/Gcode-Send.sh

elif [ "$Options_control" == "--stop" ]; then
  # /home/ubuntu/eureka/tui-app/Gcode-Transmit/main/Gcode-Stop.sh
  ./main/Gcode-Stop.sh

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
