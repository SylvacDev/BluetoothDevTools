/******************************************************************************/
/ BLED112 SDK TO CONNECT WITH SYLVAC BLUETOOTH ENABLED INSTRUMENTS             /
/ READ ME FIRST                                                                /
/******************************************************************************/

IMPORTANT: this SDK is for the Bluetooth 4 dongle only (BLED112 module, BT4,
Sylvac ref. 981-7100). It is a low-level access to the dongle through the
Bluegiga BGAPI protocol.

Most projects do not need it. The Sylvac libraries (folder libraries) are the
recommended way: they work with both the Bluetooth 5 SMART dongle (BL654 module,
BT5, ref. 981-7102 and 981-7103) and the Bluetooth 4 dongle (BLED112, BT4),
without any Bluetooth programming.


1. First you may be familiar with "Sylvac - Custom Bluetooth GATT profile"
   See folder gatt-profiles


2. Once you got sufficient background and need to go through development steps, 
   please start with the following document

    step-by-step single connection to a Sylvac Bluetooth technology enabled instruments:
                                        BLED112/MEM-PM 292-1806-04  Bluetooth Profile Quick Start Guide


3. Develop on Windows

    Through BLED112 Bluetooth Low Energy dongle
		According to BLED112 API, see folder \BLED112 (support by Silicon Labs and Sylvac)


Note:   Bluegiga SDK original installation files can be downloaded on Silicon Labs website 
        - https://www.silabs.com/wireless/bluetooth see legacy modules
        The Sylvac variant is unpacked and provided with dongle firmware compiled for 8 simultaneous connections (instead of 3)