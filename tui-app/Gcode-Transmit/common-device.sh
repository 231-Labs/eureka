# Shared helpers for Eureka G-code transmit scripts (bash).
# shellcheck shell=bash

# Print first usable serial device path, or nothing.
eureka_resolve_printer_device() {
  if [ -n "${EUREKA_PRINTER_DEVICE:-}" ] && [ -e "${EUREKA_PRINTER_DEVICE}" ]; then
    printf '%s\n' "${EUREKA_PRINTER_DEVICE}"
    return 0
  fi
  for d in /dev/3Dprinter /dev/ttyACM0 /dev/ttyUSB0; do
    if [ -e "$d" ]; then
      printf '%s\n' "$d"
      return 0
    fi
  done
  # macOS USB serial (optional)
  local shopt_state=""
  shopt_state="$(shopt -p nullglob 2>/dev/null || true)"
  shopt -s nullglob 2>/dev/null || true
  for d in /dev/tty.usbmodem* /dev/tty.usbserial*; do
    if [ -e "$d" ]; then
      printf '%s\n' "$d"
      eval "$shopt_state" 2>/dev/null || true
      return 0
    fi
  done
  eval "$shopt_state" 2>/dev/null || true
  return 1
}

# Print path to eureka-serial if executable, else exit 1 from this helper's perspective.
eureka_find_serial_bin() {
  local here="$1"
  local root
  root="$(cd "$here/../.." && pwd)"
  local cand
  for cand in \
    "$here/eureka-serial" \
    "$root/target/release/eureka-serial" \
    "$root/target/debug/eureka-serial"; do
    if [ -x "$cand" ]; then
      printf '%s\n' "$cand"
      return 0
    fi
  done
  return 1
}
