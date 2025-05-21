#!/bin/bash
# Get the directory of the script
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Get the parent directory path
PARENT_DIR="$(dirname "$SCRIPT_DIR")"

prusa-slicer --export-gcode --load Ender-3_set.ini --output test.gcode "$PARENT_DIR/test.stl" &
echo $! > "$SCRIPT_DIR/Gcode-Send-PID.pid"
# echo $! > Gcode-Send-PID.pid
wait $!
rm -rf "$PARENT_DIR/test.stl"
./serial &
echo $! > "$SCRIPT_DIR/Gcode-Send-PID.pid"
# echo $! > Gcode-Send-PID.pid
wait $!
rm -rf test.gcode

