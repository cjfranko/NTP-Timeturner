import subprocess
import time
import shutil

# Check tools
if not shutil.which("ltcdump") or not shutil.which("ffmpeg"):
    print("❌ Required tools not found. Please ensure ffmpeg and ltcdump are installed.")
    exit(1)

print("🕰️  Starting LTC timecode reader (refreshes every second)...\n")

try:
    while True:
        # Capture 1 second of audio and pipe into ltcdump
        ffmpeg = subprocess.Popen(
            ["ffmpeg", "-f", "alsa", "-i", "hw:1", "-t", "1", "-f", "s16le", "-ac", "1", "-ar", "48000", "-"],
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL
        )
        ltcdump = subprocess.Popen(
            ["ltcdump", "-f", "-"],
            stdin=ffmpeg.stdout,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL
        )
        ffmpeg.stdout.close()
        output, _ = ltcdump.communicate()

        # Extract and print LTC timecode
        lines = output.decode().splitlines()
        if lines:
            print(f"\r⏱️  LTC: {lines[-1]}", end="")
        else:
            print("\r⚠️  No LTC decoded...", end="")

        time.sleep(1)

except KeyboardInterrupt:
    print("\n🛑 Stopped by user.")
