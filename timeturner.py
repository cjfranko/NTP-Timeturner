import numpy as np
import sounddevice as sd
import scipy.signal as signal
import matplotlib.pyplot as plt
import time

# --- Configuration ---
SAMPLE_RATE = 48000
CUTOFF_FREQ = 1000.0
BLOCK_SIZE = 2048
SYNC_WORD = "0011111111111101"

# --- Filtering ---
def highpass_filter(data, cutoff=CUTOFF_FREQ, fs=SAMPLE_RATE, order=5):
    nyq = 0.5 * fs
    normal_cutoff = cutoff / nyq
    b, a = signal.butter(order, normal_cutoff, btype='high', analog=False)
    return signal.lfilter(b, a, data)

# --- Edge Detection ---
def detect_edges(data):
    return np.where(np.diff(np.signbit(data)))[0]

# --- Pulse Width to Bitstream ---
def classify_bits(edges, fs):
    durations = np.diff(edges) / fs
    threshold = np.median(durations) * 1.2
    bits = ['1' if dur < threshold else '0' for dur in durations]
    return bits, durations, threshold

# --- LTC Sync & Decode ---
def extract_ltc_frame(bitstream):
    bits = ''.join(bitstream)
    idx = bits.find(SYNC_WORD)
    if idx != -1 and idx + 80 <= len(bits):
        return bits[idx:idx+80], idx
    return None, None

def decode_timecode(bits):
    def bcd(b): return int(b[0:4], 2) + 10 * int(b[4:8], 2)
    frames = bcd(bits[0:8])
    seconds = bcd(bits[16:24])
    minutes = bcd(bits[32:40])
    hours = bcd(bits[48:56])
    return f"{hours:02}:{minutes:02}:{seconds:02}:{frames:02}"

# --- Stream Callback ---
def process(indata, frames, time_info, status):
    audio = indata[:, 0]
    filtered = highpass_filter(audio)

    peak_db = 20 * np.log10(np.max(np.abs(filtered)) + 1e-6)
    print(f"🎚️  Input Level: {peak_db:.2f} dB")

    edges = detect_edges(filtered)
    print(f"📎 Found {len(edges)} edges.")

    if len(edges) < 10:
        return

    bits, durations, threshold = classify_bits(edges, SAMPLE_RATE)
    print(f"📊 Avg pulse width: {np.mean(durations):.5f} sec")
    print(f"📊 Min: {np.min(durations):.5f}, Max: {np.max(durations):.5f}")
    print(f"🔧 Adaptive threshold: {threshold:.5f} sec")
    print(f"🧮 Extracted {len(bits)} bits.")
    print(f"🧾 Bitstream (first 80 bits):\n{''.join(bits[:80])}")

    frame, idx = extract_ltc_frame(bits)
    if frame:
        print(f"🔓 Sync word found at bit {idx}")
        print(f"✅ LTC Timecode: {decode_timecode(frame)}")
    else:
        print(f"⚠️ No valid LTC frame detected.")

# --- Main ---
print("🔊 Starting real-time audio stream...")

try:
    with sd.InputStream(callback=process, channels=1, samplerate=SAMPLE_RATE, blocksize=BLOCK_SIZE):
        while True:
            time.sleep(0.5)
except KeyboardInterrupt:
    print("🛑 Stopped by user.")
