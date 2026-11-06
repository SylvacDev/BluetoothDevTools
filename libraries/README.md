# Sylvac libraries

The recommended way to connect Sylvac Bluetooth® instruments to your software. The libraries talk to a Sylvac USB dongle and give you instruments, live values and commands, without any Bluetooth programming.

They work with both Sylvac dongles, and the same code runs with either of them:

- Bluetooth 5 SMART dongle, BL654 module (BT5), ref. 981-7102 and 981-7103: up to 16 instruments at the same time
- Bluetooth 4 dongle, BLED112 module (BT4), ref. 981-7100: up to 8 instruments at the same time

| Folder | Language | Platforms | Content |
|---|---|---|---|
| [csharp](csharp) | C# / .NET | Windows | SylvacBleLib and the BluetoothExample WPF application |
| rust | Rust, Python | Windows, Linux | Compiled Rust library with its Python interface, Rust and Python examples (coming soon) |

All libraries offer the same features:

1. list the Sylvac dongles plugged in the computer,
2. scan for Sylvac instruments,
3. connect and disconnect instruments,
4. read the live value, the unit and the measuring mode,
5. receive the value sent with the instrument button,
6. send a command to the instrument and receive its answer.
