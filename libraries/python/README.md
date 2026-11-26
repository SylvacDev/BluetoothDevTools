# Sylvac BLE library for Python

> **Preview.** Version 0.1.0. The API may still change before the first release.

Connect Sylvac Bluetooth® instruments through a Sylvac USB dongle from Python 3.10 or later, on
Windows and Linux. The package `sylvac_ble` finds the dongles, connects the instruments, keeps
them connected, and reports everything as events. It works with both Sylvac dongles:

- Bluetooth 5 SMART dongle (BL654, BT5), up to 16 instruments
- Bluetooth 4 dongle (BLED112, BT4), up to 8 instruments

## Install

Pick the wheel for your computer in [`wheels/`](wheels) and install it with pip:

| Computer | Wheel |
|---|---|
| Windows x64 | `sylvac_ble-0.1.0-cp310-abi3-win_amd64.whl` |
| Linux x86_64 | `sylvac_ble-0.1.0-cp310-abi3-manylinux2014_x86_64.manylinux_2_17_x86_64.whl` |
| Linux ARM 64-bit (e.g. Raspberry Pi 4/5, 64-bit OS) | `sylvac_ble-0.1.0-cp310-abi3-manylinux2014_aarch64.manylinux_2_17_aarch64.whl` |
| Linux ARM 32-bit (ARMv7) | `sylvac_ble-0.1.0-cp310-abi3-manylinux2014_armv7l.manylinux_2_17_armv7l.whl` |

```bash
pip install wheels/sylvac_ble-0.1.0-cp310-abi3-win_amd64.whl
```

One wheel serves every Python from 3.10 on. The package has no dependency.

On Linux the dongles appear as `/dev/ttyACM*`: install [the udev rule](../linux) and add your
user to the `dialout` group.

## Example

[`bluetooth_example.py`](bluetooth_example.py) is the Python twin of the C#
[BluetoothExample](../csharp/BluetoothExample): a window with the dongles, the connected
instruments (status, value, unit, mode, button data, response), a command sent to all of them,
and the scan. It uses the standard library only (tkinter).

```bash
python bluetooth_example.py
```

## Use

```python
import sylvac_ble
from sylvac_ble import (BleManager, ConnectionStep, ConnectionStepChanged,
                        InstrumentResponseReceived, NumericalValueChanged)

with BleManager() as ble:                      # starts the library; stops it at the end
    ble.scan.sylvac_only = True                # list Sylvac instruments only
    ble.scan.start()
    ble.start_connect("D216182F6CCC")          # address in the BT > MAC menu of the instrument
    for event in ble.events():                 # Ctrl+C ends the loop
        match event:
            case ConnectionStepChanged(peripheral=mac, value=ConnectionStep.SUBSCRIBED):
                ble.peripheral(mac).send_command("ID?")
            case NumericalValueChanged(value=value, measurement_params=params):
                print(params.format(value), params.unit.name)
            case InstrumentResponseReceived(value=text, command=command):
                print(command, "->", text)
```

## How it works

The library works in the background. When you ask it to do something, such as connecting an
instrument or sending a command, the call returns straight away; the result arrives later as an
event. Instruments also send events on their own: a new value, a press on the Data button, a
lost connection.

You can read the events the way that suits your program: one at a time with `next_event()`, in
a `for` loop with `ble.events()`, with `async for` in asyncio code, or by giving a function to
`set_callback()`. If you would rather wait for a result, `connect()`, `disconnect()` and
`send_and_wait()` block until it comes (each has an `_async` version too).

Every call is documented: try `help(sylvac_ble.BleManager)`.

Writing a window? Read the events from your GUI's own thread. `bluetooth_example.py` shows how,
with tkinter.

The wheels include open-source components; see
[THIRD-PARTY-NOTICES.txt](../c/THIRD-PARTY-NOTICES.txt).
