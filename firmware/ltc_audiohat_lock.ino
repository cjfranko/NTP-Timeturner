/* Linear Timecode for Audio Library for Teensy 3.x / 4.x
   Copyright (c) 2019, Frank Bösing, f.boesing (at) gmx.de

   Development of this audio library was funded by PJRC.COM, LLC by sales of
   Teensy and Audio Adaptor boards.  Please support PJRC's efforts to develop
   open source software by purchasing Teensy or other PJRC products.

   Permission is hereby granted, free of charge, to any person obtaining a copy
   of this software and associated documentation files (the "Software"), to deal
   in the Software without restriction, including without limitation the rights
   to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
   copies of the Software, and to permit persons to whom the Software is
   furnished to do so, subject to the following conditions:

   The above copyright notice, development funding notice, and this permission
   notice shall be included in all copies or substantial portions of the Software.

   THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
   IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
   FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
   AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
   LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
   OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
   THE SOFTWARE.
*/

/*

 https://forum.pjrc.com/threads/41584-Audio-Library-for-Linear-Timecode-(LTC)

 LTC example audio at: https://www.youtube.com/watch?v=uzje8fDyrgg

 Adapted by Chris Frankland-Wright 2025 for Teensy Audio Shield Input with autodetect FPS for the NTP-TimeTurner Project
 
*/

#include <Arduino.h>
#include <Audio.h>
#include "analyze_ltc.h"

// —— Configuration ——
// 0.0 → auto-detect; or force 24.0, 25.0, 29.97
const float FORCE_FPS       = 0.0f;
// frame-delay compensation (in frames)
const int   FRAME_OFFSET    = 4;
// how many frame-periods to wait before declaring “lost”
const float LOSS_THRESHOLD_FRAMES = 1.5f;
// Blink periods (ms) for NO_LTC, ACTIVE, LOST
const unsigned long BLINK_PERIOD[3] = { 2000, 100, 500 };

AudioInputI2S        i2s1;
AudioAnalyzeLTC      ltc1;
AudioControlSGTL5000 sgtl5000;
AudioConnection      patchCord(i2s1, 0, ltc1, 0);

enum State { NO_LTC = 0, LTC_ACTIVE, LTC_LOST };
State         ltcState    = NO_LTC;
bool          ledOn       = false;
unsigned long lastDecode  = 0;
unsigned long lastBlink   = 0;

// auto-detect vars
float        currentFps    = 25.0f;
float        periodMs      = 0;
const float  SMOOTH_ALPHA  = 0.1f;
unsigned long lastDetectTs = 0;

// free-run tracking
long         freeAbsFrame  = 0;
unsigned long lastFreeRun  = 0;

void setup() {
  Serial.begin(115200);
 // while (!Serial);
  AudioMemory(12);
  sgtl5000.enable();
  sgtl5000.inputSelect(AUDIO_INPUT_LINEIN);
  pinMode(LED_BUILTIN, OUTPUT);
}

void loop() {
  unsigned long now = millis();
  // compute dynamic framePeriod (ms) from last known fps
  unsigned long framePeriod = (unsigned long)(1000.0f/currentFps + 0.5f);

  if (ltc1.available()) {
    // —— LOCKED —— read a frame
    ltcframe_t frame = ltc1.read();
    int h = ltc1.hour(&frame),
        m = ltc1.minute(&frame),
        s = ltc1.second(&frame),
        f = ltc1.frame(&frame);

    // —— FPS detect or force ——
    if (FORCE_FPS > 0.0f) {
      currentFps = FORCE_FPS;
    } else {
      if (lastDetectTs) {
        float dt = now - lastDetectTs;
        periodMs = periodMs==0 ? dt : (SMOOTH_ALPHA*dt + (1-SMOOTH_ALPHA)*periodMs);
        float measured = 1000.0f/periodMs;
        const float choices[3] = {24.0f,25.0f,29.97f};
        float bestD=1e6, pick=25.0f;
        for (auto c: choices) {
          float d = fabs(measured - c);
          if (d < bestD) { bestD = d; pick = c; }
        }
        currentFps = pick;
      }
      lastDetectTs = now;
    }

    // —— pack + offset + wrap ——
    int nominal    = (currentFps>29.5f)?30:int(currentFps+0.5f);
    long dayFrames = 24L*3600L*nominal;
    long absF = ((long)h*3600 + m*60 + s)*nominal + f + FRAME_OFFSET;
    absF = (absF % dayFrames + dayFrames) % dayFrames;

    // save for free-run
    freeAbsFrame = absF;
    lastFreeRun  = now;

    // unpack for display
    long totSec = absF/nominal;
    int outF    = absF % nominal;
    int outS    = totSec % 60;
    long totMin = totSec/60;
    int outM    = totMin % 60;
    int outH    = (totMin/60)%24;

    // dynamic drop-frame from bit 10
    bool isDF = ltc1.bit10(&frame);
    char sep  = isDF ? ';' : ':';

    // print locked
    Serial.printf("[LOCK] %02d:%02d:%02d%c%02d | %.2ffps\r\n",
                  outH,outM,outS,sep,outF,currentFps);

    // update state
    ltcState   = LTC_ACTIVE;
    lastDecode = now;
  }
  else {
    // —— NOT LOCKED —— check if we should switch to free-run
    if (ltcState == LTC_ACTIVE) {
      // only switch after losing more than LOSS_THRESHOLD_FRAMES
      float elapsedFrames = float(now - lastDecode) / float(framePeriod);
      if (elapsedFrames >= LOSS_THRESHOLD_FRAMES) {
        ltcState = LTC_LOST;
        // free-run will begin below
      }
    }
  }

  // —— FREE-RUN —— when lost
  if (ltcState == LTC_LOST) {
    if ((now - lastFreeRun) >= framePeriod) {
      freeAbsFrame = (freeAbsFrame + 1) % (24L*3600L*(int)(currentFps+0.5f));
      lastFreeRun += framePeriod;

      long totSec = freeAbsFrame/((int)(currentFps+0.5f));
      int outF    = freeAbsFrame % (int)(currentFps+0.5f);
      int outS    = totSec % 60;
      long totMin = totSec/60;
      int outM    = totMin % 60;
      int outH    = (totMin/60)%24;

      Serial.printf("[FREE] %02d:%02d:%02d:%02d | %.2ffps\r\n",
                    outH,outM,outS,outF,currentFps);
    }
  }

  // —— LED heartbeat —— non-blocking
  unsigned long period = BLINK_PERIOD[ltcState];
  if (now - lastBlink >= period/2) {
    ledOn = !ledOn;
    digitalWrite(LED_BUILTIN, ledOn);
    lastBlink = now;
  }
}
