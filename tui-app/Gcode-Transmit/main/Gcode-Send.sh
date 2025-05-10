#!/bin/bash
cd "$(dirname "$0")"
prusa-slicer --export-gcode --load Ender-3_set.ini --output test.gcode ../test.stl &
echo $! > /home/ubuntu/eureka/tui-app/Gcode-Transmit/main/Gcode-Send-PID.pid
# echo $! > Gcode-Send-PID.pid
wait $!
rm -rf test.stl
./serial &
echo $! > /home/ubuntu/eureka/tui-app/Gcode-Transmit/main/Gcode-Send-PID.pid
# echo $! > Gcode-Send-PID.pid
wait $!
rm -rf test.gcode

