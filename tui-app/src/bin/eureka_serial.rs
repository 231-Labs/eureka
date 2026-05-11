//! Sends `test.gcode` (or path from argv[1]) to the printer over USB serial.
//! Honors `EUREKA_PRINTER_DEVICE`; otherwise tries /dev/3Dprinter, /dev/ttyACM0, /dev/ttyUSB0.
//! Optional: `EUREKA_PRINTER_BAUD` (default 115200), `EUREKA_LINE_DELAY_MS` (default 0).
//! After each non-comment line, waits for a Marlin-style `ok` (or `ok T:...`) before sending the next.
//! `EUREKA_OK_TIMEOUT_SEC` caps how long to wait per line (default 600; use for M109/M190).

use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::Path;
use std::time::{Duration, Instant};

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
        .unwrap_or(0)
}

fn ok_timeout_per_line() -> Duration {
    let sec: u64 = env::var("EUREKA_OK_TIMEOUT_SEC")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(600);
    Duration::from_secs(sec.max(1))
}

/// Marlin acknowledges each queued line with `ok` or `ok T:...`.
fn line_indicates_ok(line: &str) -> bool {
    let s = line.trim();
    let lower = s.to_ascii_lowercase();
    lower == "ok" || lower.starts_with("ok ")
}

fn wait_for_ok(port: &mut (impl Read + ?Sized), overall: Duration) -> anyhow::Result<()> {
    let deadline = Instant::now() + overall;
    let mut acc: Vec<u8> = Vec::new();
    let mut scratch = [0u8; 1024];

    while Instant::now() < deadline {
        let n = match port.read(&mut scratch) {
            Ok(n) => n,
            Err(e) if e.kind() == std::io::ErrorKind::TimedOut => continue,
            Err(e) => return Err(e.into()),
        };
        if n == 0 {
            continue;
        }
        acc.extend_from_slice(&scratch[..n]);

        while let Some(pos) = acc.iter().position(|&b| b == b'\n') {
            let raw: Vec<u8> = acc.drain(..=pos).collect();
            let line = String::from_utf8_lossy(&raw[..raw.len().saturating_sub(1)]);
            let t = line.trim_end_matches('\r');
            if t.is_empty() {
                continue;
            }
            let lower = t.to_ascii_lowercase();
            if lower.contains("resend") {
                anyhow::bail!("Printer requested resend; line: {t:?}");
            }
            if line_indicates_ok(t) {
                return Ok(());
            }
        }
    }

    anyhow::bail!("Timed out waiting for ok from printer (within {:?})", overall);
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

    let file = File::open(&gcode_path).map_err(|e| {
        let hint = if gcode_path.contains("path/to") {
            " Pass a real .gcode path (for example: Gcode-Transmit/diagnostics/<timestamp>/diagnostic-slice.gcode)."
        } else {
            ""
        };
        anyhow::anyhow!("Failed to open {}: {}{}", gcode_path, e, hint)
    })?;
    let delay = Duration::from_millis(line_delay_ms());
    let per_line_ok = ok_timeout_per_line();
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
        wait_for_ok(&mut *port, per_line_ok)?;
        if !delay.is_zero() {
            std::thread::sleep(delay);
        }
    }

    Ok(())
}
