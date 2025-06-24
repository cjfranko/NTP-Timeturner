#!/usr/bin/env python3

import curses
import subprocess
import shutil
import time
import select

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
        text=True,
        bufsize=1  # Line-buffered
    )
    ffmpeg.stdout.close()  # Let ltcdump consume the pipe
    return ffmpeg, ltcdump

def main(stdscr):
    curses.curs_set(0)
    stdscr.nodelay(True)

    ffmpeg_proc, ltcdump_proc = start_ltc_stream()

    latest_tc = "⌛ Waiting for LTC..."
    last_update = time.time()

    try:
        while True:
            # Check for new LTC output (non-blocking)
            rlist, _, _ = select.select([ltcdump_proc.stdout], [], [], 0)
            if rlist:
                line = ltcdump_proc.stdout.readline()
                if line:
                    line = line.strip()
                    if line and line[0].isdigit():
                        latest_tc = line
                        last_update = time.time()

            # Check if signal or subprocess died
            if time.time() - last_update > 1:
                if ltcdump_proc.poll() is not None or ffmpeg_proc.poll() is not None:
                    latest_tc = "💥 Decoder crashed or stream stopped"
                else:
                    latest_tc = "⚠️  No LTC signal"

            # Draw UI
            stdscr.erase()
            stdscr.addstr(1, 2, "🌀 NTP Timeturner Status")
            stdscr.addstr(3, 4, "Streaming LTC from default input...")
            stdscr.addstr(5, 6, f"🕰️ LTC Timecode: {latest_tc}")
            stdscr.refresh()

            time.sleep(0.04)  # ~25 FPS

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
