import curses
import datetime
import serial
import subprocess
import time
import re
import glob
import os

BAUD_RATE = 115200
line_regex = re.compile(r"\[(LOCK|FREE)\]\s+(\d{2}:\d{2}:\d{2}[:;]\d{2})\s+\|\s+([\d.]+)fps")

def find_serial_port():
    candidates = sorted(glob.glob('/dev/ttyACM*') + glob.glob('/dev/ttyUSB*'))
    for port in candidates:
        try:
            s = serial.Serial(port, BAUD_RATE, timeout=1)
            s.close()
            return port
        except serial.SerialException:
            continue
    return None

def parse_ltc_line(line):
    match = line_regex.match(line.strip())
    if not match:
        return None
    status, timecode, fps = match.groups()
    return status, timecode.replace(';', ':'), float(fps)

def get_system_time():
    return datetime.datetime.now()

def timecode_to_dt(tc):
    try:
        h, m, s, f = map(int, tc.split(":"))
        return datetime.datetime.now().replace(hour=h, minute=m, second=s, microsecond=0)
    except Exception:
        return None

def run_curses(stdscr):
    curses.curs_set(0)
    curses.start_color()
    curses.use_default_colors()

    curses.init_pair(1, curses.COLOR_GREEN, -1)
    curses.init_pair(2, curses.COLOR_YELLOW, -1)
    curses.init_pair(3, curses.COLOR_RED, -1)
    curses.init_pair(4, curses.COLOR_CYAN, -1)

    serial_port = find_serial_port()
    if not serial_port:
        stdscr.addstr(0, 0, "❌ Could not find Teensy serial port (ACM/USB).")
        stdscr.refresh()
        time.sleep(3)
        return

    try:
        ser = serial.Serial(serial_port, BAUD_RATE, timeout=1)
    except Exception as e:
        stdscr.addstr(0, 0, f"❌ Failed to open {serial_port}: {e}")
        stdscr.refresh()
        time.sleep(3)
        return

    lock_count = 0
    free_count = 0
    last_ltc_dt = None
    last_status = "LOST"
    frame_rate = 0.0

    sync_requested = False
    syncing = False

    while True:
        now = get_system_time()
        line = ser.readline().decode(errors='ignore').strip()
        if line:
            parsed = parse_ltc_line(line)
            if parsed:
                status, tc_str, fps = parsed
                frame_rate = fps
                last_ltc_dt = timecode_to_dt(tc_str)
                last_status = status
                if status == "LOCK":
                    lock_count += 1
                else:
                    free_count += 1

                if sync_requested and not syncing:
                    syncing = True
                    new_time = last_ltc_dt.strftime("%H:%M:%S")
                    try:
                        subprocess.run(["sudo", "date", "-s", new_time], check=True)
                        sync_feedback = f"[OK] Clock set to {new_time}"
                    except subprocess.CalledProcessError as e:
                        sync_feedback = f"[ERR] Failed to sync: {e}"
                    sync_requested = False
                    syncing = False

        if last_ltc_dt:
            sys_time = now.replace(microsecond=0)
            offset = (sys_time - last_ltc_dt).total_seconds()
            offset_ms = int(offset * 1000)
            offset_frames = int(round(offset * frame_rate))
            offset_str = f"{offset_ms:+} ms ({offset_frames:+} frames)"
        else:
            offset_str = "n/a"

        stdscr.erase()
        stdscr.addstr(0, 0, "NTP Timeturner v0.5")
        stdscr.addstr(1, 0, f"Using Serial Port: {serial_port}")
        stdscr.addstr(3, 0, "LTC Status   : ")
        if last_status == "LOCK":
            stdscr.addstr("LOCK", curses.color_pair(1))
        elif last_status == "FREE":
            stdscr.addstr("FREE", curses.color_pair(2))
        else:
            stdscr.addstr("LOST", curses.color_pair(3))

        stdscr.addstr(4, 0, f"LTC Timecode : {last_ltc_dt.strftime('%H:%M:%S') if last_ltc_dt else 'n/a'}")
        stdscr.addstr(5, 0, f"Frame Rate   : {frame_rate:.2f}fps")
        stdscr.addstr(6, 0, f"System Clock : {now.strftime('%H:%M:%S.%f')[:-3]}")
        stdscr.addstr(7, 0, "Sync Offset  : ")
        stdscr.addstr(offset_str, curses.color_pair(4))
        stdscr.addstr(8, 0, f"Lock Ratio   : {lock_count} LOCK / {free_count} FREE")
        stdscr.addstr(10, 0, "[S] Set system clock to LTC    [Ctrl+C] Quit")

        if 'sync_feedback' in locals():
            stdscr.addstr(12, 0, sync_feedback[:curses.COLS - 1])
            del sync_feedback

        stdscr.refresh()

        stdscr.nodelay(True)
        try:
            key = stdscr.getch()
            if key == ord('s') or key == ord('S'):
                sync_requested = True
        except:
            pass

        time.sleep(0.1)

if __name__ == "__main__":
    curses.wrapper(run_curses)
