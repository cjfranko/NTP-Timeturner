import serial
import re

# Adjust as needed
SERIAL_PORT = "/dev/ttyACM0"
BAUD_RATE = 115200

# Updated pattern to match drop-frame (;) and non-drop (:) timecode
ltc_pattern = re.compile(
    r"\[(LOCK|FREE)\]\s+(\d{2}:\d{2}:\d{2}[:;]\d{2})\s+\|\s+([\d.]+fps)", re.IGNORECASE
)

def main():
    print(f"[INFO] Connecting to serial port: {SERIAL_PORT} @ {BAUD_RATE} baud")
    try:
        with serial.Serial(SERIAL_PORT, BAUD_RATE, timeout=1) as ser:
            print("[INFO] Listening for LTC messages...\n")
            while True:
                line = ser.readline().decode(errors='ignore').strip()
                match = ltc_pattern.match(line)
                if match:
                    status, timecode, framerate = match.groups()
                    framerate = framerate.upper()
                    if status == "LOCK":
                        print(f"[LOCK] {status:<4} | Timecode: {timecode} | Rate: {framerate}")
                    else:
                        print(f"[FREE] {status:<4} | Timecode: {timecode} | Rate: {framerate}")
                else:
                    if line:
                        print(f"[WARN] Unrecognised line: {line}")
    except serial.SerialException as e:
        print(f"[ERROR] Serial error: {e}")
    except KeyboardInterrupt:
        print("\n[EXIT] Stopped by user.")

if __name__ == "__main__":
    main()
