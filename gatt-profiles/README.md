# Sylvac custom Bluetooth GATT profiles

Specifications of the custom Bluetooth® GATT profiles of Sylvac instruments. You only need them if you program the Bluetooth link yourself, with a native API, a cross-platform framework or the [BGAPI SDK](../dongles/BLED112) of the BLED112 dongle. The [Sylvac libraries](../libraries) do it for you.

## How Sylvac instruments talk Bluetooth

- Sylvac instruments advertise a device name starting with `SY`. OEM-compatible instruments use `MTY`.
- **Sylvac Simple Data Service (SDS)** for receiving the stream of measured values. See [below](#sylvac-simple-data-service-sds).
- **Sylvac Metrology Service (SMS)** for receiving a measurement when the instrument button is pressed, and for sending Sylvac commands. See [below](#sylvac-metrology-service-sms).
- Both services have the short UUID `0x5000`. Only their base UUID tells them apart.
- Sylvac base UUID: `C1B2xxxx-CAAF-6D0E-4C33-7DAE30052840`.
- SMS strings are plain ASCII terminated by a carriage return (`0x0D`), sent in chunks of 20 bytes at most.
- The instrument can be configured in SIMPLE (no pairing), PAIR, or HID (keyboard) profile.
- Enabling notifications and indications slightly increases the current consumption of the instrument. Enable them only on the characteristics you use.

The instruments also expose the standard GAP (`0x1800`), GATT (`0x1801`), Device Information (`0x180A`) and Battery (`0x180F`) services of the Bluetooth SIG.

## Sylvac Simple Data Service (SDS)

The SDS transmits the measured value and its parameters (unit, resolution, measuring mode).

The SDS is not a service adopted by the Bluetooth SIG. It uses 16-bit UUIDs on the Bluetooth SIG base UUID `0000xxxx-0000-1000-8000-00805F9B34FB` to keep the advertising packet small: the measured value is also broadcast in the [advertising data](#advertising-data).

| Characteristic | UUID | Full UUID | Properties | Format |
|---|---|---|---|---|
| *Service* | `0x5000` | `00005000-0000-1000-8000-00805F9B34FB` | | |
| SDS Measurement | `0x5020` | `00005020-0000-1000-8000-00805F9B34FB` | Notify | SINT32, unit given by the Characteristic Presentation Format descriptor (`0x2904`) |
| SDS Parameters | `0x5021` | `00005021-0000-1000-8000-00805F9B34FB` | Notify | UINT16 bitmap |

### SDS Measurement

The value is a little-endian signed 32-bit integer, in the unit and exponent given by the *Characteristic Presentation Format* descriptor (`0x2904`). The client converts it to the display unit and resolution given by the *SDS Parameters* characteristic.

Example:

| Step | Value |
|---|---|
| Bytes received | `A0 06 3A 01` |
| Descriptor unit | `0x2701` (metre) |
| Descriptor exponent | `0xF7` (-9) |
| Little-endian value | `0x013A06A0` = 20'580'000 |
| Measured value | 20'580'000 × 10⁻⁹ m = 0.02058 m (20.58 mm) |

Enable notifications on *SDS Measurement*. A read without notification returns the non-valid value `0x7FFFFFFF` (bytes `FF FF FF 7F`, 2'147'483'647).

### SDS Parameters

| Bits | Field | Values |
|---|---|---|
| 15–12 | Unit | `0000` undefined, `0001` mm, `0010` inch, `0100` radians, `0101` degrees, `0110` degrees-minutes, others reserved |
| 11–8 | Resolution | `0000` undefined, `0001` 100 µm / 5 mil, `0010` 10 µm / 0.5 mil / 0.01° / 0.0001 rad, `0011` 1 µm / 0.05 mil, `0100` 0.1 µm / 0.005 mil, `0101` 0.01 µm, others reserved |
| 7–2 | Reserved | Always 0 |
| 1–0 | Measuring mode | `00` undefined, `01` minimum, `10` maximum, `11` delta / TIR |

Example: `0x1302` means mm, resolution 1 µm, maximum mode.

## Sylvac Metrology Service (SMS)

The SMS replaces the Sylvac cable: it carries the same plain ASCII commands. The command list of each instrument is in its user manual.

The SMS and its characteristics use 128-bit UUIDs on the Sylvac base UUID `C1B2xxxx-CAAF-6D0E-4C33-7DAE30052840`.

| Characteristic | UUID | Full UUID | Properties | Use |
|---|---|---|---|---|
| *Service* | `0x5000` | `C1B25000-CAAF-6D0E-4C33-7DAE30052840` | | |
| SMS Data (DataSend) | `0x5010` | `C1B25010-CAAF-6D0E-4C33-7DAE30052840` | Indicate, optional Read | Value sent when the instrument button is pressed |
| (DataSet) | `0x5011` | `C1B25011-CAAF-6D0E-4C33-7DAE30052840` | Read | Obsolete, not implemented in new instruments |
| SMS Request (RemoteRequest) | `0x5012` | `C1B25012-CAAF-6D0E-4C33-7DAE30052840` | Write without response | Command sent to the instrument |
| SMS Response (RemoteResponse) | `0x5013` | `C1B25013-CAAF-6D0E-4C33-7DAE30052840` | Notify | Answer from the instrument |
| SMS Free 1 | `0x5014` | `C1B25014-CAAF-6D0E-4C33-7DAE30052840` | Write without response, Read | User data |
| SMS Free 2 | `0x5015` | `C1B25015-CAAF-6D0E-4C33-7DAE30052840` | Write without response, Read | User data |
| SMS Free 3 | `0x5016` | `C1B25016-CAAF-6D0E-4C33-7DAE30052840` | Write without response, Read | User data |

All characteristics follow the security level of the active Bluetooth profile: no security with SIMPLE, paired link with PAIR.

### Data format

- All characteristics use plain ASCII text.
- On *SMS Data*, *SMS Request* and *SMS Response* (`0x5010`, `0x5012`, `0x5013`), strings end with a carriage return (`0x0D`).
- A characteristic can handle 20 bytes at most. Send longer strings in several transfers: the string is incomplete until the carriage return.
- Examples: *SMS Data* `+001.234\r`, *SMS Request* `UNI?\r`, *SMS Response* `MM\r`.
- *SMS Free 1* to *3* (`0x5014` to `0x5016`) trigger no action. They store user data on the instrument, for example an identifier. The values are kept in non-volatile memory, are not validated, and are reset on any Bluetooth profile change or factory reset.

### Sylvac Bluetooth commands

These are text commands, terminated by a carriage return, that are sent to the *SMS Request* characteristic.

| Command | Description |
|---|---|
| `CFG BT SIMPLE` | Restart with the SIMPLE profile (no pairing, default) |
| `CFG BT PAIR` | Restart with the PAIR profile (pairing) |
| `CFG BT HID` | Restart with the HID profile (keyboard emulation, pairing) |
| `CFG BT ?` | Query the current profile |
| `SVR1` / `SVR0` / `SVR?` | Saver mode on / off / state. When on, the instrument sends the value on *SMS Response* only when the displayed value changes. Turned off at each disconnection |
| `BT0` | Turn Bluetooth off, without erasing the bond |
| `BTRST` | Bluetooth reset: erases the bond (when paired) and closes the connection |

The SIMPLE, PAIR and HID profiles are available from firmware 4.xx onwards. Pairing is lost on every profile change. See [Sylvac BT Smart additional features (PDF)](Various/MEM-PM%20292-1538-01-Sylvac%20BT%20Smart%20additional%20features.pdf) for the saver, energy efficient and advertisement idle modes.

## Advertising data

| AD type | AD data |
|---|---|
| Flags (`0x01`) | *SIMPLE profile*: `0x06` (general discoverable). *PAIR profile*, not paired yet: `0x05` (limited discoverable). *PAIR profile*, already paired: `0x04` (non-discoverable) |
| Complete local name (`0x09`), or Shortened local name (`0x08`) when paired | Device name, for example `SY289`. Shortened to `SY` when paired |
| Service data (`0x16`), UUID `0x180A` | DIS (Device Information Service) *Model Number String*: Sylvac part number |
| Service data (`0x16`), UUID `0x5000` | SDS (Simple Data Service) *Measurement*: little-endian signed 32-bit integer, in display units × 10⁴ |

Example of advertised *SDS Measurement*: bytes `E8 23 03 00` → `0x000323E8` = 205'800 → 205'800 × 10⁻⁴ = 20.58

The broadcast measurement helps identify which instrument to connect to when several are nearby. It is not meant for measuring without a connection.

The service data is not available with the HID profile. Sylvac software uses passive scan (recommended) and ignores scan responses.

## Reading order

| # | Document | Ref. |
|---|---|---|
| 1 | [Sylvac Bluetooth Profile Specifications (PDF)](MEM-PM%20292-1806-01-Bluetooth%20Profile%20Specifications.pdf) | MEM-PM 292-1806-01 |
| 2 | [Sylvac SDS Specification (PDF)](Spec/SPE-PM%20292-1744-01%20SDS%20Specification.pdf) | SPE-PM 292-1744-01 |
| 3 | [Sylvac SMS Specification (PDF)](Spec/SPE-PM%20292-1746-01%20SMS%20Specification.pdf) | SPE-PM 292-1746-01 |
| 4 | [Sylvac Bluetooth Profile Recommendation For Use (PDF)](MEM-PM%20292-1806-03-Bluetooth%20Profile%20Recommendation%20For%20Use.pdf) | MEM-PM 292-1806-03 |
| 5 | [Sylvac Bluetooth Profile Compatibility (PDF)](MEM-PM%20292-1806-02-Bluetooth%20Profile%20Compatibility.pdf) | MEM-PM 292-1806-02 |

Additional information:

- [Sylvac BT Smart additional features (PDF)](Various/MEM-PM%20292-1538-01-Sylvac%20BT%20Smart%20additional%20features.pdf) (MEM-PM 292-1538-01)
- [Sylvac user manual extract, Bluetooth chapters (PDF)](Various/User%20manual%20extract%20-%20Bluetooth%20chapters.pdf)
- [Version release notes](Version%20release%20notes.txt) of these specifications

## Background

- [Sylvac Bluetooth use](https://www.sylvac.ch/data-transmission-software/)
- [Sylvac instruments](https://www.sylvac.ch/product/digital-indicators-s_dial-work-smart/)
- [Sylvac Bluetooth dongle](https://www.sylvac.ch/product/dongle-smart/)

## Support

Sylvac supports the above profiles. Native APIs, cross-platform frameworks and third-party Bluetooth dongles are documented by their providers and not supported by Sylvac. See the [list of third-party Bluetooth APIs and libraries (PDF)](../docs/MAN-DeveloperResources_OS.pdf).

To choose between the development options, see [Choose your path](../README.md#choose-your-path).
