#!/usr/bin/env python3

import curses
import subprocess
import time
import shutil

AUDIO_DEVICE = "hw:1"  # Change this if needed

def read_ltc():
    ffmpeg = subprocess.Popen(
        ["ffmpeg", "-f", "alsa", "-i", AUDIO_DEVICE, "-t", "1", "-f", "s16le", "-ac", "1", "-ar", "48000", "-"],
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL
    )
    ltcdump = subprocess.Popen(
        ["ltcdump", "-f", "-"],
        stdin=ffmpeg.stdout,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL
    )
    ffmpeg.stdout.close()
    output, _ = ltcdump.communicate()
    lines = output.decode().splitlines()
    return lines[-1] if lines else "⚠️ No LTC signal"

def main(stdscr):
    curses.curs_set(0)
    stdscr.nodelay(True)

    while True:
        stdscr.clear()

        stdscr.addstr(1, 2, "🌀 NTP Timeturner Status")
        stdscr.addstr(3, 4, "Reading LTC from audio device...")

        try:
            ltc_timecode = read_ltc()
        except Exception as e:
            ltc_timecode = f"Error: {e}"

        stdscr.addstr(5, 6, f"🕰️ LTC Timecode: {ltc_timecode}")

        stdscr.refresh()
        time.sleep(1)

if __name__ == "__main__":
    # Pre-flight checks
    if not shutil.which("ltcdump") or not shutil.which("ffmpeg"):
        print("❌ Required tools not found (ltcdump, ffmpeg). Install and retry.")
        exit(1)

    curses.wrapper(main)
