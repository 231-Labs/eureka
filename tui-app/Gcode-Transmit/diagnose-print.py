#!/usr/bin/env python3
"""Diagnose STL -> slicer -> Eureka sender -> printer handoff issues.

The default run is read-only and does not talk to the printer. Use
--printer-probe for a non-heating M115/M105 serial probe.
"""

from __future__ import annotations

import argparse
import datetime as dt
import math
import os
import re
import select
import shutil
import struct
import subprocess
import sys
import termios
import time
from dataclasses import dataclass
from pathlib import Path


SCRIPT_DIR = Path(__file__).resolve().parent
TUI_ROOT = SCRIPT_DIR.parent
DEFAULT_CONFIG = SCRIPT_DIR / "main" / "Ender-3_set.ini"
DEFAULT_STL = SCRIPT_DIR / "test.stl"
SERIAL_SOURCE = TUI_ROOT / "src" / "bin" / "eureka_serial.rs"


@dataclass
class Finding:
    level: str
    area: str
    message: str


def add_find(findings: list[Finding], level: str, area: str, message: str) -> None:
    findings.append(Finding(level, area, message))


def strip_gcode_comment(line: str) -> str:
    return line.split(";", 1)[0].strip()


def parse_params(text: str) -> dict[str, float]:
    params: dict[str, float] = {}
    for key, value in re.findall(r"([A-Z])\s*(-?(?:\d+(?:\.\d*)?|\.\d+))", text.upper()):
        try:
            params[key] = float(value)
        except ValueError:
            pass
    return params


@dataclass
class GcodeCommand:
    line_no: int
    cmd: str
    params: dict[str, float]
    raw: str


def read_text(path: Path) -> str:
    return path.read_text(encoding="utf-8", errors="replace")


def parse_gcode(path: Path) -> list[GcodeCommand]:
    commands: list[GcodeCommand] = []
    for line_no, raw in enumerate(read_text(path).splitlines(), start=1):
        clean = strip_gcode_comment(raw)
        if not clean:
            continue
        match = re.match(r"^([GMT]\s*\d+)\b(.*)$", clean.upper())
        if not match:
            continue
        cmd = match.group(1).replace(" ", "")
        commands.append(GcodeCommand(line_no, cmd, parse_params(match.group(2)), raw.rstrip()))
    return commands


def analyze_gcode(path: Path, findings: list[Finding]) -> dict[str, object]:
    commands = parse_gcode(path)
    heat_cmds: list[GcodeCommand] = []
    cooldown_cmds: list[GcodeCommand] = []
    wait_cmds: list[GcodeCommand] = []
    end_cmds: list[GcodeCommand] = []
    extrusion_lines: list[int] = []
    travel_lines: list[int] = []
    mode_absolute_e = True
    last_e = 0.0

    for c in commands:
        if c.cmd in {"M104", "M109", "M140", "M190"}:
            heat_cmds.append(c)
            if c.cmd in {"M109", "M190"}:
                wait_cmds.append(c)
            if c.params.get("S", 9999.0) <= 0:
                cooldown_cmds.append(c)
        if c.cmd in {"M84", "M2", "M30"}:
            end_cmds.append(c)
        if c.cmd == "M82":
            mode_absolute_e = True
        elif c.cmd == "M83":
            mode_absolute_e = False
        elif c.cmd == "G92" and "E" in c.params:
            last_e = c.params["E"]
        elif c.cmd in {"G0", "G1"}:
            if "X" in c.params or "Y" in c.params or "Z" in c.params:
                travel_lines.append(c.line_no)
            if "E" in c.params:
                e = c.params["E"]
                extrudes = e > 0.0001 if not mode_absolute_e else e > last_e + 0.0001
                if extrudes:
                    extrusion_lines.append(c.line_no)
                if mode_absolute_e:
                    last_e = e

    first_cooldown = cooldown_cmds[0].line_no if cooldown_cmds else None
    first_extrusion = extrusion_lines[0] if extrusion_lines else None
    first_end = end_cmds[0].line_no if end_cmds else None

    if not commands:
        add_find(findings, "FAIL", "G-code", f"{path} has no parsed G/M/T commands.")
    if not heat_cmds:
        add_find(findings, "WARN", "Slicer", "No nozzle/bed temperature commands found.")
    if not any(c.cmd == "M109" for c in wait_cmds):
        add_find(findings, "WARN", "Slicer", "No M109 nozzle wait command found before printing.")
    if not any(c.cmd == "M190" for c in wait_cmds):
        add_find(findings, "WARN", "Slicer", "No M190 bed wait command found before printing.")
    if not extrusion_lines:
        add_find(findings, "FAIL", "STL/Slicer", "No extrusion moves were found; the sliced file may be empty or non-printable.")
    if first_cooldown is not None and first_extrusion is not None and first_cooldown < first_extrusion:
        add_find(
            findings,
            "FAIL",
            "Slicer",
            f"First cooldown command is line {first_cooldown}, before first extrusion line {first_extrusion}.",
        )
    if first_cooldown is not None and len(commands) > 0:
        cooldown_index = next(i for i, c in enumerate(commands, start=1) if c.line_no == first_cooldown)
        if cooldown_index < 25:
            add_find(
                findings,
                "WARN",
                "Slicer",
                f"Cooldown appears within the first {cooldown_index} commands; confirm this is not a tiny or truncated G-code file.",
            )
    if first_end is not None and first_extrusion is not None and first_end < first_extrusion:
        add_find(findings, "FAIL", "Slicer", f"End command line {first_end} appears before first extrusion line {first_extrusion}.")
    if first_extrusion and first_cooldown and first_extrusion < first_cooldown:
        add_find(
            findings,
            "PASS",
            "G-code",
            f"Print body exists: first extrusion line {first_extrusion}, first cooldown line {first_cooldown}.",
        )

    return {
        "path": str(path),
        "size_bytes": path.stat().st_size,
        "commands": len(commands),
        "heat_commands": [(c.line_no, c.cmd, c.params) for c in heat_cmds[:12]],
        "wait_commands": [(c.line_no, c.cmd, c.params) for c in wait_cmds[:12]],
        "cooldown_commands": [(c.line_no, c.cmd, c.params) for c in cooldown_cmds[:12]],
        "first_extrusion_line": first_extrusion,
        "extrusion_move_count": len(extrusion_lines),
        "travel_move_count": len(travel_lines),
        "first_end_line": first_end,
    }


def analyze_stl(path: Path, findings: list[Finding]) -> dict[str, object]:
    data = path.read_bytes()
    result: dict[str, object] = {"path": str(path), "size_bytes": len(data)}
    if len(data) < 15:
        add_find(findings, "FAIL", "STL", "STL file is too small to contain printable geometry.")
        return result

    is_binary = False
    tri_count = 0
    bounds = [[math.inf, -math.inf], [math.inf, -math.inf], [math.inf, -math.inf]]
    degenerate = 0

    if len(data) >= 84:
        binary_count = struct.unpack_from("<I", data, 80)[0]
        expected_size = 84 + binary_count * 50
        if expected_size == len(data):
            is_binary = True
            tri_count = binary_count
            for i in range(binary_count):
                offset = 84 + i * 50 + 12
                coords = struct.unpack_from("<9f", data, offset)
                p1 = coords[0:3]
                p2 = coords[3:6]
                p3 = coords[6:9]
                for p in (p1, p2, p3):
                    for axis, value in enumerate(p):
                        bounds[axis][0] = min(bounds[axis][0], value)
                        bounds[axis][1] = max(bounds[axis][1], value)
                if p1 == p2 or p2 == p3 or p1 == p3:
                    degenerate += 1

    if not is_binary:
        text = data.decode("utf-8", errors="ignore")
        vertices: list[tuple[float, float, float]] = []
        for match in re.finditer(
            r"vertex\s+(-?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][-+]?\d+)?)\s+"
            r"(-?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][-+]?\d+)?)\s+"
            r"(-?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][-+]?\d+)?)",
            text,
            re.IGNORECASE,
        ):
            p = tuple(float(v) for v in match.groups())
            vertices.append(p)
            for axis, value in enumerate(p):
                bounds[axis][0] = min(bounds[axis][0], value)
                bounds[axis][1] = max(bounds[axis][1], value)
        tri_count = len(vertices) // 3
        for i in range(0, len(vertices) - 2, 3):
            if vertices[i] == vertices[i + 1] or vertices[i + 1] == vertices[i + 2] or vertices[i] == vertices[i + 2]:
                degenerate += 1

    result.update(
        {
            "format": "binary" if is_binary else "ascii",
            "triangles": tri_count,
            "bounds_xyz": bounds if tri_count else None,
            "dimensions_xyz": [round(b[1] - b[0], 4) for b in bounds] if tri_count else None,
            "degenerate_triangle_estimate": degenerate,
        }
    )

    if tri_count == 0:
        add_find(findings, "FAIL", "STL", "No triangles were found in the STL.")
    elif degenerate / max(tri_count, 1) > 0.5:
        add_find(findings, "WARN", "STL", "More than half of sampled triangles look degenerate.")
    if tri_count:
        dims = result["dimensions_xyz"]
        assert isinstance(dims, list)
        if any(d <= 0 for d in dims):
            add_find(findings, "FAIL", "STL", f"Invalid STL bounds/dimensions: {dims}.")
        elif max(dims) < 0.5:
            add_find(findings, "WARN", "STL", f"Model dimensions are tiny ({dims}); units may be wrong.")
        elif max(dims) > 300:
            add_find(findings, "WARN", "STL", f"Model dimensions are large ({dims}); confirm it fits the printer profile.")
    return result


def run_slicer(stl: Path, config: Path, output: Path, findings: list[Finding]) -> bool:
    slicer = shutil.which("prusa-slicer")
    if not slicer:
        add_find(findings, "FAIL", "Slicer", "prusa-slicer is not in PATH; cannot run a fresh slice.")
        return False
    if not config.exists():
        add_find(findings, "FAIL", "Slicer", f"Config file missing: {config}")
        return False
    cmd = [slicer, "--export-gcode", "--load", str(config), "--output", str(output), str(stl)]
    completed = subprocess.run(cmd, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, check=False)
    log_path = output.with_suffix(".slicer.log")
    log_path.write_text(completed.stdout, encoding="utf-8")
    if completed.returncode != 0:
        add_find(findings, "FAIL", "Slicer", f"PrusaSlicer failed with exit code {completed.returncode}; see {log_path}.")
        return False
    if not output.exists() or output.stat().st_size == 0:
        add_find(findings, "FAIL", "Slicer", "PrusaSlicer finished but did not produce a non-empty G-code file.")
        return False
    add_find(findings, "PASS", "Slicer", f"Fresh G-code was generated at {output}.")
    return True


def inspect_eureka_sender(findings: list[Finding]) -> dict[str, object]:
    result: dict[str, object] = {"path": str(SERIAL_SOURCE), "exists": SERIAL_SOURCE.exists()}
    if not SERIAL_SOURCE.exists():
        add_find(findings, "WARN", "Eureka", f"Could not inspect sender source: {SERIAL_SOURCE}")
        return result
    src = read_text(SERIAL_SOURCE)
    has_write = ".write_all" in src
    has_read = ".read" in src or "read_line" in src or ".bytes()" in src
    waits_for_ok = re.search(r"""["']ok["']""", src, re.IGNORECASE) is not None
    result.update({"writes_serial": has_write, "reads_serial": has_read, "has_ok_ack_literal": waits_for_ok})
    if has_write and not has_read:
        add_find(
            findings,
            "WARN",
            "Eureka",
            "eureka-serial writes G-code lines but does not read printer responses; compare with SD card/Pronterface/OctoPrint before blaming the STL.",
        )
    elif has_write and has_read and waits_for_ok:
        add_find(findings, "PASS", "Eureka", "Serial sender appears to read printer acknowledgements.")
    return result


def baud_constant(baud: int) -> int:
    name = f"B{baud}"
    if not hasattr(termios, name):
        raise ValueError(f"Unsupported baud for termios on this host: {baud}")
    return getattr(termios, name)


def probe_printer(device: Path, baud: int, timeout: float, findings: list[Finding]) -> dict[str, object]:
    result: dict[str, object] = {"device": str(device), "baud": baud}
    if not device.exists():
        add_find(findings, "FAIL", "Printer", f"Serial device does not exist: {device}")
        return result
    fd = None
    try:
        fd = os.open(str(device), os.O_RDWR | os.O_NOCTTY | os.O_NONBLOCK)
        attrs = termios.tcgetattr(fd)
        speed = baud_constant(baud)
        attrs[0] = 0
        attrs[1] = 0
        attrs[2] = termios.CS8 | termios.CREAD | termios.CLOCAL
        attrs[3] = 0
        attrs[4] = speed
        attrs[5] = speed
        termios.tcsetattr(fd, termios.TCSANOW, attrs)
        time.sleep(2.0)
        os.write(fd, b"\nM115\nM105\n")
        deadline = time.time() + timeout
        chunks: list[bytes] = []
        while time.time() < deadline:
            readable, _, _ = select.select([fd], [], [], 0.2)
            if fd in readable:
                try:
                    chunk = os.read(fd, 4096)
                except BlockingIOError:
                    chunk = b""
                if chunk:
                    chunks.append(chunk)
        response = b"".join(chunks).decode("utf-8", errors="replace")
        result["response"] = response.strip()
        if "T:" in response or "FIRMWARE_NAME" in response or "ok" in response.lower():
            add_find(findings, "PASS", "Printer", "Printer responded to non-heating M115/M105 probe.")
        else:
            add_find(findings, "WARN", "Printer", "No recognizable response to M115/M105; check device path, baud, cable, permissions, or firmware.")
    except Exception as exc:  # noqa: BLE001 - diagnostics should report host-specific serial failures.
        result["error"] = str(exc)
        add_find(findings, "FAIL", "Printer", f"Serial probe failed: {exc}")
    finally:
        if fd is not None:
            os.close(fd)
    return result


def emit_control_gcode(report_dir: Path) -> Path:
    path = report_dir / "control_heat_hold_170c.gcode"
    path.write_text(
        "\n".join(
            [
                "; Eureka diagnostic control file.",
                "; Use only from a known-good sender or SD card, supervised.",
                "; Expected behavior: heat nozzle to 170 C, hold 30 seconds, then cool down.",
                "M140 S0",
                "M104 S170",
                "M109 S170",
                "M105",
                "G4 S30",
                "M104 S0",
                "M84",
                "",
            ]
        ),
        encoding="utf-8",
    )
    return path


def write_report(report_path: Path, sections: list[tuple[str, object]], findings: list[Finding]) -> None:
    lines: list[str] = []
    lines.append("# Eureka Print Diagnostic Report")
    lines.append("")
    lines.append(f"Generated: {dt.datetime.now().isoformat(timespec='seconds')}")
    lines.append("")
    lines.append("## Findings")
    if findings:
        for f in findings:
            lines.append(f"- [{f.level}] {f.area}: {f.message}")
    else:
        lines.append("- [INFO] No findings were produced.")
    lines.append("")
    lines.append("## How To Isolate The Fault")
    lines.extend(
        [
            "1. STL layer: FAIL/WARN in STL means the model may be empty, tiny, huge, or malformed before slicing.",
            "2. Slicer layer: cooldown before extrusion, missing extrusion, or missing M109/M190 points to the slicer profile or sliced G-code.",
            "3. Eureka layer: if G-code is healthy but the printer cools only when sent by Eureka, compare with SD card/Pronterface/OctoPrint; the sender may need acknowledgement-aware streaming.",
            "4. Printer layer: if the same control G-code cools or aborts from a known-good sender, inspect printer firmware, thermal runaway settings, sensors, power, or host disconnects.",
        ]
    )
    lines.append("")
    for title, payload in sections:
        lines.append(f"## {title}")
        if isinstance(payload, dict):
            for key, value in payload.items():
                if key == "response" and value:
                    lines.append(f"- {key}:")
                    lines.append("```")
                    lines.append(str(value))
                    lines.append("```")
                else:
                    lines.append(f"- {key}: {value}")
        else:
            lines.append(str(payload))
        lines.append("")
    report_path.write_text("\n".join(lines), encoding="utf-8")


def resolve_default_device() -> Path | None:
    env_device = os.environ.get("EUREKA_PRINTER_DEVICE")
    candidates = [env_device] if env_device else []
    candidates.extend(["/dev/3Dprinter", "/dev/ttyACM0", "/dev/ttyUSB0"])
    candidates.extend(str(p) for p in sorted(Path("/dev").glob("tty.usbmodem*")))
    candidates.extend(str(p) for p in sorted(Path("/dev").glob("tty.usbserial*")))
    for candidate in candidates:
        if candidate and Path(candidate).exists():
            return Path(candidate)
    return None


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Diagnose whether heat-then-cool print failures come from STL, slicer, Eureka, or printer.",
        formatter_class=argparse.ArgumentDefaultsHelpFormatter,
    )
    parser.add_argument("--stl", type=Path, help=f"STL to inspect and optionally slice. --slice defaults to {DEFAULT_STL}.")
    parser.add_argument("--gcode", type=Path, help="Existing G-code to inspect.")
    parser.add_argument("--slice", action="store_true", help="Run PrusaSlicer with the Eureka Ender-3 profile and inspect the result.")
    parser.add_argument("--config", type=Path, default=DEFAULT_CONFIG, help="PrusaSlicer config used for --slice.")
    parser.add_argument("--report-dir", type=Path, help="Directory for generated report and sliced G-code.")
    parser.add_argument("--printer-probe", action="store_true", help="Open serial port and send only M115/M105. This does not heat the printer.")
    parser.add_argument("--device", type=Path, help="Printer serial device for --printer-probe.")
    parser.add_argument("--baud", type=int, default=int(os.environ.get("EUREKA_PRINTER_BAUD", "115200")), help="Serial baud rate.")
    parser.add_argument("--probe-timeout", type=float, default=5.0, help="Seconds to wait for printer probe response.")
    parser.add_argument("--emit-control-gcode", action="store_true", help="Write a supervised 170 C heat-hold-cool control G-code file.")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if args.slice and args.stl is None:
        args.stl = DEFAULT_STL
    stamp = dt.datetime.now().strftime("%Y%m%d-%H%M%S")
    report_dir = args.report_dir or (SCRIPT_DIR / "diagnostics" / stamp)
    report_dir.mkdir(parents=True, exist_ok=True)
    findings: list[Finding] = []
    sections: list[tuple[str, object]] = []

    if args.stl and args.stl.exists():
        sections.append(("STL", analyze_stl(args.stl, findings)))
    elif args.stl:
        add_find(findings, "WARN", "STL", f"STL not found: {args.stl}")

    gcode_paths: list[Path] = []
    if args.gcode:
        if args.gcode.exists():
            gcode_paths.append(args.gcode)
        else:
            add_find(findings, "FAIL", "G-code", f"G-code not found: {args.gcode}")

    if args.slice:
        sliced = report_dir / "diagnostic-slice.gcode"
        if args.stl is None:
            add_find(findings, "FAIL", "Slicer", "Cannot slice because no STL was provided.")
        elif not args.stl.exists():
            add_find(findings, "FAIL", "Slicer", f"Cannot slice because STL is missing: {args.stl}")
        elif run_slicer(args.stl, args.config, sliced, findings):
            gcode_paths.append(sliced)

    for gcode in gcode_paths:
        sections.append((f"G-code: {gcode.name}", analyze_gcode(gcode, findings)))

    sections.append(("Eureka Sender", inspect_eureka_sender(findings)))

    if args.printer_probe:
        device = args.device or resolve_default_device()
        if device is None:
            add_find(findings, "FAIL", "Printer", "No serial device found; set --device or EUREKA_PRINTER_DEVICE.")
        else:
            sections.append(("Printer Probe", probe_printer(device, args.baud, args.probe_timeout, findings)))

    if args.emit_control_gcode:
        path = emit_control_gcode(report_dir)
        add_find(findings, "INFO", "Printer", f"Control heat-hold G-code written to {path}.")

    report_path = report_dir / "report.md"
    write_report(report_path, sections, findings)

    print(f"Diagnostic report: {report_path}")
    for finding in findings:
        print(f"[{finding.level}] {finding.area}: {finding.message}")
    return 1 if any(f.level == "FAIL" for f in findings) else 0


if __name__ == "__main__":
    sys.exit(main())
