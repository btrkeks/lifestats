# Bedtime Watch

A Wear OS tile for the Pixel Watch that logs bedtime to Lifestats (`就寝時間`, statistic 20) with one tap.

## How it works

- Tapping **Log now** on the tile opens a confirmation screen that logs the current time and closes after 3 seconds. Opening the app from the app list only shows the last bedtime, so a stray launch logs nothing.
- The bedtime is filed under the evening's date: anything before 12:00 belongs to the previous evening, so 00:24 on Oct 14 becomes `2026-10-13 = 00:24`.
- The time is saved on the watch first. A WorkManager job uploads it with `update_policy: overwrite` and retries with backoff until the server answers, so the logged time is the tap time even if the upload happens the next morning. Logging again the same evening replaces the earlier value.
- The tile shows the last bedtime and whether it was sent, is waiting to send, or was rejected (4xx).

## Network

The watch tries each URL in `lifestats.urls` (`gradle.properties`) in order:

1. `http://192.168.68.74:3000`: the laptop on the home LAN. This is the path at bedtime, when the phone is off and the watch is on its own Wi-Fi.
2. `https://laptop-1.tail87da75.ts.net`: the tailnet. There is no Tailscale on Wear OS, so this only works through the phone's connection.

If the laptop's LAN address changes, reserve it in the router or change `lifestats.urls` and rebuild.

## Build and install

```bash
./gradlew testDebugUnitTest assembleDebug
```

On the watch: Settings → System → About → tap *Build number* 7 times, then Settings → Developer options → enable *ADB debugging* and *Wireless debugging*. Use *Pair new device* there:

```bash
adb pair <watch-ip>:<pair-port>        # enter the pairing code shown on the watch
adb connect <watch-ip>:<debug-port>
adb -s <watch-ip>:<debug-port> install -r app/build/outputs/apk/debug/app-debug.apk
```

Then long-press the tiles carousel → add the **Bedtime** tile.

## Testing on an emulator

Build with emulator URLs (`10.0.2.2` is the host), run a side API on a DB copy, and add the tile through the debug surface:

```bash
./gradlew assembleDebug -Plifestats.urls=http://10.0.2.2:3010
adb shell am broadcast -a com.google.android.wearable.app.DEBUG_SURFACE \
  --es operation add-tile --ecn component dev.lifestats.bedtime/.BedtimeTileService --ei index 0
adb logcat -s BedtimeUpload
```

Emulators start "charging" and show a charging screen over the tiles; `adb shell dumpsys battery unplug` hides it.
