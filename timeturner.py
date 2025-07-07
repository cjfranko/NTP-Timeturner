import serial
import curses
import time
import datetime
import re
import subprocess
import os
import threading
import queue

SERIAL_PORT = None
BAUD_RATE = 115200
FRAME_RATE = 25.0

lock_count = 0
free_count = 0
sync_pending = False

ltc_data_queue = queue.Queue()
latest_ltc = None

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

def serial_thread(port, baud, q):
    try:
        ser = serial.Serial(port, baud, timeout=1)
        while True:
            line = ser.readline().decode(errors='ignore').strip()
            timestamp = datetime.datetime.now()
            parsed = parse_ltc_line(line)
            if parsed:
                q.put((parsed, timestamp))
    except Exception as e:
        print(f"Serial thread error: {e}")

def get_system_time():
    return datetime.datetime.now()

def format_time(dt):
    return dt.strftime("%H:%M:%S.%f")[:-3]

def run_curses(stdscr):
    global FRAME_RATE, lock_count, free_count, sync_pending, SERIAL_PORT, latest_ltc

    curses.curs_set(0)
    stdscr.nodelay(True)
    stdscr.timeout(50)

    SERIAL_PORT = find_teensy_serial()
    if not SERIAL_PORT:
        stdscr.addstr(0, 0, "❌ No serial device found.")
        stdscr.refresh()
        time.sleep(2)
        return

    # Start serial reader thread
    thread = threading.Thread(target=serial_thread, args=(SERIAL_PORT, BAUD_RATE, ltc_data_queue), daemon=True)
    thread.start()

    while True:
        try:
            # Pull latest from queue if available
            while not ltc_data_queue.empty():
                parsed, arrival_time = ltc_data_queue.get_nowait()
                latest_ltc = (parsed, arrival_time)

            stdscr.erase()
            stdscr.addstr(0, 2, "NTP Timeturner v0.9")
            stdscr.addstr(1, 2, f"Using Serial Port: {SERIAL_PORT}")

            if latest_ltc:
                parsed, arrival_time = latest_ltc
                FRAME_RATE = parsed["frame_rate"]
                if parsed["status"] == "LOCK":
                    lock_count += 1
                else:
                    free_count += 1

                stdscr.addstr(3, 2, f"LTC Status   : {parsed['status']}")
                stdscr.addstr(4, 2, f"LTC Timecode : {parsed['hours']:02}:{parsed['minutes']:02}:{parsed['seconds']:02}:{parsed['frames']:02}")
                stdscr.addstr(5, 2, f"Frame Rate   : {FRAME_RATE:.2f}fps")
                stdscr.addstr(6, 2, f"System Clock : {format_time(get_system_time())}")

                offset_ms = (get_system_time() - arrival_time).total_seconds() * 1000
                offset_frames = offset_ms / (1000 / FRAME_RATE)
                stdscr.addstr(7, 2, f"Sync Offset  : {offset_ms:+.0f} ms ({offset_frames:+.0f} frames)")
            else:
                stdscr.addstr(3, 2, "LTC Status   : (waiting)")
                stdscr.addstr(4, 2, "LTC Timecode : …")
                stdscr.addstr(5, 2, "Frame Rate   : …")
                stdscr.addstr(6, 2, f"System Clock : {format_time(get_system_time())}")
                stdscr.addstr(7, 2, "Sync Offset  : …")

            stdscr.addstr(8, 2, f"Lock Ratio   : {lock_count} LOCK / {free_count} FREE")
            stdscr.addstr(10, 2, "[S] Set system clock to LTC    [Ctrl+C] Quit")
            stdscr.refresh()

            key = stdscr.getch()
            if key in (ord('s'), ord('S')) and latest_ltc:
                do_sync(stdscr, latest_ltc[0], latest_ltc[1])
                sync_pending = False

        except KeyboardInterrupt:
            break
        except Exception as e:
            stdscr.addstr(13, 2, f"⚠️ Error: {e}")
            stdscr.refresh()
            time.sleep(1)

def do_sync(stdscr, parsed, arrival_time):
    try:
        ms = int((parsed["frames"] / parsed["frame_rate"]) * 1000)
        sync_time = arrival_time.replace(
            hour=parsed["hours"],
            minute=parsed["minutes"],
            second=parsed["seconds"],
            microsecond=ms * 1000
        )
        timestamp = sync_time.strftime("%H:%M:%S.%f")[:-3]
        subprocess.run(["sudo", "date", "-s", timestamp], check=True)
        stdscr.addstr(13, 2, f"✔️ Synced to LTC: {timestamp}")
    except Exception as e:
        stdscr.addstr(13, 2, f"❌ Sync failed: {e}")

if __name__ == "__main__":
    curses.wrapper(run_curses)
