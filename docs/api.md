# NTP Timeturner API

This document describes the HTTP API for the NTP Timeturner application.

## Endpoints

### Status and Logs

- **`GET /api/status`**

  Retrieves the real-time status of the LTC reader and system clock synchronization. The `ltc_timecode` field uses `:` as a separator for non-drop-frame timecode, and `;` for drop-frame timecode between seconds and frames (e.g., `10:20:30;00`).

  **Possible values for status fields:**
  - `ltc_status`: `"LOCK"`, `"FREE"`, or `"(waiting)"`
  - `sync_status`: `"IN SYNC"`, `"CLOCK AHEAD"`, `"CLOCK BEHIND"`, `"TIMETURNING"`
  - `jitter_status`: `"GOOD"`, `"AVERAGE"`, `"BAD"`

  **Example Response:**
  ```json
  {
    "ltc_status": "LOCK",
    "ltc_timecode": "10:20:30;00",
    "frame_rate": "25.00fps",
    "system_clock": "10:20:30.005",
    "system_date": "2025-07-30",
    "timecode_delta_ms": 5,
    "timecode_delta_frames": 0,
    "sync_status": "IN SYNC",
    "jitter_status": "GOOD",
    "lock_ratio": 99.5,
    "ntp_active": true,
    "interfaces": ["192.168.1.100"],
    "hardware_offset_ms": 20
  }
  ```

- **`GET /api/logs`**

  Retrieves the last 100 log entries from the application.

  **Example Response:**
  ```json
  [
    "2025-08-07 10:00:00 [INFO] Starting TimeTurner daemon...",
    "2025-08-07 10:00:01 [INFO] Found serial port: /dev/ttyACM0"
  ]
  ```

### System Clock Control

- **`POST /api/sync`**

  Triggers a manual synchronization of the system clock to the current LTC timecode. This requires the application to have `sudo` privileges to execute the `date` command.

  **Request Body:** None

  **Success Response (200 OK):**
  ```json
  {
    "status": "success",
    "message": "Sync command issued."
  }
  ```

  **Error Response (400 Bad Request):**
  ```json
  {
    "status": "error",
    "message": "No LTC timecode available to sync to."
  }
  ```
  **Error Response (500 Internal Server Error):**
  ```json
  {
    "status": "error",
    "message": "Sync command failed."
  }
  ```

- **`POST /api/nudge_clock`**

  Nudges the system clock by a specified number of microseconds. This requires `sudo` privileges to run `adjtimex`.

  **Example Request:**
  ```json
  {
    "microseconds": -2000
  }
  ```
  **Success Response (200 OK):**
  ```json
  {
    "status": "success",
    "message": "Clock nudge command issued."
  }
  ```
  **Error Response (500 Internal Server Error):**
  ```json
  {
    "status": "error",
    "message": "Clock nudge command failed."
  }
  ```


- **`POST /api/set_date`**

  Sets the system date. This is useful as LTC does not contain date information. Requires `sudo` privileges.

  **Example Request:**
  ```json
  {
    "date": "2025-07-30"
  }
  ```

  **Success Response (200 OK):**
  ```json
  {
    "status": "success",
    "message": "Date update command issued."
  }
  ```

  **Error Response (500 Internal Server Error):**
  ```json
  {
    "status": "error",
    "message": "Date update command failed."
  }
  ```

### Configuration

- **`GET /api/config`**

  Retrieves the current application configuration from `config.yml`.

  **Example Response (200 OK):**
  ```json
  {
    "hardwareOffsetMs": 20,
    "timeturnerOffset": {
      "hours": 0,
      "minutes": 0,
      "seconds": 0,
      "frames": 0,
      "milliseconds": 0
    },
    "defaultNudgeMs": 2,
    "autoSyncEnabled": false
  }
  ```

- **`POST /api/config`**

  Updates the application configuration. The new configuration is persisted to `config.yml` and takes effect immediately.

  **Example Request:**
  ```json
  {
    "hardwareOffsetMs": 55,
    "timeturnerOffset": {
      "hours": 1,
      "minutes": 2,
      "seconds": 3,
      "frames": 4,
      "milliseconds": 5
    },
    "defaultNudgeMs": 2,
    "autoSyncEnabled": true
  }
  ```

  **Success Response (200 OK):** (Returns the updated configuration)
  ```json
  {
    "hardwareOffsetMs": 55,
    "timeturnerOffset": {
      "hours": 1,
      "minutes": 2,
      "seconds": 3,
      "frames": 4,
      "milliseconds": 5
    },
    "defaultNudgeMs": 2,
    "autoSyncEnabled": true
  }
  ```
  **Error Response (500 Internal Server Error):**
  ```json
  {
    "status": "error",
    "message": "Failed to write config.yml"
  }
  ```
