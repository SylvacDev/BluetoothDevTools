# Sylvac BLE library for Rust

> **Preview.** Version 0.1.0. The API may still change before the first release.

Connect Sylvac Bluetooth® instruments through a Sylvac USB dongle from a Rust program, on
Windows and Linux. It works with both Sylvac dongles:

- Bluetooth 5 SMART dongle (BL654, BT5), up to 16 instruments
- Bluetooth 4 dongle (BLED112, BT4), up to 8 instruments

| Crate | What it is |
|---|---|
| [`sylvac-ble`](sylvac-ble) | the API: `BleManager`, `BlePeripheral`, `Event` |
| [`sylvac-ble-sys`](sylvac-ble-sys) | raw bindings to the C library, used by `sylvac-ble` |

The two crates are open source (MIT) and have no dependency. They link the compiled Sylvac
library of [`../c`](../c), the same one the C and Python libraries use. Rust 1.82 or later.

## Build

Point `SYLVAC_BLE_SDK_DIR` at the [`../c`](../c) folder of this repository: the build picks the
library of your target in its sub-folder (`x86_64-pc-windows-msvc`, `x86_64-unknown-linux-gnu`,
`aarch64-unknown-linux-gnu`, `armv7-unknown-linux-gnueabihf`).

```toml
[dependencies]
sylvac-ble = { path = "path/to/BluetoothDevTools/libraries/rust/sylvac-ble" }
```

```bash
export SYLVAC_BLE_SDK_DIR=/path/to/BluetoothDevTools/libraries/c       # Linux
set SYLVAC_BLE_SDK_DIR=C:\path\to\BluetoothDevTools\libraries\c        # Windows
cargo run --example quickstart
```

`cargo run` and `cargo test` find the library by themselves. A program started otherwise needs
`sylvac_ble.dll` next to the `.exe` (Windows), or `libsylvac_ble.so.0` on its loader path
(Linux: `LD_LIBRARY_PATH`, or next to the program when linked with `-Wl,-rpath,$ORIGIN`).

On Linux the dongles appear as `/dev/ttyACM*`: install [the udev rule](../linux) and add your
user to the `dialout` group.

## Use

```rust
use std::time::Duration;
use sylvac_ble::{mac, BleConfig, BleManager, ConnectionStep, Event};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ble = BleManager::start(BleConfig::default())?;
    let scan = ble.scan();

    // List Sylvac instruments only
    scan.set_sylvac_only(true)?;
    scan.start()?;

    // Bluetooth address of the instrument (BT > MAC menu)
    ble.start_connect(mac!("D216182F6CCC"))?;

    while let Some(event) = ble.next_event(Duration::from_secs(30)) {
        match event {
            Event::ConnectionStepChanged { peripheral, value: ConnectionStep::Subscribed } => {
                ble.peripheral(peripheral).send_command("ID?")?;
            }
            Event::NumericalValueChanged { value, measurement_params: p, .. } => {
                println!("{} {:?}", p.format(value), p.unit);
            }
            Event::InstrumentResponseReceived { value, command, .. } => {
                println!("{command:?} -> {value}");
            }
            _ => {}
        }
    }
    ble.stop();
    Ok(())
}
```

## How it works

The library works in the background, on its own thread. When you ask it to do something, such as
connecting an instrument or sending a command, the call returns straight away; the result
arrives later as an event. Instruments also send events on their own: a new value, a press on
the Data button, a lost connection.

You can take the events one at a time with `next_event()`, as above, or hand a closure to
`set_callback()`. The closure runs on the library's thread, so keep it short. If you would
rather wait for a result, `connect()`, `disconnect()` and `send_and_wait()` block until it
comes. `BleManager` and `BlePeripheral` can be shared between threads.

Use the crates and the compiled library from the same release: if they don't match,
`BleManager::start` fails with `Error::AbiMismatch`.

## Example

[`sylvac-ble/examples/quickstart.rs`](sylvac-ble/examples/quickstart.rs) finds the dongle, lists
the instruments around (nearest first, with battery and advertised value) and connects the one
you choose. It then asks it `ID?` and `?`, shows its value live, and tells you when the
connection drops and comes back.

```bash
cargo run --example quickstart                    # choose among the instruments heard
cargo run --example quickstart -- D216182F6CCC    # connect this one
```
