#!/usr/bin/env python3

import curses
import subprocess
import shutil
import time

def start_ltc_stream():
    # Launch ffmpeg piped into ltcdump
    ffmpeg = subprocess.Popen(
        ["ffmpeg", "-f", "alsa", "-i", "default", "-ac", "1", "-ar", "48000", "-f", "s16le", "-"],
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL
    )
    ltcdump = subprocess.Popen(
        ["ltcdump", "-f", "-"],
        stdin=ffmpeg.stdout,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        text=True
    )
    ffmpeg.stdout.close()  # Let ltcdump consume the pipe
    return ffmpeg, ltcdump

def main(stdscr):
    curses.curs_set(0)
    stdscr.nodelay(True)

    stdscr.addstr(1, 2, "🌀 NTP Timeturner Status")
    stdscr.addstr(3, 4, "Streaming LTC from default input...")

    ffmpeg_proc, ltcdump_proc = start_ltc_stream()

    latest_tc = "⌛ Waiting for LTC..."
    last_refresh = time.time()

    try:
        while True:
            stdscr.clear()
            stdscr.addstr(1, 2, "🌀 NTP Timeturner Status")
            stdscr.addstr(3, 4, "Streaming LTC from default input...")
            stdscr.addstr(5, 6, f"🕰️ LTC Timecode: {latest_tc}")
            stdscr.refresh()

            # Check if new LTC line available
            if ltcdump_proc.stdout.readable():
                line = ltcdump_proc.stdout.readline().strip()
                if line and line[0].isdigit():
                    latest_tc = line

            # Limit screen redraw to ~10fps
            time.sleep(0.1)

    except KeyboardInterrupt:
        stdscr.addstr(8, 6, "🔚 Shutting down...")
        stdscr.refresh()
        time.sleep(1)
    finally:
        ffmpeg_proc.terminate()
        ltcdump_proc.terminate()

if __name__ == "__main__":
    if not shutil.which("ltcdump") or not shutil.which("ffmpeg"):
        print("❌ Required tools not found (ltcdump or ffmpeg). Install and retry.")
        exit(1)

    curses.wrapper(main)
