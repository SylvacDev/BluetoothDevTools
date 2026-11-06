# Sylvac Bluetooth Developer Tools

Everything you need to connect Sylvac Bluetooth® instruments (calipers, indicators, micrometers, height gauges and more) to your own software.

With these tools your application can:

- discover Sylvac instruments around it and connect one or many of them,
- read the live measured value, the unit and the measuring mode,
- receive the value sent when the operator presses the instrument button,
- send commands to the instrument (for example `ID?`) and read its answer.

## Choose your path

| You want to... | Use | Platform |
|---|---|---|
| Build a Windows application in C# / .NET (recommended) | [SylvacBleLib](csharp) with a Sylvac USB dongle | Windows |
| Talk to the instruments with the native Bluetooth stack of any platform (Android, iOS, macOS, Linux, Windows, embedded) | [Sylvac custom Bluetooth GATT profiles](gatt-profiles) and the [list of native APIs and libraries](docs/MAN-DeveloperResources_OS.pdf) | Any |
| Drive the dongle at a low level with the BGAPI serial protocol | [BGAPI dongle SDK](bgapi-dongle-sdk) | Any OS with a serial port |
| Maintain an existing integration based on the C++ communication DLL | [C++ communication DLL](legacy/cpp-dll) (legacy, 2019) | Windows |

![Which resource to use](https://github.com/SylvacDev/BluetoothDevTools/assets/111576217/c834d79d-0fbe-49f9-a4f8-99c445f60ca8)

## Quick start in C#

A complete WPF example is available in [csharp](csharp), together with a [ready-to-run build](csharp/BluetoothExample.zip). The essentials look like this:

```cs
using SylvacBleLib;

var ble = BleManager.Instance;
ble.Start();

// Dongles plugged in the computer
ble.BleCentrals.ListChanged += (s, e) => { /* e.AddedItems, e.RemovedItems */ };

// Scan for Sylvac instruments only
ble.BleScanManager.SylvacOnly = true;
ble.BleScanManager.Advertisers.ListChanged += (s, e) => { /* found instruments */ };
ble.BleScanManager.StartScan();

// Connect an instrument found by the scan
dongle.StartConnect(advItem.Mac, out BlePeripheral instrument);

// Live values, button presses and commands
instrument.NumericalValueChanged += (s, e) => { /* instrument.NumericalValue */ };
instrument.InstrumentButtonPressed += (s, e) => { /* e.Value */ };
instrument.InstrumentResponseReceived += (s, e) => { /* e.Value */ };
instrument.SendCommand("ID?");
```

The [library guide](csharp/README.md) explains installation, threading and every step in detail. SylvacBleLib is still in beta and its API may change.

## Hardware

SylvacBleLib, the C++ DLL and the BGAPI SDK need a Sylvac USB dongle. Native Bluetooth development only needs the Bluetooth radio of your device.

Sylvac offers two USB dongles, both supported by SylvacBleLib and the BGAPI SDK:

- **Bluetooth 5 SMART dongle** (981-7102, and 981-7103 with external antenna): up to 16 instruments at the same time, long range mode.
- **Bluetooth 4 dongle** (BLED112, 981-7100): up to 8 instruments at the same time.

Both dongles speak the same BGAPI serial protocol, so the same software works with either of them.

More information on the [dongle product page](https://www.sylvac.ch/product/dongle-smart/).

## How Sylvac instruments talk Bluetooth

Sylvac instruments advertise with a device name starting with `SY` (OEM instruments use `MTY`). Once connected they expose standard services (Device Information, Battery) and two Sylvac services:

- **Simple Data Service (SDS)**: the measured value, also broadcast in the advertising data.
- **Sylvac Metrology Service (SMS)**: a text channel to send commands and receive answers, in plain ASCII terminated by a carriage return.

The instrument can be configured in SIMPLE (no pairing), PAIR or HID (keyboard) profile. Start with [the README of the GATT profiles folder](gatt-profiles/README.txt), which gives the reading order of the specifications.

## Repository content

```
csharp/              SylvacBleLib (.NET) and the BluetoothExample WPF application
gatt-profiles/       Bluetooth profile, SDS and SMS specifications (PDF)
bgapi-dongle-sdk/    BGAPI SDK and tools (Silicon Labs Bluegiga), dongle documentation
docs/                Native Bluetooth APIs and libraries for other platforms
legacy/cpp-dll/      C++ communication DLL v1.45, headers and HTML API reference
```

## Support

Sylvac supports SylvacBleLib, the C++ communication DLL, the dongles and the GATT profiles. Native development on other platforms is not supported by Sylvac, but the specifications in this repository contain everything needed. To get in touch, go to [sylvac.ch](https://www.sylvac.ch/data-transmission-software/).

## License

See [LICENSE](LICENSE). Third-party components keep their own license.
