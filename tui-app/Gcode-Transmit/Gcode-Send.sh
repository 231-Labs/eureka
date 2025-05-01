#!/bin/bash


echo $$ > ~/eureka/tui-app/Gcode-Transmit/Gcode-Send-PID.pid
prusa-slicer --export-gcode --load Ender-3_set.ini --output test.gcode test.stl
echo $! > ~/eureka/tui-app/Gcode-Transmit/Gcode-Send-PID.pid
rm -rf test.stl
./serial
echo $! > ~/eureka/tui-app/Gcode-Transmit/Gcode-Send-PID.pid
rm test.gcode

