# Using BGAPI to connect Sylvac instruments with Bluetooth dongles

> **Important:** this SDK is for driving directly the Bluetooth dongles sold by Sylvac. It is a low-level access to the dongle through the Bluegiga BGAPI protocol.
>
> Most projects do not need it. The [Sylvac libraries](../../libraries) are the recommended way: they work with both the Bluetooth 5 SMART dongle (BL654 module, BT5, ref. 981-7102) and the Bluetooth 4 dongle (BLED112, BT4), without needing advanced Bluetooth knowledge.

## 1. Sylvac custom Bluetooth GATT profiles

You should be familiar with the [Sylvac Bluetooth GATT profiles and services](../../gatt-profiles/README.md).

## 2. Development steps

Once you have enough background and are ready to start developing, begin with the following documents:

- Step-by-step single connection to a Sylvac Bluetooth instrument: [MEM-PM 292-1806-04 Bluetooth Profile Quick Start Guide (PDF)](<MEM-PM 292-1806-04-Bluetooth Profile Quick Start Guide.pdf>)
- Reference manual for BGAPI: [Bluegiga Bluetooth Low Energy Software V.1.10 API Documentation (PDF)](<API doc/Bluetooth-LE-Software-API Reference-Manual-for-BLE-Version-1.10.pdf>)

## 3. Develop on Windows or Linux

Using the Silicon Labs (formerly Bluegiga) BLED112 Bluetooth Low Energy dongle, with the BGAPI: see the folders [API doc](<API doc>) and [ble-1.10.0-153](ble-1.10.0-153).

The dongle is detected as a virtual serial port, and can be used on any OS that supports USB CDC ACM devices. It appears as `COMx` on Windows, `/dev/ttyACMx` on Linux.

## Note

Bluegiga SDK original installation files can be downloaded on the Silicon Labs website: <https://www.silabs.com/wireless/bluetooth>, see legacy modules.

The Sylvac variant of the SDK is unpacked here and provided with BLED112 dongle firmware compiled for 8 simultaneous connections (instead of 3).