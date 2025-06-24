#!/usr/bin/env python3

import os
import subprocess
import curses
import time
import shutil
import select
import signal

FIFO_PATH = "/tmp/ltcpipe"

def setup_fifo():
    if os.path.exists(FIFO_PATH):
        os.remove(FIFO_PATH)
    os.mkfifo(FIFO_PATH)

def start_arecord():
    return subprocess.Popen([
        "arecord", "-f", "S16_LE", "-c", "1", "-r", "48000", "-D", "hw:2,0", FIFO_PATH
    ], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)

def start_ltcdump():
    return subprocess.Popen([
        "ltcdump", "-f", FIFO_PATH
    ], stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True, bufsize=1)

def clean_up_processes(*procs):
    for proc in procs:
        if proc and proc.poll() is None:
            proc.terminate()
            try:
                proc.wait(timeout=1)
            except subprocess.TimeoutExpired:
                proc.kill()
    if os.path.exists(FIFO_PATH):
        os.remove(FIFO_PATH)

def main(stdscr):
    curses.curs_set(0)
    stdscr.nodelay(True)

    setup_fifo()
    arecord_proc = start_arecord()
    ltcdump_proc = start_ltcdump()

    latest_tc = "⌛ Waiting for LTC..."
    last_update = time.time()

    try:
        while True:
            # Non-blocking read from ltcdump
            rlist, _, _ = select.select([ltcdump_proc.stdout], [], [], 0)
            if rlist:
                line = ltcdump_proc.stdout.readline()
                if line and line[0].isdigit():
                    latest_tc = line.strip()
                    last_update = time.time()

            # If stream stalls or breaks
            if time.time() - last_update > 1:
                if ltcdump_proc.poll() is not None or arecord_proc.poll() is not None:
                    latest_tc = "💥 Stream stopped or decoder crashed"
                else:
                    latest_tc = "⚠️  No LTC signal detected"

            # Draw interface
            stdscr.erase()
            stdscr.addstr(1, 2, "🌀 NTP Timeturner Status")
            stdscr.addstr(3, 4, f"Reading LTC from hw:2,0 via FIFO...")
            stdscr.addstr(5, 6, f"🕰️ LTC Timecode: {latest_tc}")
            stdscr.refresh()

            time.sleep(0.04)  # 25Hz refresh

    except KeyboardInterrupt:
        stdscr.addstr(8, 6, "🔚 Exiting gracefully...")
        stdscr.refresh()
        time.sleep(1)
    finally:
        clean_up_processes(arecord_proc, ltcdump_proc)

if __name__ == "__main__":
    if not shutil.which("ltcdump") or not shutil.which("arecord"):
        print("❌ Required tools not found (ltcdump or arecord). Install and retry.")
        exit(1)

    curses.wrapper(main)
