//! Sends `test.gcode` (or path from argv[1]) to the printer over USB serial.
//! Honors `EUREKA_PRINTER_DEVICE`; otherwise tries /dev/3Dprinter, /dev/ttyACM0, /dev/ttyUSB0.
//! Optional: `EUREKA_PRINTER_BAUD` (default 115200), `EUREKA_LINE_DELAY_MS` (default 5).

use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::time::Duration;

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

    Ok(())
}
