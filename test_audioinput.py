#!/usr/bin/env python3

"""
test_audioinput.py
Quick sanity check to ensure audio input device is working.
Records 2 seconds of audio from default input and plots waveform.
"""

import numpy as np
import matplotlib.pyplot as plt
import sounddevice as sd

DURATION = 2  # seconds
SAMPLERATE = 48000
CHANNELS = 1

print("🔊 Recording 2 seconds from default input device...")
recording = sd.rec(int(DURATION * SAMPLERATE), samplerate=SAMPLERATE, channels=CHANNELS, dtype='float32')
sd.wait()

# Generate time axis
time_axis = np.linspace(0, DURATION, len(recording))

# Plot
plt.figure(figsize=(10, 4))
plt.plot(time_axis, recording, linewidth=0.5)
plt.title("Audio Input Waveform")
plt.xlabel("Time [s]")
plt.ylabel("Amplitude")
plt.grid(True)
plt.tight_layout()
plt.show()
