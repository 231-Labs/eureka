#!/bin/bash



prusa-slicer --export-gcode --load Ender-3_set.ini --output test.gcode test.stl
./serial

