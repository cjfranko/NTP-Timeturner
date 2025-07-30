# NTP Timeturner API

This document describes the HTTP API for the NTP Timeturner application.

## Endpoints

### Status

- **`GET /api/status`**

  Retrieves the real-time status of the LTC reader and system clock synchronization.

  **Example Response:**
  ```json
  {
    "ltc_status": "LOCK",
    "ltc_timecode": "10:20:30:00",
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
    "hardware_offset_ms": 0
  }
  ```

### Sync

- **`POST /api/sync`**

  Triggers a manual synchronization of the system clock to the current LTC timecode. This requires the application to have `sudo` privileges to execute the `date` command.

  **Request Body:** None

  **Success Response:**
  ```json
  {
    "status": "success",
    "message": "Sync command issued."
  }
  ```

  **Error Responses:**
  ```json
  {
    "status": "error",
    "message": "No LTC timecode available to sync to."
  }
  ```
  ```json
  {
    "status": "error",
    "message": "Sync command failed."
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

  **Success Response:**
  ```json
  {
    "status": "success",
    "message": "Date update command issued."
  }
  ```

  **Error Response:**
  ```json
  {
    "status": "error",
    "message": "Date update command failed."
  }
  ```

### Configuration

- **`GET /api/config`**

  Retrieves the current application configuration.

  **Example Response:**
  ```json
  {
    "hardware_offset_ms": 0
  }
  ```

- **`POST /api/config`**

  Updates the `hardware_offset_ms` configuration. The new value is persisted to `config.json` and reloaded by the application automatically.

  **Example Request:**
  ```json
  {
    "hardware_offset_ms": 10
  }
  ```

  **Success Response:**
  ```json
  {
    "hardware_offset_ms": 10
  }
  ```
