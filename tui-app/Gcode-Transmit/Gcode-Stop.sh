#!/bin/bash

if [ -f ~/eureka/tui-app/Gcode-Transmit/Gcode-Send-PID.pid ]; then
    PID=$(cat /tmp/my_script.pid)
    if ps -p $PID > /dev/null; then
        kill $PID
        echo "已成功終止列印，PID: $PID"
    else
        echo "PID 存在但已不在運行中"
    fi
else
    echo "PID 檔不存在，可能列印尚未啟動"
fi
