#!/usr/bin/env python3

import curses
import subprocess
import time
import shutil
import tempfile
import os

def read_ltc():
    with tempfile.NamedTemporaryFile(suffix=".wav", delete=False) as tmp:
        wav_path = tmp.name

    try:
        # Record 1 second of audio from default device
        subprocess.run([
            "ffmpeg", "-f", "alsa", "-i", "default",
            "-t", "1", "-ac", "1", "-ar", "48000", "-y", wav_path
        ], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)

        # Decode LTC from the recorded file
        result = subprocess.run(
            ["ltcdump", wav_path],
            capture_output=True,
            text=True
        )

        lines = result.stdout.strip().splitlines()
        ltc_lines = [line for line in lines if line and line[0].isdigit()]

        return ltc_lines[-1] if ltc_lines else "⚠️ No LTC decoded"

    finally:
        os.remove(wav_path)

def main(stdscr):
    curses.curs_set(0)
    stdscr.nodelay(True)

    while True:
        stdscr.clear()

        stdscr.addstr(1, 2, "🌀 NTP Timeturner Status")
        stdscr.addstr(3, 4, "Reading LTC from default audio input...")

        try:
            ltc_timecode = read_ltc()
        except Exception as e:
            ltc_timecode = f"❌ Error: {e}"

        stdscr.addstr(5, 6, f"🕰️ LTC Timecode: {ltc_timecode}")

        stdscr.refresh()
        time.sleep(1)

if __name__ == "__main__":
    # Pre-flight check
    if not shutil.which("ltcdump") or not shutil.which("ffmpeg"):
        print("❌ Required tools not found (ltcdump or ffmpeg). Install them and retry.")
        exit(1)

    curses.wrapper(main)
