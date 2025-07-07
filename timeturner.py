# -*- coding: utf-8 -*-
import curses
import serial
import re
import time
from datetime import datetime

# Serial config
SERIAL_PORT = "/dev/ttyACM0"
BAUD_RATE = 115200
REFRESH_INTERVAL = 0.5  # seconds

# Regex pattern
ltc_pattern = re.compile(
    r"\[(LOCK|FREE)\]\s+(\d{2}:\d{2}:\d{2}[:;]\d{2})\s+\|\s+([\d.]+fps)", re.IGNORECASE
)

# Stats
lock_count = 0
free_count = 0
last_frame = None
drift_warnings = []

def parse_timecode(tc_str):
    sep = ":" if ":" in tc_str else ";"
    h, m, s, f = map(int, tc_str.replace(";", ":").split(":"))
    return h, m, s, f, sep

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

def draw_ui(stdscr):
    global lock_count, free_count, last_frame, drift_warnings

    curses.curs_set(0)
    stdscr.nodelay(True)

    try:
        ser = serial.Serial(SERIAL_PORT, BAUD_RATE, timeout=1)
    except serial.SerialException as e:
        stdscr.addstr(0, 0, f"[ERROR] Couldn't open {SERIAL_PORT}: {e}")
        stdscr.getch()
        return

    # Init variables
    ltc_status = "--"
    ltc_timecode = "--:--:--:--"
    framerate = "--"
    offset_str = "--"

    while True:
        try:
            line = ser.readline().decode(errors='ignore').strip()
            match = ltc_pattern.match(line)
            now = datetime.now()

            if match:
                status, tc_str, fps_str = match.groups()
                ltc_status = status
                ltc_timecode = tc_str
                framerate = fps_str
                fps = float(fps_str.lower().replace("fps", ""))

                h, m, s, f, sep = parse_timecode(tc_str)
                current_frame = timecode_to_milliseconds(h, m, s, f, fps)

                # Drift detection
                if last_frame is not None:
                    expected = last_frame + int(1000 / fps)
                    if abs(current_frame - expected) > int(2 * (1000 / fps)):
                        drift_warnings.append(f"Drift: Δ{current_frame - last_frame} ms")
                        if len(drift_warnings) > 3:
                            drift_warnings.pop(0)
                last_frame = current_frame

                # Sync offset
                offset_ms = get_offset(now, h, m, s, f, fps)
                offset_str = format_offset(offset_ms, fps)

                # Stats
                if ltc_status == "LOCK":
                    lock_count += 1
                else:
                    free_count += 1

            # UI
            stdscr.clear()
            stdscr.addstr(0, 0, "🕰  NTP Timeturner v0.3")
            stdscr.addstr(2, 0, f"LTC Status   : {ltc_status}")
            stdscr.addstr(3, 0, f"LTC Timecode : {ltc_timecode}")
            stdscr.addstr(4, 0, f"Frame Rate   : {framerate}")
            stdscr.addstr(5, 0, f"System Clock : {now.strftime('%H:%M:%S.%f')[:-3]}")
            stdscr.addstr(6, 0, f"Sync Offset  : {offset_str}")
            stdscr.addstr(7, 0, f"Lock Ratio   : {lock_count} LOCK / {free_count} FREE")
            if drift_warnings:
                stdscr.addstr(9, 0, f"⚠️  {drift_warnings[-1]}")
            stdscr.addstr(11, 0, "Press Ctrl+C to exit.")
            stdscr.refresh()
            time.sleep(REFRESH_INTERVAL)

        except KeyboardInterrupt:
            break
        except Exception as e:
            stdscr.addstr(13, 0, f"[EXCEPTION] {e}")
            stdscr.refresh()
            time.sleep(1)

    ser.close()

if __name__ == "__main__":
    curses.wrapper(draw_ui)
