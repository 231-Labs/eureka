# Eureka Print Diagnostics

Use `diagnose-print.py` when a printer heats up and then cools down without printing. The goal is to isolate the fault before changing STL files, slicer settings, Eureka, or printer firmware.

## Quick Run

```bash
cd tui-app/Gcode-Transmit
./run-diagnostics.sh
```

This runs the known-good 20mm cube through STL inspection, PrusaSlicer, G-code inspection, and writes a supervised control heat-hold G-code file.

For a custom STL:

```bash
cd tui-app/Gcode-Transmit
./diagnose-print.py --stl test.stl --slice --emit-control-gcode
```

The script writes a timestamped report under `tui-app/Gcode-Transmit/diagnostics/`.

If you already have the G-code that Eureka sent:

```bash
./diagnose-print.py --stl test.stl --gcode main/test.gcode
```

For a non-heating printer connectivity check:

```bash
./diagnose-print.py --printer-probe --device /dev/3Dprinter
```

## Reading The Result

1. STL issue: the report says the STL has no triangles, invalid dimensions, tiny dimensions, or impossible bounds.
2. Slicer issue: the report says there is no extrusion, the first cooldown command appears before extrusion, or `M109` / `M190` waits are missing.
3. Eureka issue: the sliced G-code looks healthy, but the same file only fails when sent by Eureka. Compare with SD card, Pronterface, or OctoPrint. The current serial sender is also inspected because a sender that writes without reading printer acknowledgements can overrun firmware buffers.
4. Printer issue: a known-good sender or SD card also heats and then cools, or the non-heating `M115/M105` probe cannot get stable printer responses.

## Safe Control Test

`--emit-control-gcode` writes `control_heat_hold_170c.gcode`. Run it only while supervised and preferably from a known-good sender or SD card. Expected behavior: heat nozzle to 170 C, hold for 30 seconds, then cool down. If this fails outside Eureka, investigate printer firmware, thermistor readings, power, or thermal runaway protection first.
