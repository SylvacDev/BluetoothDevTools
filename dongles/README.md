# Sylvac USB dongles

Sylvac sells two USB dongles to connect Sylvac Bluetooth® instruments to a computer. Both are supported by the [Sylvac libraries](../libraries), so the same software works with either of them.

| Folder | Dongle | Bluetooth | Inside | Sylvac ref. | Instrument connections |
|---|---|---|---|---|---|
| [BL654](BL654/README.md) | Bluetooth 5 SMART dongle | **Bluetooth 5** (BT5) | BL654 (Ezurio, formerly Laird Connectivity) | 981-7102 | up to 16, long range mode available |
| [BLED112](BLED112/README.txt) | Bluetooth 4 dongle | **Bluetooth 4.0** (BT4) | BLED112 (Silicon Labs, formerly Bluegiga) | 981-7100 | up to 8 |

Both dongles appear as a virtual serial port: `COMx` on Windows, `/dev/ttyACMx` on Linux. More information on the [dongle product page](https://www.sylvac.ch/product/dongle-smart/).

The [BLED112 folder](BLED112) also contains the Bluegiga BGAPI SDK and tools, for low-level communication with the Bluetooth dongle.
