//! Sends `test.gcode` (or path from argv[1]) to the printer over USB serial.
//! Honors `EUREKA_PRINTER_DEVICE`; otherwise tries /dev/3Dprinter, /dev/ttyACM0, /dev/ttyUSB0.
//! Optional: `EUREKA_PRINTER_BAUD` (default 115200), `EUREKA_LINE_DELAY_MS` (default 5).
//!
//! After the last G-code line (default): drains the RX buffer, sends `M400`, then waits for an
//! `ok` response line so the host does not finish before motion stops. Set
//! `EUREKA_SKIP_PRINT_COMPLETION_WAIT=1` or `true` to skip (testing or firmware without `M400`).

use std::env;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::Path;
use std::time::Duration;

use serialport::SerialPort;

fn skip_print_completion_wait() -> bool {
    match env::var("EUREKA_SKIP_PRINT_COMPLETION_WAIT")
        .ok()
        .as_deref()
        .map(str::trim)
    {
        Some("1") | Some("true") | Some("TRUE") | Some("True") => true,
        _ => false,
    }
}

/// Drop stale RX data so a following `ok` is from `M400`, not an earlier command.
fn drain_receive_buffer(port: &mut dyn SerialPort) -> io::Result<()> {
    let prev = port.timeout();
    port.set_timeout(Duration::from_millis(25))?;
    let mut buf = [0u8; 512];
    loop {
        match port.read(&mut buf) {
            Ok(0) => break,
            Ok(_) => continue,
            Err(e) if e.kind() == io::ErrorKind::TimedOut => break,
            Err(e) => {
                let _ = port.set_timeout(prev);
                return Err(e);
            }
        }
    }
    port.set_timeout(prev)?;
    Ok(())
}

/// Wait for Marlin-style `ok` / `ok …` line after `M400` (long timeout: motion may be large).
fn wait_for_ok_line(port: &mut dyn SerialPort) -> io::Result<()> {
    port.set_timeout(Duration::from_secs(300))?;
    let mut reader = BufReader::new(port);
    let mut line = String::new();
    loop {
        line.clear();
        reader.read_line(&mut line)?;
        let t = line.trim();
        if t == "ok" || t.starts_with("ok ") {
            break;
        }
    }
    Ok(())
}

fn resolve_device() -> Option<String> {
    if let Ok(p) = env::var("EUREKA_PRINTER_DEVICE") {
        if Path::new(&p).exists() {
            return Some(p);
        }
    }
    for d in ["/dev/3Dprinter", "/dev/ttyACM0", "/dev/ttyUSB0"] {
        if Path::new(d).exists() {
            return Some(d.to_string());
        }
    }
    None
}

fn baud() -> u32 {
    env::var("EUREKA_PRINTER_BAUD")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(115_200)
}

fn line_delay_ms() -> u64 {
    env::var("EUREKA_LINE_DELAY_MS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(5)
}

fn main() -> anyhow::Result<()> {
    let gcode_path = env::args().nth(1).unwrap_or_else(|| "test.gcode".to_string());
    let port_name = resolve_device().ok_or_else(|| {
        anyhow::anyhow!(
            "No serial device (set EUREKA_PRINTER_DEVICE or use /dev/3Dprinter per README)"
        )
    })?;

    let baud = baud();
    let mut port = serialport::new(&port_name, baud)
        .timeout(Duration::from_millis(500))
        .open()
        .map_err(|e| anyhow::anyhow!("Failed to open {}: {}", port_name, e))?;

    let file = File::open(&gcode_path)
        .map_err(|e| anyhow::anyhow!("Failed to open {}: {}", gcode_path, e))?;
    let delay = Duration::from_millis(line_delay_ms());
    let reader = BufReader::new(file);

    for line in reader.lines() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with(';') {
            continue;
        }
        let mut out = trimmed.to_string();
        out.push('\n');
        port
            .write_all(out.as_bytes())
            .map_err(|e| anyhow::anyhow!("Write failed: {}", e))?;
        port
            .flush()
            .map_err(|e| anyhow::anyhow!("Flush failed: {}", e))?;
        std::thread::sleep(delay);
    }

    if !skip_print_completion_wait() {
        drain_receive_buffer(&mut *port)?;
        port
            .write_all(b"M400\n")
            .map_err(|e| anyhow::anyhow!("Write M400 failed: {}", e))?;
        port
            .flush()
            .map_err(|e| anyhow::anyhow!("Flush after M400 failed: {}", e))?;
        wait_for_ok_line(&mut *port).map_err(|e| {
            anyhow::anyhow!(
                "Waiting for ok after M400 failed: {} (set EUREKA_SKIP_PRINT_COMPLETION_WAIT=1 to skip)",
                e
            )
        })?;
    }

    Ok(())
}
