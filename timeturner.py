import curses
import serial
import re
import time
import subprocess
from datetime import datetime

SERIAL_PORT = "/dev/ttyACM0"
BAUD_RATE = 115200
UI_REFRESH_INTERVAL = 0.25
SIGNAL_TIMEOUT = 1.5

ltc_pattern = re.compile(
    r"\[(LOCK|FREE)\]\s+(\d{2}:\d{2}:\d{2}[:;]\d{2})\s+\|\s+([\d.]+fps)", re.IGNORECASE
)

state = {
    "ltc_status": "--",
    "ltc_timecode": "--:--:--:--",
    "framerate": "--",
    "system_clock": "--:--:--.---",
    "offset_str": "--",
    "lock_count": 0,
    "free_count": 0,
    "last_received": None,
    "signal_loss": False,
    "last_ltc_dt": None,
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

            state.update({
                "ltc_status": status,
                "ltc_timecode": tc_str,
                "framerate": fps_str,
                "system_clock": now.strftime("%H:%M:%S.%f")[:-3],
                "offset_str": format_offset(get_offset(now, h, m, s, f, fps), fps),
                "last_received": time.time(),
                "signal_loss": False,
                "last_ltc_dt": f"{h:02}:{m:02}:{s:02}"
            })
            if status == "LOCK":
                state["lock_count"] += 1
            else:
                state["free_count"] += 1


def draw_ui(stdscr):
    global state
    curses.curs_set(0)
    stdscr.nodelay(True)
    curses.start_color()

    curses.init_pair(1, curses.COLOR_GREEN, curses.COLOR_BLACK)   # LOCK
    curses.init_pair(2, curses.COLOR_YELLOW, curses.COLOR_BLACK)  # FREE
    curses.init_pair(3, curses.COLOR_RED, curses.COLOR_BLACK)     # LOST
    curses.init_pair(4, curses.COLOR_CYAN, curses.COLOR_BLACK)    # OFFSET

    try:
        ser = serial.Serial(SERIAL_PORT, BAUD_RATE, timeout=0.1)
    except serial.SerialException as e:
        stdscr.addstr(0, 0, f"[ERR] Couldn't open {SERIAL_PORT}: {e}")
        stdscr.getch()
        return

    while True:
        try:
            serial_reader(ser)
            if state["last_received"] and (time.time() - state["last_received"]) > SIGNAL_TIMEOUT:
                state["signal_loss"] = True

            stdscr.clear()
            stdscr.addstr(0, 0, "NTP Timeturner v0.5")

            if state["signal_loss"]:
                stdscr.attron(curses.color_pair(3))
                stdscr.addstr(2, 0, "! No LTC signal detected")
                stdscr.attroff(curses.color_pair(3))
            else:
                colour = curses.color_pair(1 if state['ltc_status'] == "LOCK" else 2)
                stdscr.addstr(2, 0, f"LTC Status   : ")
                stdscr.attron(colour)
                stdscr.addstr(state['ltc_status'])
                stdscr.attroff(colour)

                stdscr.addstr(3, 0, f"LTC Timecode : {state['ltc_timecode']}")
                stdscr.addstr(4, 0, f"Frame Rate   : {state['framerate']}")
                stdscr.addstr(5, 0, f"System Clock : {state['system_clock']}")
                stdscr.attron(curses.color_pair(4))
                stdscr.addstr(6, 0, f"Sync Offset  : {state['offset_str']}")
                stdscr.attroff(curses.color_pair(4))
                stdscr.addstr(7, 0, f"Lock Ratio   : {state['lock_count']} LOCK / {state['free_count']} FREE")

            stdscr.addstr(9, 0, "[S] Set system clock to LTC    [Ctrl+C] Quit")
            stdscr.refresh()

            key = stdscr.getch()
            if key in (ord('s'), ord('S')) and not state['signal_loss'] and state['last_ltc_dt']:
                try:
                    subprocess.run(["sudo", "date", "-s", state['last_ltc_dt']], check=True)
                    stdscr.addstr(11, 0, "[OK] System clock updated to LTC")
                except Exception as e:
                    stdscr.addstr(11, 0, f"[ERR] Failed to set clock: {e}")

            time.sleep(UI_REFRESH_INTERVAL)

        except KeyboardInterrupt:
            break
        except Exception as e:
            stdscr.addstr(13, 0, f"[ERR] {e}")
            time.sleep(1)

    ser.close()


if __name__ == "__main__":
    curses.wrapper(draw_ui)
