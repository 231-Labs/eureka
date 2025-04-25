#!/bin/bash

chmod +x Slic3r-1.3.0-x86_64.AppImage
chmod +x serial

./Slic3r-1.3.0-x86_64.AppImage --no-gui --load Ender-3_set.ini --output test.gcode test.stl
./serial
