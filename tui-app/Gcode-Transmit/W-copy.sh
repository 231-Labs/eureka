#!/bin/bash

cp /home/ubuntu/eureka/tui-app/test1.stl /home/ubuntu/eureka/tui-app/test.stl > /dev/null 2>&1
mv /home/ubuntu/eureka/tui-app/test.stl /home/ubuntu/eureka/tui-app/Gcode-Transmit >/dev/null 2>&1

if [ $? -eq 0 ]; then
  echo "Success"
  echo -n "Success" > "/home/ubuntu/eureka/tui-app/Gcode-Transmit/Gcode-Status"
else
  echo "failed"
  echo -n "failed" > "/home/ubuntu/eureka/tui-app/Gcode-Transmit/Gcode-Status"
fi
