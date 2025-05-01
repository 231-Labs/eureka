#!/bin/bash



prusa-slicer --export-gcode --load Ender-3_set.ini --output test.gcode test.stl
rm -rf test.stl
./serial
rm test.gcode

