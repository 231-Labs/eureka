#!/bin/bash

if [ -f ~/eureka/tui-app/Gcode-Transmit/Gcode-Send-PID.pid ]; then
    PID=$(cat ~/eureka/tui-app/Gcode-Transmit/Gcode-Send-PID.pid )

    if ps -p $PID > /dev/null ; then

        kill $PID > /dev/null
        echo "Print job terminated successfully!"
        stty -F /dev/3Dprinter 115200
        if [ -e /dev/3Dprinter ]; then

           echo "G21" > /dev/3Dprinter
           echo "M104 S0" > /dev/3Dprinter
           echo "M140 S0" > /dev/3Dprinter
           echo "G1 Z180 F1500" > /dev/3Dprinter
           echo "G1 Y220 F1500" > /dev/3Dprinter

        else
            echo "Error: Unable to access /dev/3Dprinter"
        fi
    else echo "PID exists but process is not running: "$PID

    fi
else
    echo "PID file does not exist. Print job may not have started."
fi
