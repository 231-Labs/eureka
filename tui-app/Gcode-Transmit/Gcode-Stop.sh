#!/bin/bash

if [ -f ~/eureka/tui-app/Gcode-Transmit/Gcode-Send-PID.pid ]; then
    PID=$(cat ~/eureka/tui-app/Gcode-Transmit/Gcode-Send-PID.pid )

    if ps -p $PID > /dev/null ; then

        kill $PID > /dev/null
        echo "已成功終止列印!"
        stty -F /dev/3Dprinter 115200
        if [ -e /dev/3Dprinter ]; then

           echo "G21" > /dev/3Dprinter
           echo "M104 S0" > /dev/3Dprinter
           echo "M140 S0" > /dev/3Dprinter
           echo "G1 Z180 F1500" > /dev/3Dprinter
           echo "G1 Y220 F1500" > /dev/3Dprinter

        else
            echo "Error: /dev/3Dprinter 無法訪問"
        fi
    else echo "PID 存在但已不在運行中"$PID

    fi
else
    echo "PID 檔不存在，可能列印尚未啟動"
fi
