#!/usr/bin/env python3

import curses
import subprocess
import time
import shutil
import fcntl
import os
import errno
from datetime import datetime

MAX_LOG_LINES = 5

def set_nonblocking(fileobj):
    fd = fileobj.fileno()
    flags = fcntl.fcntl(fd, fcntl.F_GETFL)
    fcntl.fcntl(fd, fcntl.F_SETFL, flags | os.O_NONBLOCK)

def nonblocking_readline(output):
    try:
        return output.readline()
    except IOError as e:
        if e.errno == errno.EAGAIN:
            return ""
        else:
            raise

def frame_to_int(line):
    try:
        return int(line.strip().split(":")[-1])
    except Exception:
        return None

def start_ltc_stream(log_lines):
    try:
        ffmpeg = subprocess.Popen(
            ["ffmpeg", "-f", "alsa", "-i", "default", "-ac", "1", "-ar", "48000", "-f", "s16le", "-"],
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL
        )
        ltcdump = subprocess.Popen(
            ["ltcdump", "-f", "-"],
            stdin=ffmpeg.stdout,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True
        )
        ffmpeg.stdout.close()
        set_nonblocking(ltcdump.stdout)
        set_nonblocking(ltcdump.stderr)
        log_lines.append(f"[{datetime.now().strftime('%H:%M:%S')}] ✅ Subprocesses started successfully")
        return ffmpeg, ltcdump
    except Exception as e:
        log_lines.append(f"[{datetime.now().strftime('%H:%M:%S')}] ❌ Failed to start subprocesses: {e}")
        return None, None

def main(stdscr):
    curses.curs_set(0)
    stdscr.nodelay(True)

    log_lines = []
    ffmpeg_proc, ltcdump_proc = start_ltc_stream(log_lines)

    last_tc = "⌛ Waiting for LTC..."
    last_frame = None
    fail_count = 0

    while True:
        # Check for subprocess failure
        if ffmpeg_proc and ffmpeg_proc.poll() is not None:
            log_lines.append(f"[{datetime.now().strftime('%H:%M:%S')}] ❌ ffmpeg exited (code {ffmpeg_proc.returncode})")
            ffmpeg_proc = None
        if ltcdump_proc and ltcdump_proc.poll() is not None:
            err_output = ltcdump_proc.stderr.read().strip()
            log_lines.append(f"[{datetime.now().strftime('%H:%M:%S')}] ❌ ltcdump exited (code {ltcdump_proc.returncode})")
            if err_output:
                log_lines.append(err_output.splitlines()[-1])
            ltcdump_proc = None

        # Read LTC line if possible
        if ltcdump_proc:
            try:
                line = nonblocking_readline(ltcdump_proc.stdout).strip()
                if line and line[0].isdigit():
                    current_frame = frame_to_int(line)
                    if last_frame is not None and current_frame is not None:
                        delta = abs(current_frame - last_frame)
                        if delta > 3:
                            log_lines.append(f"[{datetime.now().strftime('%H:%M:%S')}] ⚠️ Timecode jump: Δ{delta} frames")
                    last_tc = line
                    last_frame = current_frame
                elif line:
                    fail_count += 1
            except Exception as e:
                log_lines.append(f"[{datetime.now().strftime('%H:%M:%S')}] ⚠️ Read error: {e}")
                fail_count += 1

        log_lines = log_lines[-MAX_LOG_LINES:]

        # Draw UI
        stdscr.clear()
        stdscr.addstr(1, 2, "🌀 NTP Timeturner Status")
        stdscr.addstr(3, 4, "Streaming LTC from default input..." if ltcdump_proc else "⚠️ LTC decoder inactive")

        stdscr.addstr(5, 6, f"🕰️ LTC Timecode: {last_tc}")
        stdscr.addstr(6, 6, f"❌ Decode Failures: {fail_count}")

        stdscr.addstr(8, 4, "📜 Logs:")
        for i, log in enumerate(log_lines):
            stdscr.addstr(9 + i, 6, log[:curses.COLS - 8])

        stdscr.refresh()
        time.sleep(1)

if __name__ == "__main__":
    if not shutil.which("ltcdump") or not shutil.which("ffmpeg"):
        print("❌ Required tools not found (ltcdump or ffmpeg). Install and retry.")
        exit(1)

    curses.wrapper(main)
