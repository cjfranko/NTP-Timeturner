import curses
import serial
import re
from datetime import datetime
import time

# Configurable parameters
SERIAL_PORT = "/dev/ttyACM0"
BAUD_RATE = 115200
REFRESH_INTERVAL = 0.5  # seconds

# Regex to match LTC lines
ltc_pattern = re.compile(
    r"\[(LOCK|FREE)\]\s+(\d{2}:\d{2}:\d{2}[:;]\d{2})\s+\|\s+([\d.]+fps)", re.IGNORECASE
)

def read_ltc(ser):
    """Reads and parses one line of LTC from the serial interface"""
    try:
        line = ser.readline().decode(errors='ignore').strip()
        match = ltc_pattern.match(line)
        if match:
            status, timecode, framerate = match.groups()
            return f"{status} {timecode} ({framerate.upper()})"
        return None
    except:
        return None

def draw_ui(stdscr):
    curses.curs_set(0)  # Hide cursor
    stdscr.nodelay(True)
    stdscr.timeout(int(REFRESH_INTERVAL * 1000))

    # Open serial connection
    try:
        ser = serial.Serial(SERIAL_PORT, BAUD_RATE, timeout=1)
    except serial.SerialException as e:
        stdscr.addstr(0, 0, f"[ERROR] Failed to open serial: {e}")
        stdscr.getch()
        return

    ltc_string = "Waiting for LTC…"

    while True:
        try:
            stdscr.clear()

            # Read LTC if available
            new_ltc = read_ltc(ser)
            if new_ltc:
                ltc_string = new_ltc

            # Get system time
            now = datetime.now().strftime("%Y-%m-%d %H:%M:%S")

            # Draw UI
            stdscr.addstr(0, 0, "🕰️  NTP Timeturner v0.1")
            stdscr.addstr(2, 0, f"LTC Timecode:   {ltc_string}")
            stdscr.addstr(3, 0, f"System Clock:   {now}")
            stdscr.addstr(5, 0, "Press Ctrl+C to quit.")

            stdscr.refresh()
            time.sleep(REFRESH_INTERVAL)

        except KeyboardInterrupt:
            break
        except Exception as e:
            stdscr.addstr(7, 0, f"[EXCEPTION] {e}")
            stdscr.refresh()
            time.sleep(1)

    ser.close()

if __name__ == "__main__":
    curses.wrapper(draw_ui)
