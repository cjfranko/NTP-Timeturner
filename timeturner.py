# -*- coding: utf-8 -*-
import curses
import serial
import re
import time
from datetime import datetime

# Serial config
SERIAL_PORT = "/dev/ttyACM0"
BAUD_RATE = 115200
UI_REFRESH_INTERVAL = 0.25  # seconds
SIGNAL_TIMEOUT = 1.5  # seconds

# Regex pattern
ltc_pattern = re.compile(
    r"\[(LOCK|FREE)\]\s+(\d{2}:\d{2}:\d{2}[:;]\d{2})\s+\|\s+([\d.]+fps)", re.IGNORECASE
)

# Shared state
state = {
    "ltc_status": "--",
    "ltc_timecode": "--:--:--:--",
    "framerate": "--",
    "system_clock": "--:--:--.---",
    "offset_str": "--",
    "lock_count": 0,
    "free_count": 0,
    "last_received": None,
    "signal_loss": False
}

def parse_timecode(tc_str):
    h, m, s, f = map(int, tc_str.replace(";", ":").split(":"))
    return h, m, s, f

def timecode_to_milliseconds(h, m, s, f, fps):
    return int(((h * 3600 + m * 60 + s) * 1000) + (f * (1000 / fps)))

def get_offset(system_dt, h, m, s, f, fps):
    sys_ms = (system_dt.hour * 3600 + system_dt.minute * 60 + system_dt.second) * 1000 + system_dt.microsecond // 1000
    ltc_ms = timecode_to_milliseconds(h, m, s, f, fps)
    return sys_ms - ltc_ms

def format_offset(ms, fps):
    frame_duration = 1000 / fps
    frame_offset = int(round(ms / frame_duration))
    return f"{ms:+} ms ({frame_offset:+} frames)"

def serial_reader(ser):
    global state
    while ser.in_waiting:
        line = ser.readline().decode(errors='ignore').strip()
        match = ltc_pattern.match(line)
        now = datetime.now()

        if match:
            status, tc_str, fps_str = match.groups()
            fps = float(fps_str.lower().replace("fps", ""))
            h, m, s, f = parse_timecode(tc_str)

            # Update shared state
            state["ltc_status"] = status
            state["ltc_timecode"] = tc_str
            state["framerate"] = fps_str
            state["system_clock"] = now.strftime("%H:%M:%S.%f")[:-3]
            state["offset_str"] = format_offset(get_offset(now, h, m, s, f, fps), fps)
            state["last_received"] = time.time()
            state["signal_loss"] = False
            if status == "LOCK":
                state["lock_count"] += 1
            else:
                state["free_count"] += 1

def draw_ui(stdscr):
    global state

    curses.curs_set(0)
    stdscr.nodelay(True)

    try:
        ser = serial.Serial(SERIAL_PORT, BAUD_RATE, timeout=0.1)
    except serial.SerialException as e:
        stdscr.addstr(0, 0, f"[ERROR] Couldn't open {SERIAL_PORT}: {e}")
        stdscr.getch()
        return

    while True:
        try:
            # Read as fast as possible
            serial_reader(ser)

            # Check for signal timeout
            if state["last_received"]:
                elapsed = time.time() - state["last_received"]
                if elapsed > SIGNAL_TIMEOUT:
                    state["signal_loss"] = True

            # Draw UI
            stdscr.clear()
            stdscr.addstr(0, 0, "🕰  NTP Timeturner v0.4")

            if state["signal_loss"]:
                stdscr.addstr(2, 0, "⚠️  No LTC signal detected!")
                stdscr.addstr(3, 0, f"Last seen: {elapsed:.2f}s ago")
            else:
                stdscr.addstr(2, 0, f"LTC Status   : {state['ltc_status']}")
                stdscr.addstr(3, 0, f"LTC Timecode : {state['ltc_timecode']}")
                stdscr.addstr(4, 0, f"Frame Rate   : {state['framerate']}")
                stdscr.addstr(5, 0, f"System Clock : {state['system_clock']}")
                stdscr.addstr(6, 0, f"Sync Offset  : {state['offset_str']}")
                stdscr.addstr(7, 0, f"Lock Ratio   : {state['lock_count']} LOCK / {state['free_count']} FREE")

            stdscr.addstr(9, 0, "Press Ctrl+C to exit.")
            stdscr.refresh()
            time.sleep(UI_REFRESH_INTERVAL)

        except KeyboardInterrupt:
            break
        except Exception as e:
            stdscr.addstr(11, 0, f"[EXCEPTION] {e}")
            stdscr.refresh()
            time.sleep(1)

    ser.close()

if __name__ == "__main__":
    curses.wrapper(draw_ui)
