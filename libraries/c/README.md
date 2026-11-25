# Sylvac BLE library for C and C++

> **Preview.** Version 0.1.0 (C ABI 21). The API may still change before the first release.

A compiled library to connect Sylvac Bluetooth® instruments through a Sylvac USB dongle from C,
C++ or any language that can call a C library (C#, Delphi, LabVIEW, Python `ctypes`...). It
finds the dongles, connects the instruments, keeps them connected, and reports everything as
events. It works with both Sylvac dongles:

- Bluetooth 5 SMART dongle (BL654, BT5), up to 16 instruments
- Bluetooth 4 dongle (BLED112, BT4), up to 8 instruments

The same library is behind the [Python package](../python) and the [Rust crates](../rust).

## Content

| File | What it is |
|---|---|
| `include/sylvac_ble.h` | the header: every function, event and structure, documented |
| `x86_64-pc-windows-msvc/sylvac_ble.dll`, `sylvac_ble.dll.lib` | Windows x64: the library and its import library |
| `x86_64-unknown-linux-gnu/libsylvac_ble.so.0` | Linux x86_64 (glibc 2.17 or later) |
| `aarch64-unknown-linux-gnu/libsylvac_ble.so.0` | Linux ARM 64-bit, e.g. Raspberry Pi 4/5 with a 64-bit OS |
| `armv7-unknown-linux-gnueabihf/libsylvac_ble.so.0` | Linux ARM 32-bit (ARMv7, hard float) |

## Build

Windows (Visual Studio Developer Command Prompt, x64):

```bat
cl /W4 /I include my_program.c x86_64-pc-windows-msvc\sylvac_ble.dll.lib
```

Put `sylvac_ble.dll` next to your `.exe`.

Linux:

```bash
cc -I include my_program.c -L x86_64-unknown-linux-gnu -l:libsylvac_ble.so.0 -Wl,-rpath,'$ORIGIN'
```

Put `libsylvac_ble.so.0` next to the program (thanks to the `rpath`), or in a folder of
`LD_LIBRARY_PATH`.

On Linux the dongles appear as `/dev/ttyACM*`: install [the udev rule](../linux) and add your
user to the `dialout` group.

## Use

```c
#include <stdio.h>
#include "sylvac_ble.h"

int main(void) {
    sylvac_ble_config config;
    sylvac_ble_config_init(&config);
    config.sylvac_only = 1;                 /* list Sylvac instruments only */

    sylvac_ble_manager *ble;
    if (sylvac_ble_manager_start(&config, &ble) != SYLVAC_BLE_OK) return 1;

    sylvac_ble_mac mac;
    sylvac_ble_mac_parse("D216182F6CCC", &mac);   /* the address on the instrument label */
    sylvac_ble_start_connect(ble, &mac);         /* kept connected until disconnected */

    sylvac_ble_measuring_params params;
    int have_params = 0;
    for (;;) {
        sylvac_ble_event *ev;
        sylvac_ble_status st = sylvac_ble_next_event(ble, 1000, &ev);
        if (st == SYLVAC_BLE_TIMEOUT) continue;
        if (st != SYLVAC_BLE_OK) break;
        switch (ev->kind) {
        case SYLVAC_BLE_EVENT_CONNECTION_STEP_CHANGED:
            if (ev->data.connection_step_changed.value == SYLVAC_BLE_CONNECTION_STEP_SUBSCRIBED)
                sylvac_ble_send_command(ble, &ev->data.connection_step_changed.peripheral, "ID?");
            break;
        case SYLVAC_BLE_EVENT_MEASUREMENT_PARAMS_CHANGED:   /* always before the values */
            params = ev->data.measurement_params_changed.value;
            have_params = 1;
            break;
        case SYLVAC_BLE_EVENT_NUMERICAL_VALUE_CHANGED:
            if (have_params) {
                char text[SYLVAC_BLE_VALUE_STRING_SIZE];
                sylvac_ble_format_value(&params, ev->data.numerical_value_changed.value,
                                        text, sizeof text);
                printf("value %s\n", text);
            }
            break;
        case SYLVAC_BLE_EVENT_INSTRUMENT_RESPONSE_RECEIVED:
            printf("answer %s\n", ev->data.instrument_response_received.value);
            break;
        }
        sylvac_ble_event_free(ev);
    }
    sylvac_ble_manager_free(ble);           /* stops the library if needed */
    return 0;
}
```

## How it works

The library works in the background, on its own thread. When you ask it to do something, such as
connecting an instrument or sending a command, the call returns straight away; the result
arrives later as an event. Instruments also send events on their own: a new value, a press on
the Data button, a lost connection.

You can take the events one at a time with `sylvac_ble_next_event()`, as above, or have the
library call a function of yours with `sylvac_ble_set_callback()`. That function runs on the
library's thread, so keep it short. If you would rather wait for a result,
`sylvac_ble_connect()`, `sylvac_ble_disconnect()` and `sylvac_ble_send_and_wait()` block
until it comes.

The header documents every function, structure and event, and says which thread may call what.
Always use the header that comes with the binaries: `sylvac_ble_version()` and
`SYLVAC_BLE_ABI_VERSION` tell you which library you have.

The library includes open-source components; see [THIRD-PARTY-NOTICES.txt](THIRD-PARTY-NOTICES.txt).
