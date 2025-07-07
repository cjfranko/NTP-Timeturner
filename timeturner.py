import serial
import curses
import time
import datetime
import re
import subprocess
import os

SERIAL_PORT = None
BAUD_RATE = 115200
FRAME_RATE = 25.0

last_ltc_time = None
last_frame = None
lock_count = 0
free_count = 0
sync_pending = False

def find_teensy_serial():
    for dev in os.listdir('/dev'):
        if dev.startswith('ttyACM') or dev.startswith('ttyUSB'):
            return f'/dev/{dev}'
    return None

def parse_ltc_line(line):
    match = re.match(r"\[(LOCK|FREE)\]\s+(\d{2}):(\d{2}):(\d{2})[:;](\d{2})\s+\|\s+([\d.]+)fps", line)
    if not match:
        return None
    status, hh, mm, ss, ff, fps = match.groups()
    return {
        "status": status,
        "hours": int(hh),
        "minutes": int(mm),
        "seconds": int(ss),
        "frames": int(ff),
        "frame_rate": float(fps)
    }

def get_system_time():
    return datetime.datetime.now()

def format_time(dt):
    return dt.strftime("%H:%M:%S.%f")[:-3]

def run_curses(stdscr):
    global last_ltc_time, FRAME_RATE, lock_count, free_count, sync_pending, SERIAL_PORT

    curses.curs_set(0)
    stdscr.nodelay(True)
    stdscr.timeout(100)

    SERIAL_PORT = find_teensy_serial()
    if not SERIAL_PORT:
        stdscr.addstr(0, 0, "No serial device found.")
        stdscr.refresh()
        time.sleep(3)
        return

    try:
        ser = serial.Serial(SERIAL_PORT, BAUD_RATE, timeout=1)
    except Exception as e:
        stdscr.addstr(0, 0, f"Error opening serial: {e}")
        stdscr.refresh()
        time.sleep(3)
        return

    while True:
        try:
            pre_read_time = datetime.datetime.now()
            line = ser.readline().decode(errors='ignore').strip()
            parsed = parse_ltc_line(line)

            if parsed:
                FRAME_RATE = parsed["frame_rate"]

                if parsed["status"] == "LOCK":
                    lock_count += 1
                else:
                    free_count += 1

                ms = int((parsed["frames"] / FRAME_RATE) * 1000)
                ltc_dt = pre_read_time.replace(
                    hour=parsed["hours"],
                    minute=parsed["minutes"],
                    second=parsed["seconds"],
                    microsecond=ms * 1000
                )

                last_ltc_time = ltc_dt
                last_frame = parsed["frames"]

                if sync_pending:
                    do_sync(stdscr, ltc_dt)
                    sync_pending = False

            # Drawing UI
            stdscr.erase()
            stdscr.addstr(0, 2, f"NTP Timeturner v0.8")
            stdscr.addstr(1, 2, f"Using Serial Port: {SERIAL_PORT}")
            stdscr.addstr(3, 2, f"LTC Status   : {parsed['status'] if parsed else '…'}")
            stdscr.addstr(4, 2, f"LTC Timecode : {parsed['hours']:02}:{parsed['minutes']:02}:{parsed['seconds']:02}:{parsed['frames']:02}" if parsed else "LTC Timecode : …")
            stdscr.addstr(5, 2, f"Frame Rate   : {FRAME_RATE:.2f}fps")

            now = get_system_time()
            stdscr.addstr(6, 2, f"System Clock : {format_time(now)}")

            if last_ltc_time:
                offset_ms = (now - last_ltc_time).total_seconds() * 1000
                offset_frames = offset_ms / (1000 / FRAME_RATE)
                stdscr.addstr(7, 2, f"Sync Offset  : {offset_ms:+.0f} ms ({offset_frames:+.0f} frames)")

            stdscr.addstr(8, 2, f"Lock Ratio   : {lock_count} LOCK / {free_count} FREE")
            stdscr.addstr(10, 2, "[S] Set system clock to LTC    [Ctrl+C] Quit")
            stdscr.refresh()

            key = stdscr.getch()
            if key in (ord('s'), ord('S')):
                sync_pending = True

        except KeyboardInterrupt:
            break
        except Exception as e:
            stdscr.addstr(13, 2, f"⚠️ Error: {e}")
            stdscr.refresh()
            time.sleep(1)

def do_sync(stdscr, ltc_dt):
    try:
        timestamp = ltc_dt.strftime("%H:%M:%S.%f")[:-3]
        subprocess.run(["sudo", "date", "-s", timestamp], check=True)
        stdscr.addstr(13, 2, f"✔️ System clock set to: {timestamp}")
    except Exception as e:
        stdscr.addstr(13, 2, f"❌ Sync failed: {e}")

if __name__ == "__main__":
    curses.wrapper(run_curses)
