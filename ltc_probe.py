#!/usr/bin/env python3

"""
ltc_probe.py
Advanced LTC-like signal probe using pulse duration clustering
for reliable short/long classification — works even with imbalanced timecodes.
"""

import numpy as np
import sounddevice as sd

DURATION = 1.0  # seconds
SAMPLERATE = 48000
CHANNELS = 1
MIN_EDGES = 1000

def detect_rising_edges(signal):
    above_zero = signal > 0
    edges = np.where(np.logical_and(~above_zero[:-1], above_zero[1:]))[0]
    return edges

def cluster_durations(durations):
    if len(durations) < 2:
        return None, None

    # Use 2-means clustering (basic method)
    durations = np.array(durations)
    mean1, mean2 = np.min(durations), np.max(durations)
    
    for _ in range(10):  # converge in a few iterations
        group1 = durations[np.abs(durations - mean1) < np.abs(durations - mean2)]
        group2 = durations[np.abs(durations - mean1) >= np.abs(durations - mean2)]
        if len(group1) == 0 or len(group2) == 0:
            break
        mean1 = np.mean(group1)
        mean2 = np.mean(group2)
    
    short = group1 if mean1 < mean2 else group2
    long = group2 if mean1 < mean2 else group1
    return short, long

def analyze_pulse_durations(edges, samplerate):
    durations = np.diff(edges) / samplerate
    if len(durations) == 0:
        return None

    short, long = cluster_durations(durations)
    if short is None or long is None:
        return None

    total = len(durations)
    return {
        "count": total,
        "avg_width_ms": np.mean(durations) * 1000,
        "short_pulses": len(short),
        "long_pulses": len(long),
        "short_pct": (len(short) / total) * 100,
        "long_pct": (len(long) / total) * 100
    }

def verdict(pulse_data):
    if pulse_data is None or pulse_data["count"] < MIN_EDGES:
        return "❌ No signal or not enough pulses"
    elif 10 <= pulse_data["short_pct"] <= 90:
        return f"✅ LTC-like bi-phase signal detected ({pulse_data['count']} pulses)"
    else:
        return f"⚠️ Pulse imbalance suggests non-LTC or noisy signal"

def main():
    print("🔍 Capturing 1 second of audio for LTC probing...")
    audio = sd.rec(int(DURATION * SAMPLERATE), samplerate=SAMPLERATE, channels=CHANNELS, dtype='float32')
    sd.wait()

    signal = audio.flatten()
    edges = detect_rising_edges(signal)
    pulse_data = analyze_pulse_durations(edges, SAMPLERATE)

    print(f"\n📊 Pulse Analysis:")
    if pulse_data:
        print(f"  Total pulses:      {pulse_data['count']}")
        print(f"  Avg pulse width:   {pulse_data['avg_width_ms']:.2f} ms")
        print(f"  Short pulses:      {pulse_data['short_pulses']} ({pulse_data['short_pct']:.1f}%)")
        print(f"  Long pulses:       {pulse_data['long_pulses']} ({pulse_data['long_pct']:.1f}%)")
    else:
        print("  Not enough data to analyze.")

    print("\n🧭 Verdict:")
    print(" ", verdict(pulse_data))

if __name__ == "__main__":
    main()
