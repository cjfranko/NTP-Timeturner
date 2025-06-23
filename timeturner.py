#!/usr/bin/env python3

"""
timeturner.py
NTP Timeturner — LTC-to-NTP time server for Raspberry Pi
Now with a colourful LTC status light 🌈🟢
"""

import os
import sys
import time
import logging
import json
import curses
from datetime import datetime, timedelta

CONFIG_PATH = "config.json"

DEFAULT_CONFIG = {
    "ltc_device": "/dev/audio",
    "offset": {
        "hours": 0,
        "minutes": 0,
        "seconds": 0,
        "milliseconds": 0
    },
    "frame_rate": 25
}

TIMETURNER_SPINNER = ['⧗', '⧖', '⧗', '⧖', '🕰']
HEARTBEAT_PULSE = ['●', '○']

def load_config(path=CONFIG_PATH):
    if not os.path.exists(path):
        logging.warning("Config file not found, using defaults.")
        return DEFAULT_CONFIG
    with open(path, "r") as f:
        return json.load(f)

def read_ltc_time():
    return datetime.utcnow()

def apply_offset(base_time, offset):
    delta = timedelta(
        hours=offset.get("hours", 0),
        minutes=offset.get("minutes", 0),
        seconds=offset.get("seconds", 0),
        milliseconds=offset.get("milliseconds", 0)
    )
    return base_time + delta

def set_system_time(new_time):
    formatted = new_time.strftime('%Y-%m-%d %H:%M:%S')
    logging.info(f"Setting system time to: {formatted}")
    # os.system(f"sudo timedatectl set-time \"{formatted}\"")

def draw_dashboard(stdscr, ltc_time, adjusted_time, config, frame):
    stdscr.clear()

    # Setup strings
    offset = config["offset"]
    offset_str = f"{offset['hours']:02}:{offset['minutes']:02}:{offset['seconds']:02}.{offset['milliseconds']:03}"
    spinner = TIMETURNER_SPINNER[frame % len(TIMETURNER_SPINNER)]
    heartbeat = HEARTBEAT_PULSE[frame % len(HEARTBEAT_PULSE)]

    # Hardcoded LTC status
    ltc_status = "LOCKED"
    ltc_colour = curses.color_pair(2)  # Green

    # Draw
    stdscr.addstr(0, 0, f"┌───────────────────────────────┐")
    stdscr.addstr(1, 0, f"│   {spinner} NTP Timeturner {spinner}        │")
    stdscr.addstr(2, 0, f"├───────────────────────────────┤")
    stdscr.addstr(3, 0, f"│ LTC Time:       {ltc_time.strftime('%H:%M:%S:%f')[:-3]} │")
    stdscr.addstr(4, 0, f"│ Offset Applied: +{offset_str:<17}│")
    stdscr.addstr(5, 0, f"│ System Time:    {adjusted_time.strftime('%H:%M:%S')}         │")
    stdscr.addstr(6, 0, f"│ Frame Rate:     {config['frame_rate']} fps            │")
    stdscr.addstr(7, 0, f"│ LTC Status:     ")
    stdscr.addstr("● ", ltc_colour)
    stdscr.addstr(f"{ltc_status:<14}│")
    stdscr.addstr(8, 0, f"│ NTP Broadcast:  PENDING        │")
    stdscr.addstr(9, 0, f"├───────────────────────────────┤")
    stdscr.addstr(10, 0, f"│ System Status:  {heartbeat}                 │")
    stdscr.addstr(11, 0, f"│ [Ctrl+C to exit]              │")
    stdscr.addstr(12, 0, f"└───────────────────────────────┘")

    stdscr.refresh()

def start_timeturner(stdscr):
    curses.curs_set(0)
    curses.start_color()
    curses.init_pair(1, curses.COLOR_RED, curses.COLOR_BLACK)     # Not used yet
    curses.init_pair(2, curses.COLOR_GREEN, curses.COLOR_BLACK)   # LOCKED
    curses.init_pair(3, curses.COLOR_YELLOW, curses.COLOR_BLACK)  # UNSTABLE

    stdscr.nodelay(True)
    stdscr.timeout(1000)

    config = load_config()
    frame = 0

    while True:
        try:
            ltc_time = read_ltc_time()
            adjusted_time = apply_offset(ltc_time, config["offset"])
            set_system_time(adjusted_time)
            draw_dashboard(stdscr, ltc_time, adjusted_time, config, frame)
            frame += 1
        except KeyboardInterrupt:
            break

if __name__ == "__main__":
    logging.basicConfig(level=logging.INFO,
                        format="[%(asctime)s] %(levelname)s: %(message)s")
    logging.info("✨ Timeturner console mode started.")
    curses.wrapper(start_timeturner)
