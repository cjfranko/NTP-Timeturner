import serial
import time
import datetime
import curses
import os
from datetime import datetime as dt, timedelta

BAUD_RATE = 115200
SCAN_PORTS = ["/dev/ttyACM0", "/dev/ttyUSB0", "/dev/serial0", "/dev/serial1"]
VERSION = "0.6"

def find_serial_port():
    for port in SCAN_PORTS:
        if os.path.exists(port):
            return port
    return None

def parse_ltc_line(line):
    try:
        parts = line.strip().split()
        if len(parts) != 4:
            return None

        status = parts[0][1:-1]  # [LOCK] -> LOCK
        timecode = parts[1]      # e.g. 10:00:00:12 or 10:00:00;12
        framerate = float(parts[3].replace("fps", ""))

        sep = ":" if ":" in timecode else ";"
        hh, mm, ss, ff = map(int, timecode.replace(";", ":").split(":"))

        now = dt.now()
        ltc_dt = now.replace(hour=hh, minute=mm, second=ss, microsecond=0)
        return (status, ltc_dt, framerate, ff)
    except Exception:
        return None

def run_curses(stdscr):
    curses.curs_set(0)
    stdscr.nodelay(True)
    stdscr.timeout(200)

    port = find_serial_port()
    if port is None:
        stdscr.addstr(0, 0, "No serial port found. Check connection.")
        stdscr.refresh()
        time.sleep(3)
        return

    ser = serial.Serial(port, BAUD_RATE, timeout=0.1)
    buffer = ""
    last_ltc_dt = None
    last_frame = None
    frame_rate = None
    status = "FREE"
    lock_count = 0
    free_count = 0
    sync_offset_ms = 0

    while True:
        try:
            buffer += ser.read(ser.in_waiting or 1).decode("utf-8", errors="ignore")
            while "\n" in buffer:
                line, buffer = buffer.split("\n", 1)
                result = parse_ltc_line(line)
                if result:
                    status, ltc_dt, frame_rate, frame = result
                    last_ltc_dt = ltc_dt
                    last_frame = frame
                    if status == "LOCK":
                        lock_count += 1
                    else:
                        free_count += 1

        except Exception as e:
            stdscr.addstr(0, 0, f"Serial Error: {str(e)}")
            stdscr.refresh()
            time.sleep(1)
            return

        now = dt.now()
        if last_ltc_dt:
            offset_td = now - last_ltc_dt
            sync_offset_ms = round(offset_td.total_seconds() * 1000)

        stdscr.erase()
        stdscr.addstr(0, 2, f"NTP Timeturner v{VERSION}")
        stdscr.addstr(1, 2, f"Using Serial Port: {port}")

        stdscr.addstr(3, 2, f"LTC Status   : {status}")
        if last_ltc_dt and last_frame is not None:
            stdscr.addstr(4, 2, f"LTC Timecode : {last_ltc_dt.strftime('%H:%M:%S')}:{last_frame:02}")
        else:
            stdscr.addstr(4, 2, f"LTC Timecode : ---")

        stdscr.addstr(5, 2, f"Frame Rate   : {frame_rate:.2f}fps" if frame_rate else "Frame Rate   : ---")
        stdscr.addstr(6, 2, f"System Clock : {now.strftime('%H:%M:%S.%f')[:-3]}")

        if sync_offset_ms and frame_rate:
            frame_duration_ms = 1000 / frame_rate
            offset_frames = round(sync_offset_ms / frame_duration_ms)
            stdscr.addstr(7, 2, f"Sync Offset  : {sync_offset_ms:+} ms ({offset_frames:+} frames)")
        else:
            stdscr.addstr(7, 2, "Sync Offset  : ---")

        stdscr.addstr(8, 2, f"Lock Ratio   : {lock_count} LOCK / {free_count} FREE")
        stdscr.addstr(10, 2, "[S] Set system clock to LTC    [Ctrl+C] Quit")
        stdscr.refresh()

        # Handle user input
        try:
            key = stdscr.getch()
            if key in [ord('s'), ord('S')]:
                if last_ltc_dt:
                    new_dt = last_ltc_dt.replace(microsecond=0)
                    date_str = new_dt.strftime("%H:%M:%S")
                    os.system(f"sudo date -s \"{date_str}\"")
            elif key == 3:  # Ctrl+C
                raise KeyboardInterrupt
        except:
            pass

        time.sleep(0.05)

if __name__ == "__main__":
    try:
        curses.wrapper(run_curses)
    except KeyboardInterrupt:
        print("\nExited cleanly.")
