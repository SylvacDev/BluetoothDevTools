# Sylvac libraries

The recommended way to connect Sylvac Bluetooth® instruments to your software. The libraries drive a Sylvac USB dongle, so you can connect instruments, read their live values and send them commands, without any Bluetooth programming.

They work with both Sylvac dongles, and the same code runs with either of them:

- Bluetooth 5 SMART dongle, BL654 module (BT5), ref. 981-7102 and 981-7103: up to 16 instruments at the same time
- Bluetooth 4 dongle, BLED112 module (BT4), ref. 981-7100: up to 8 instruments at the same time

| Folder | Language | Platforms | Content |
|---|---|---|---|
| [csharp](csharp) | C# / .NET 10 | Windows | `SylvacBleLib` and the BluetoothExample WPF application |
| [python](python) | Python 3.10+ | Windows x64, Linux (x86_64, ARM) | `sylvac_ble` package (wheels) and `bluetooth_example.py`, the Python twin of BluetoothExample (preview) |
| [rust](rust) | Rust | Windows x64, Linux (x86_64, ARM) | Crates over the compiled library (preview) |
| [c](c) | C, C++, any language that calls a C library | Windows x64, Linux (x86_64, ARM) | Compiled library `sylvac_ble` and its header (preview) |

The Python, C and Rust libraries share one compiled library.

On Linux, install the [udev rule](linux) for the dongles.

All libraries offer the same features:

1. list the Sylvac dongles plugged in the computer,
2. scan for Sylvac instruments,
3. connect and disconnect instruments,
4. read the live value, the unit and the measuring mode (normal, min, max, delta),
5. receive the value sent with the instrument button,
6. send a command to the instrument and receive its answer.
