#!/usr/bin/env python3

"""
ltc_probe.py
Improved LTC-like signal probe — detects pulse duration patterns
consistent with bi-phase mark code used in SMPTE LTC.
"""

import numpy as np
import sounddevice as sd

DURATION = 1.0  # seconds
SAMPLERATE = 48000
CHANNELS = 1
MIN_EDGES = 1000  # sanity threshold

def detect_rising_edges(signal):
    # signal: flattened 1D numpy array
    above_zero = signal > 0
    edges = np.where(np.logical_and(~above_zero[:-1], above_zero[1:]))[0]
    return edges

def analyze_pulse_durations(edges, samplerate):
    durations = np.diff(edges) / samplerate  # in seconds
    if len(durations) == 0:
        return None

    short_pulse_threshold = np.median(durations) * 1.5
    short = durations[durations <= short_pulse_threshold]
    long = durations[durations > short_pulse_threshold]

    return {
        "count": len(durations),
        "avg_width_ms": np.mean(durations) * 1000,
        "short_pulses": len(short),
        "long_pulses": len(long),
        "short_pct": (len(short) / len(durations)) * 100,
        "long_pct": (len(long) / len(durations)) * 100
    }

def verdict(pulse_data):
    if pulse_data is None or pulse_data["count"] < MIN_EDGES:
        return "❌ No signal or not enough pulses"
    elif 30 < pulse_data["short_pct"] < 70:
        return f"✅ LTC-like bi-phase signal detected ({pulse_data['count']} pulses)"
    else:
        return f"⚠️ Inconsistent signal — may be non-LTC or noisy"

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
