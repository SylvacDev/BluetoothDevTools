# Sylvac Bluetooth Example for Android (preview)

A small Android app that connects Sylvac Bluetooth® instruments with the **built-in Bluetooth of the
phone or tablet**, without dongle and without Sylvac library. It talks to the instruments directly
with the Android framework Bluetooth API and the [Sylvac GATT profiles](../../../gatt-profiles).

Use it as a reference to see, in a few files of plain Kotlin, how to:

- scan for nearby Sylvac instruments and read the value they advertise,
- connect one or many instruments, pairing them when they ask for it,
- read the live measured value, the unit, the resolution and the measuring mode,
- receive the value sent when the operator presses the instrument button,
- send a command to the instrument (`?`, `ID?`, `VER?`...) and read its answer,
- reconnect an instrument after it went out of range or Bluetooth was switched off and on.

## Requirements

| Item | Version |
|---|---|
| Phone or tablet | Android 13 (API 33) or later, Bluetooth Low Energy |
| JDK | 17 |
| Android SDK | platform `android-36`, build-tools 36 |
| Gradle | 8.14, through the wrapper (`gradlew` / `gradlew.bat`) |
| Android Gradle Plugin / Kotlin | 8.12.0 / 2.1.20 |

Android Studio is not needed: the Gradle wrapper builds the app from the command line. You can also
open this folder in Android Studio.

Android 13 is the minimum on purpose: Bluetooth then only needs the `BLUETOOTH_SCAN` and
`BLUETOOTH_CONNECT` permissions (no location permission), and the GATT calls take their values
directly, without the deprecated variants of older versions.

## Build and run

On the phone, enable **Developer options → USB debugging**, plug it in and accept the prompt.
`adb devices` must list the phone as `device`.

```bash
./gradlew installDebug                                    # build and install
adb shell am start -n com.example.bluetooth/.MainActivity # launch
adb logcat -s SylvacBle:I '*:S' -v time                   # app log: connection steps, commands, errors
```

On Windows, use `gradlew.bat` instead of `./gradlew`. `./gradlew assembleDebug` only builds the APK, in
`app/build/outputs/apk/debug/`.

Handy for testing:

```bash
# Grant the Bluetooth permissions without the dialog
adb shell pm grant com.example.bluetooth android.permission.BLUETOOTH_SCAN
adb shell pm grant com.example.bluetooth android.permission.BLUETOOTH_CONNECT

# Restart the app and connect these instruments (Bluetooth addresses without colons)
adb shell am start -S -n com.example.bluetooth/.MainActivity --es connect AABBCCDDEEFF,112233445566

# UI test without instruments: 50 fake nearby instruments, 13 fake connected ones
adb shell am start -S -n com.example.bluetooth/.MainActivity --ei simNearby 50 --ei simConnected 13
```

The package name is `com.example.bluetooth`: change `namespace` and `applicationId` in
`app/build.gradle.kts` (and the package of the sources) when you start your own app from this one.

## Using the app

- **Add instrument** opens the scanner: the instruments advertising nearby, with their live value. Tap
  one to connect it; the scanner stays open to add more. Swipe its header down to close it.
- Each connected instrument has a row with its status, its large live value and unit, the last value
  sent with its button, its mode and its battery level.
- Tap a row to open the instrument's console: quick commands (**Read value** `?`, **ID** `ID?`,
  **Version** `VER?`), a free command field, and the last answer.
- Swipe a row left to remove the instrument (with Undo).

## Code layout

```
settings.gradle.kts, build.gradle.kts, gradle.properties   Gradle project, no dependencies
app/build.gradle.kts                                       android { } settings
app/src/main/AndroidManifest.xml                           Bluetooth permissions
app/src/main/java/com/example/bluetooth/
    Sylvac.kt               UUIDs of the Sylvac services and characteristics, value decoders
    SylvacInstrument.kt     one GATT connection per instrument: setup, commands, reconnection
    Advert.kt               what an instrument's advertising data says
    MainActivity.kt         screen, permissions, Bluetooth on / off, scanning
    InstrumentRow.kt        one connected instrument in the list
    InstrumentConsole.kt    bottom sheet with the commands of one instrument
    InstrumentSummary.kt    status, name and value, shared by the row and the console
    NearbyTile.kt           one nearby instrument in the scanner
    ui/                     generic view code, nothing Sylvac-specific
    simulation/             fake instruments for UI tests
```

Start with `Sylvac.kt` and `SylvacInstrument.kt`: they hold all the Bluetooth code that talks to the
instrument. The rest is user interface.

## Sylvac Bluetooth essentials

The full description is in the [GATT profiles folder](../../../gatt-profiles). What the app relies on:

| Item | Value |
|---|---|
| Advertised name | `SY…` (Sylvac) or `MTY…` (OEM). A bonded instrument in PAIR profile shortens it to `SY`. |
| Advertising flags | `0x06` SIMPLE, `0x05` PAIR not bonded, `0x04` PAIR bonded. HID names end with `HID`. |
| Service data `0x5000` | SINT32 LE, displayed value x 10^-4, no unit. Service data `0x180A` = article number. |
| No service UUID in the advertising | Scan without `ScanFilter`, filter on the name or the service data in the callback. |
| SDS service | `00005000-0000-1000-8000-00805F9B34FB` |
| Measurement `0x5020` | Notify, SINT32 LE, unit and exponent in its `0x2904` descriptor. `0x7FFFFFFF` / `0x80000000` = invalid. |
| Parameters `0x5021` | Notify, UINT16: unit (bits 15-12), resolution (11-8), mode (1-0). |
| SMS service | `C1B25000-CAAF-6D0E-4C33-7DAE30052840` |
| DataSend `…5010` | Indicate, ASCII + CR: value sent with the instrument button. |
| RemoteRequest `…5012` | Write **without response** only, ASCII + CR, chunks of 20 bytes. |
| RemoteResponse `…5013` | Notify, ASCII, an answer ends with CR. |
| Pairing | SIMPLE profile: none. PAIR profile: Just Works bonding, the characteristics need encryption. |

## How the app talks to an instrument

- **Scan**: low-latency scan without filter, keeping the devices named `SY…` / `MTY…` or advertising
  `0x5000` service data. The scan only runs while the scanner is open or an instrument is reconnecting,
  and is restarted every 4 minutes because Android silently stops delivering results to a long scan.
- **Connection setup** (`SylvacInstrument`): one `BluetoothGatt` per instrument, all GATT operations
  queued one at a time on the main thread with a 10 s timeout: connect → bond first if the flags say
  PAIR → discover the services → read the `0x2904` descriptor and the battery → subscribe to
  Parameters, Measurement, RemoteResponse and DataSend → ready.
- **Pairing**: a GATT status 5, 15 or 137 (insufficient authentication or encryption) starts
  `createBond()` and retries the operation.
- **Value**: `raw x 10^exponent` metres, shown in mm, inch or degrees according to Parameters, with the
  Parameters resolution as number of decimals.
- **Commands**: written to RemoteRequest; the answer arrives on RemoteResponse. A command without
  answer within 3 s shows `no reply`.
- **Reconnection**: after an unexpected disconnection, a background `autoConnect` plus a direct
  connection as soon as the scanner sees the instrument advertise again (an idle instrument advertises
  only every few seconds, and `autoConnect` alone can take minutes). A link that stops answering
  within 10 s is dropped and reconnected. Android reports no disconnection when Bluetooth is switched
  off, so the app follows `BluetoothAdapter.ACTION_STATE_CHANGED` to drop and later reconnect its links.

## Support

This example is a preview, provided as is. Native development on Android is not supported by Sylvac,
see [Support](../../../README.md#support).
