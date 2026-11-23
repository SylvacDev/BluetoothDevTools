# Sylvac Bluetooth Developer Tools

Everything you need to connect Sylvac Bluetooth® instruments (calipers, indicators, micrometers, height gauges and more) to your own software.

With these tools your application can:

- discover Sylvac instruments nearby and connect one or many of them,
- read the live measured value, the unit and the measuring mode,
- receive the value sent when the operator presses the instrument button,
- send commands to the instrument and read its answer.

## Choose your path

<p align="center"><img src="docs/developer-paths.svg" width="900" alt="Recommended path: Windows or Linux, Sylvac libraries (C# / .NET 10 on Windows, Rust library called from Python on Windows and Linux), Bluetooth 5 SMART dongle BL654 (BT5, 981-7102 / 981-7103, up to 16 instruments) or Bluetooth 4 dongle BLED112 (BT4, 981-7100, up to 8 instruments). Low level, on Windows or Linux: BGAPI SDK for the BLED112 (BT4). Without dongle: native Bluetooth API or cross-platform framework with the Sylvac GATT profiles. All paths use the Sylvac custom Bluetooth GATT profile: Sylvac Metrology Service (SMS) and Simple Data Service (SDS)."></p>

| You want to... | Use | Dongles | Platforms |
|---|---|---|---|
| **Connect instruments through a Sylvac dongle (recommended)** | [Sylvac libraries](libraries) | BL654 (BT5) and BLED112 (BT4) | C# / .NET: Windows. Rust and Python library (coming soon): Windows, Linux |
| Use the built-in Bluetooth of a phone, tablet, Mac or PC, without dongle | [Sylvac GATT profiles](gatt-profiles) and the [list of third-party Bluetooth APIs and libraries (PDF)](docs/MAN-DeveloperResources_OS.pdf) | None | Android, iOS, macOS, Linux, Windows |
| Drive compatible dongles with the BGAPI protocol | [Sylvac GATT profiles](gatt-profiles) and the [BGAPI SDK](dongles/BLED112) | BLED112 (BT4) | Windows, Linux, and more |
| Maintain an existing integration based on the C++ communication DLL | [C++ communication DLL](legacy/cpp-dll) (legacy, 2019) | BLED112 (BT4), or built-in Bluetooth on Windows 10 | Windows |

The [Sylvac libraries](libraries) hide all the Bluetooth details, and the same code runs with the BT5 and the BT4 dongles. To get started in C#, follow the [library guide](libraries/csharp/README.md) or run the [ready-to-run example](libraries/csharp/BluetoothExample.zip).

If you program the Bluetooth link yourself, the [GATT profiles folder](gatt-profiles) sums up how Sylvac instruments talk Bluetooth and gives the reading order of the specifications.

## Sylvac USB dongles

Sylvac sells two USB dongles, both supported by the libraries: the **Bluetooth 5 SMART dongle** (BL654, BT5, up to 16 instruments) and the **Bluetooth 4 dongle** (BLED112, BT4, up to 8 instruments). References and details in the [dongles folder](dongles).

## Repository content

| Folder | Content |
|---|---|
| [`libraries/`](libraries) | Sylvac libraries: C# / .NET, and Rust called from Python (coming soon) |
| [`dongles/`](dongles) | Sylvac USB dongles and the Bluegiga BGAPI SDK |
| [`gatt-profiles/`](gatt-profiles) | Bluetooth profile, SDS and SMS specifications |
| [`legacy/cpp-dll/`](legacy/cpp-dll) | C++ communication DLL v1.45, headers and API reference |

## Support

Sylvac supports its libraries, dongles and GATT profiles. Native development on other platforms is not supported by Sylvac, but the specifications in this repository contain everything needed. To get in touch, go to [sylvac.ch](https://www.sylvac.ch/data-transmission-software/).

## License

See [LICENSE](LICENSE). Third-party components keep their own license.
