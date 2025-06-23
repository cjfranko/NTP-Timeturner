import json
import subprocess
import time
import threading
import os
from datetime import datetime

CONFIG_FILE = "hermione_config.json"

# Load config or create default
def load_config():
    if not os.path.exists(CONFIG_FILE):
        default_config = {
            "framerate": 25,
            "start_mode": "system",
            "manual_time": "12:00:00",
            "duration_seconds": 3600,
            "ltc_gen_path": "ltc-gen.exe",
            "autostart_timeout": 5
        }
        with open(CONFIG_FILE, "w") as f:
            json.dump(default_config, f, indent=4)
        return default_config
    else:
        with open(CONFIG_FILE, "r") as f:
            return json.load(f)

# Save updated config
def save_config(config):
    with open(CONFIG_FILE, "w") as f:
        json.dump(config, f, indent=4)

# Prompt with timeout
def prompt_with_timeout(prompt, timeout):
    print(prompt, end='', flush=True)
    input_data = []

    def get_input():
        try:
            input_data.append(input())
        except EOFError:
            pass

    thread = threading.Thread(target=get_input)
    thread.daemon = True
    thread.start()
    thread.join(timeout)
    return input_data[0] if input_data else ""

# Get timecode based on config
def get_start_time(config):
    if config["start_mode"] == "system":
        now = datetime.now()
        return now.strftime("%H:%M:%S")
    else:
        return config["manual_time"]

# Run ltc-gen
def run_ltc_gen(config):
    start_time = get_start_time(config)
    framerate = str(config["framerate"])
    duration = str(config["duration_seconds"])
    ltc_gen_path = config["ltc_gen_path"]

    cmd = [
        ltc_gen_path,
        "-f", framerate,
        "-l", duration,
        "-t", start_time
    ]

    print(f"\n🎬 Running Hermione with:")
    print(f"   Start Time: {start_time}")
    print(f"   Framerate: {framerate} fps")
    print(f"   Duration: {duration} seconds")
    print(f"   Executable: {ltc_gen_path}\n")

    try:
        subprocess.run(cmd)
    except FileNotFoundError:
        print(f"❌ Error: {ltc_gen_path} not found!")
    except Exception as e:
        print(f"❌ Failed to run Hermione: {e}")

# Main logic
def main():
    config = load_config()
    user_input = prompt_with_timeout(
        "\nPress [Enter] to run with config or type 'm' to modify (auto-starts in 5s): ",
        config.get("autostart_timeout", 5)
    )

    if user_input.lower() == 'm':
        try:
            config["framerate"] = int(input("Enter framerate (e.g. 25): "))
            config["start_mode"] = input("Start from system time or manual? (system/manual): ").strip().lower()
            if config["start_mode"] == "manual":
                config["manual_time"] = input("Enter manual start time (HH:MM:SS): ")
            config["duration_seconds"] = int(input("Enter duration in seconds: "))
            config["ltc_gen_path"] = input("Enter path to ltc-gen.exe (or leave blank for default): ") or config["ltc_gen_path"]
            save_config(config)
        except Exception as e:
            print(f"⚠️ Error updating config: {e}")
            return

    run_ltc_gen(config)

if __name__ == "__main__":
    main()
