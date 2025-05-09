#!/bin/bash

# PID_File_Path="~/eureka/tui-app/Gcode-Transmit/main/Gcode-Send-PID.pid"
PID_File_Path="./Gcode-Send-PID.pid"
USB_Device="/dev/3Dprinter"

# 檢查 Printer 是否連接
if [ ! -e "$USB_Device" ]; then
  echo "Printer not connected!"
  exit 1
fi

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
            echo "Error: Unable to access /dev/3Dprinter"
        fi
    else 
        echo "PID exists but process is not running: $PID"
    fi
else
    echo "PID file does not exist. Print job may not have started."
fi
