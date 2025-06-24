#!/usr/bin/env python3

import curses
import subprocess
import shutil
import time
import select

def start_ltc_stream():
    # Launch arecord piped into ltcdump
    arecord = subprocess.Popen(
        ["arecord", "-f", "S16_LE", "-c", "1", "-r", "48000", "-D", "hw:2,0"],
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL
    )
    ltcdump = subprocess.Popen(
        ["ltcdump", "-f", "-"],
        stdin=arecord.stdout,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        text=True,
        bufsize=1
    )
    arecord.stdout.close()  # Let ltcdump consume the pipe
    return arecord, ltcdump

def main(stdscr):
    curses.curs_set(0)
    stdscr.nodelay(True)

    arecord_proc, ltcdump_proc = start_ltc_stream()

    latest_tc = "⌛ Waiting for LTC..."
    last_update = time.time()

    try:
        while True:
            # Non-blocking read from ltcdump
            rlist, _, _ = select.select([ltcdump_proc.stdout], [], [], 0)
            if rlist:
                line = ltcdump_proc.stdout.readline()
                if line:
                    line = line.strip()
                    if line and line[0].isdigit():
                        latest_tc = line
                        last_update = time.time()

            # Timeout / error detection
            if time.time() - last_update > 1:
                if ltcdump_proc.poll() is not None or arecord_proc.poll() is not None:
                    latest_tc = "💥 Decoder crashed or stream stopped"
                else:
                    latest_tc = "⚠️  No LTC signal"

            # Draw the curses UI
            stdscr.erase()
            stdscr.addstr(1, 2, "🌀 NTP Timeturner Status")
            stdscr.addstr(3, 4, "Streaming LTC from hw:2,0...")
            stdscr.addstr(5, 6, f"🕰️ LTC Timecode: {latest_tc}")
            stdscr.refresh()

            time.sleep(0.04)  # ~25 FPS

    except KeyboardInterrupt:
        stdscr.addstr(8, 6, "🔚 Shutting down...")
        stdscr.refresh()
        time.sleep(1)
    finally:
        arecord_proc.terminate()
        ltcdump_proc.terminate()

if __name__ == "__main__":
    if not shutil.which("ltcdump") or not shutil.which("arecord"):
        print("❌ Required tools not found (ltcdump or arecord). Install and retry.")
        exit(1)

    curses.wrapper(main)
