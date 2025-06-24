#!/usr/bin/env python3

"""
ltc_probe.py
Probes audio input to detect LTC-like signal characteristics.
Reports zero crossings and estimated frequency.
"""

import numpy as np
import sounddevice as sd

DURATION = 1.0  # seconds
SAMPLERATE = 48000
CHANNELS = 1
EXPECTED_FREQ = 2000  # Approx LTC edge rate at 25fps

def count_zero_crossings(signal):
    signal = signal.flatten()
    signs = np.sign(signal)
    return np.count_nonzero(np.diff(signs))

def verdict(freq):
    if freq < 100:
        return "❌ No signal detected (flatline or silence)"
    elif 1800 <= freq <= 2200:
        return f"✅ LTC-like signal detected (freq: {freq:.1f} Hz)"
    else:
        return f"⚠️ Unstable or non-LTC signal (freq: {freq:.1f} Hz)"

def main():
    print("🔍 Capturing 1 second of audio for LTC probing...")
    audio = sd.rec(int(DURATION * SAMPLERATE), samplerate=SAMPLERATE, channels=CHANNELS, dtype='float32')
    sd.wait()

    crossings = count_zero_crossings(audio)
    estimated_freq = crossings / DURATION

    print(f"Zero crossings: {crossings}")
    print(f"Estimated frequency: {estimated_freq:.1f} Hz")
    print(verdict(estimated_freq))

if __name__ == "__main__":
    main()
