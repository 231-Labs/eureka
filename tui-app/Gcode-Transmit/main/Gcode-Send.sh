#!/bin/bash
# 獲取腳本所在的目錄
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# 獲取父目錄路徑
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

