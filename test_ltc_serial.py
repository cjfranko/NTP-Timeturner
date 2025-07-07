import serial
import re

# Adjust this as needed
SERIAL_PORT = "/dev/ttyACM0"
BAUD_RATE = 115200

# Regex pattern to match: [LOCK] 10:00:00:00 | 24.00FPS
ltc_pattern = re.compile(r"\[(LOCK|FREE)\]\s+(\d{2}:\d{2}:\d{2}:\d{2})\s+\|\s+([\d.]+FPS)")

def main():
    print(f"🔌 Connecting to serial port: {SERIAL_PORT} @ {BAUD_RATE} baud")
    try:
        with serial.Serial(SERIAL_PORT, BAUD_RATE, timeout=1) as ser:
            print("📡 Listening for LTC messages...\n")
            while True:
                line = ser.readline().decode(errors='ignore').strip()
                match = ltc_pattern.match(line)
                if match:
                    status, timecode, framerate = match.groups()
                    if status == "LOCK":
                        print(f"🔒 {status}  |  ⏱ {timecode}  |  🎞 {framerate}")
                    else:
                        print(f"🟡 {status} |  ⏱ {timecode}  |  🎞 {framerate}")
                else:
                    if line:
                        print(f"⚠️ Unrecognised line: {line}")
    except serial.SerialException as e:
        print(f"❌ Serial error: {e}")
    except KeyboardInterrupt:
        print("\n🛑 Stopped by user.")

if __name__ == "__main__":
    main()
