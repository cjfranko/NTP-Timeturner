#!/usr/bin/env python3

"""
timeturner.py
NTP Timeturner Core UI
Displays LTC signal probe info using curses, updated in real-time.
"""

import curses
import threading
import time
import numpy as np
import sounddevice as sd

# --- CONFIGURATION ---
SAMPLERATE = 48000
CHANNELS = 1
PROBE_INTERVAL = 1.0  # seconds
MIN_EDGES = 1000

status = {
    "count": 0,
    "avg_width_ms": 0.0,
    "short_pct": 0.0,
    "long_pct": 0.0,
    "verdict": "Waiting for signal...",
}


# --- LTC PROBE THREAD ---
def detect_rising_edges(signal):
    above_zero = signal > 0
    edges = np.where(np.logical_and(~above_zero[:-1], above_zero[1:]))[0]
    return edges

def cluster_durations(durations):
    if len(durations) < 2:
        return None, None

    durations = np.array(durations)
    mean1, mean2 = np.min(durations), np.max(durations)

    for _ in range(10):
        group1 = durations[np.abs(durations - mean1) < np.abs(durations - mean2)]
        group2 = durations[np.abs(durations - mean1) >= np.abs(durations - mean2)]
        if len(group1) == 0 or len(group2) == 0:
            break
        mean1 = np.mean(group1)
        mean2 = np.mean(group2)

    short = group1 if mean1 < mean2 else group2
    long = group2 if mean1 < mean2 else group1
    return short, long

def analyze_signal():
    global status
    while True:
        try:
            audio = sd.rec(int(PROBE_INTERVAL * SAMPLERATE), samplerate=SAMPLERATE,
                           channels=CHANNELS, dtype='float32')
            sd.wait()
            signal = audio.flatten()
            edges = detect_rising_edges(signal)
            durations = np.diff(edges) / SAMPLERATE
            short, long = cluster_durations(durations)

            if short is None or long is None or len(durations) < MIN_EDGES:
                status["verdict"] = "❌ No signal or not enough pulses"
                continue

            status.update({
                "count": len(durations),
                "avg_width_ms": np.mean(durations) * 1000,
                "short_pct": (len(short) / len(durations)) * 100,
                "long_pct": (len(long) / len(durations)) * 100,
                "verdict": "✅ LTC-like signal detected" if 10 <= (len(short) / len(durations)) * 100 <= 90
                            else "⚠️ Pulse imbalance — possible noise or non-LTC"
            })
        except Exception as e:
            status["verdict"] = f"⚠️ Audio error: {e}"

# --- CURSES UI ---
def draw_ui(stdscr):
    curses.curs_set(0)
    stdscr.nodelay(True)
    stdscr.timeout(500)

    while True:
        stdscr.clear()
        stdscr.addstr(0, 2, "🕰️  NTP Timeturner - Live LTC Monitor", curses.A_BOLD)
        stdscr.addstr(2, 4, f"Pulses captured:     {status['count']}")
        stdscr.addstr(3, 4, f"Avg pulse width:     {status['avg_width_ms']:.2f} ms")
        stdscr.addstr(4, 4, f"Short pulse ratio:   {status['short_pct']:.1f}%")
        stdscr.addstr(5, 4, f"Long pulse ratio:    {status['long_pct']:.1f}%")
        stdscr.addstr(7, 4, f"Status:              {status['verdict']}")
        stdscr.addstr(9, 4, "Press Ctrl+C to exit.")
        stdscr.refresh()

        try:
            time.sleep(1)
        except KeyboardInterrupt:
            break

# --- ENTRY POINT ---
if __name__ == "__main__":
    probe_thread = threading.Thread(target=analyze_signal, daemon=True)
    probe_thread.start()
    curses.wrapper(draw_ui)
