import serial
import time
import datetime
import curses
import re
import os
import json
from threading import Thread, Lock
from collections import deque

# Config
CONFIG_FILE = "config.json"
DEFAULT_CONFIG = {
    "serial_port": "auto",
    "hardware_offset_ms": 0
}

# Globals
SERIAL_PORT = "/dev/ttyUSB0"
BAUD_RATE = 115200
FRAME_RATE = 25.0
hardware_offset_ms = 0

# Shared data
latest_line = None
latest_timestamp = None
line_lock = Lock()

# Sync Jitter buffer
offset_buffer = deque(maxlen=50)

# Load config
if os.path.exists(CONFIG_FILE):
    with open(CONFIG_FILE) as f:
        try:
            config = json.load(f)
            SERIAL_PORT = config.get("serial_port", SERIAL_PORT)
            hardware_offset_ms = config.get("hardware_offset_ms", 0)
        except json.JSONDecodeError:
            print("⚠️ Failed to parse config.json. Using defaults.")
else:
    config = DEFAULT_CONFIG

def auto_detect_serial_port():
    for dev in os.listdir("/dev"):
        if dev.startswith("ttyACM") or dev.startswith("ttyUSB"):
            return f"/dev/{dev}"
    return SERIAL_PORT

def parse_ltc_line(line):
    match = re.match(r"\[(LOCK|FREE)\]\s+(\d{2}):(\d{2}):(\d{2})[:;](\d{2})\s+\|\s+([\d.]+)fps", line)
    if not match:
        return None
    return {
        "lock": match.group(1),
        "hours": int(match.group(2)),
        "minutes": int(match.group(3)),
        "seconds": int(match.group(4)),
        "frames": int(match.group(5)),
        "fps": float(match.group(6))
    }

def serial_reader(port, baud):
    global latest_line, latest_timestamp
    ser = serial.Serial(port, baud, timeout=1)
    while True:
        line = ser.readline().decode(errors='ignore').strip()
        if line:
            with line_lock:
                latest_line = line
                latest_timestamp = datetime.datetime.now()

def run_curses(stdscr):
    curses.curs_set(0)
    stdscr.nodelay(True)
    stdscr.timeout(100)

    serial_port = auto_detect_serial_port()
    stdscr.addstr(1, 2, f"NTP Timeturner v1.3")
    stdscr.addstr(2, 2, f"Using Serial Port: {serial_port}")
    stdscr.refresh()

    reader_thread = Thread(target=serial_reader, args=(serial_port, BAUD_RATE), daemon=True)
    reader_thread.start()

    lock_count = 0
    free_count = 0
    sync_allowed = False

    while True:
        now = datetime.datetime.now()
        with line_lock:
            line = latest_line
            timestamp = latest_timestamp

        parsed = parse_ltc_line(line) if line else None

        stdscr.erase()
        stdscr.addstr(1, 2, f"NTP Timeturner v1.3")
        stdscr.addstr(2, 2, f"Using Serial Port: {serial_port}")

        if parsed:
            lock_state = parsed["lock"]
            if lock_state == "LOCK":
                lock_count += 1
            else:
                free_count += 1

            stdscr.addstr(4, 2, f"LTC Status   : {lock_state}", curses.color_pair(2 if lock_state == "LOCK" else 3))

            timecode_str = f"{parsed['hours']:02}:{parsed['minutes']:02}:{parsed['seconds']:02}:{parsed['frames']:02}"
            stdscr.addstr(5, 2, f"LTC Timecode : {timecode_str}")
            stdscr.addstr(6, 2, f"Frame Rate   : {parsed['fps']:.2f}fps")

            # Generate LTC datetime object
            ltc_time = datetime.datetime.now().replace(
                hour=parsed["hours"],
                minute=parsed["minutes"],
                second=parsed["seconds"],
                microsecond=int((parsed["frames"] / parsed["fps"]) * 1_000_000)
            )

            # Show system clock
            system_time = datetime.datetime.now()
            stdscr.addstr(7, 2, f"System Clock : {system_time.strftime('%H:%M:%S.%f')[:-3]}")

            # Calculate Sync Jitter
            if lock_state == "LOCK":
                offset_ms = (system_time - ltc_time).total_seconds() * 1000 - hardware_offset_ms
                offset_buffer.append(offset_ms)
                if offset_buffer:
                    avg_offset = sum(offset_buffer) / len(offset_buffer)
                    frame_error = round((avg_offset / 1000) * parsed["fps"])
                    stdscr.addstr(8, 2, f"Sync Jitter  : {avg_offset:+.0f} ms ({frame_error:+} frames)",
                                  curses.color_pair(0 if abs(avg_offset) < 5 else 3))
                else:
                    stdscr.addstr(8, 2, f"Sync Jitter  : --")
            else:
                stdscr.addstr(8, 2, f"Sync Jitter  : -- (FREE mode)", curses.color_pair(3))

            # Timecode Match (HH:MM:SS only)
            ltc_time_str = ltc_time.strftime('%H:%M:%S')
            sys_time_str = system_time.strftime('%H:%M:%S')
            if lock_state == "LOCK":
                if ltc_time_str == sys_time_str:
                    stdscr.addstr(9, 2, "Timecode Match: MATCHED", curses.color_pair(2))
                else:
                    stdscr.addstr(9, 2, "Timecode Match: OUT OF SYNC", curses.color_pair(3))
            else:
                stdscr.addstr(9, 2, "Timecode Match: UNKNOWN", curses.color_pair(3))

            sync_allowed = lock_state == "LOCK"

            # Lock Ratio
            total = lock_count + free_count
            if total > 0:
                lock_ratio = (lock_count / total) * 100
                stdscr.addstr(10, 2, f"Lock Ratio   : {lock_ratio:.1f}% LOCK", curses.color_pair(2 if lock_ratio > 90 else 3))
        else:
            stdscr.addstr(4, 2, "Waiting for LTC data...", curses.color_pair(3))

        stdscr.addstr(12, 2, "[S] Set system clock to LTC    [Ctrl+C] Quit")

        key = stdscr.getch()
        if key in (ord('s'), ord('S')) and parsed and sync_allowed:
            clock_str = f"{parsed['hours']:02}:{parsed['minutes']:02}:{parsed['seconds']:02}"
            os.system(f"sudo date -s \"{clock_str}\"")

        stdscr.refresh()
        time.sleep(0.05)

def main():
    curses.wrapper(start_curses)

def start_curses(stdscr):
    curses.start_color()
    curses.init_pair(1, curses.COLOR_WHITE, curses.COLOR_BLACK)
    curses.init_pair(2, curses.COLOR_GREEN, curses.COLOR_BLACK)
    curses.init_pair(3, curses.COLOR_RED, curses.COLOR_BLACK)
    run_curses(stdscr)

if __name__ == "__main__":
    main()
