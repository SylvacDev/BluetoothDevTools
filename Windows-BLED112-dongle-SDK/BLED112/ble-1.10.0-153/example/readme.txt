
 This Folder contains example of projects which can be easily used to start working with Silicon Labs modules.
 There are three main folders:
    dkble        - Examples for DKBLE development board
    dkble112     - Examples for Silicon Labs DKBLE112 development board
    dkble113     - Examples for Silicon Labs DKBLE113 development board
 Below short description of each example:

 
    Project:        Silicon Labs Battery 
    Folder name:    battery
    Purpose:        Provide battery voltage readout.
    Description:    This project bases on Battery Service which is part of BLE GATT specification.
                    Project configuration contains a BG script - battery.bgs - which handles mater battery readout request, 
                    scales read ADC value for percentage notation and sends it back to the master.

    Project:        BG Demo
    Folder name:    bgdemo
    Purpose:        Provide periodic VDD readout.
    Description:    This project starts 12-bits VDD readout every 1 second. 
                    VDD value is finally written to "xgatt_battery"  characteristic and notified to the master.
                    Data can be notified by "xgatt_battery" characteristic if Client Configuration characteristic flag is set 1.

    Project:        Silicon Labs CR Demo
    Folder name:    cable_replacement
    Purpose:        Provide bi-directional communication between USART1 and "xgatt_data" characteristic.
    Description:    This project provides communication between USART1 and 128-bit uuid "xgatt_data" characteristic. 
                    Data can be sent by USART1 and write to "xgatt_data" characteristic and vice versa. 
                    Data can be indicated by "xgatt_data" characteristic if Client Configuration characteristic flag is set 2.

    Project:        BLExxx-xxx thermometer
    Folder name:    dkble
    Purpose:        Provide Thermometer readout for all BG modules.
    Description:    This is example of periodic temperature measurement for all BG modules. 
                    Temperature is displayed on development kit and also can be indicated by Temperature service if indication is enabled.

    Project:        BG Demo
    Folder name:    evkit_accelerometer
    Purpose:        Provide example of ADXL350 accelerometer readout for all BG modules.
    Description:    This project shows how to use periodic accelerometer readout. 
                    BG script configures SPI and accelerometer and starts 1 sec periodic timer.
                    Accelerometer value can be notify by "acc_value" if indication is enabled.

    Project:        BG Demo
    Folder name:    evkit_altimeter
    Purpose:        Provide example of MPL3115A2 altimeter readout for BLE121LR-m256k, BLE113 and BLE113-m256k
    Description:    This is example how to use periodic altimeter readout and notify values on characteristics.
                    MPL3115A2 and BLE module use I2C to communicate with each other.  
                    The temperature and altitude are notify on "alt_value" and "temp_value" characteristic if notification flag is set.

    Project:        BG Demo
    Folder name:    evkit_display
    Purpose:        Provide example of display usage for BLE121, BLE113 and BLE113-m256k
    Description:    This is example how to configure display on development kit and display "Hello World!" string. 
                    Note: BLE module does not enter in advertising mode.

    Project:        Silicon Labs Heart Rate Demo
    Folder name:    hr
    Purpose:        Provide example of Heart Rate readout for all BLE modules				
    Description:    This is example how to use periodic heart rate readout and notify values on "xgatt_hr" characteristics.
                    GATT configuration use Heart Rate service for indication. 
                    BG script reads 9 effective bits from AIN6, converts it and writes to "xgatt_hr" characteristic.

    Project:        Silicon Labs HTM demo
    Folder name:    htm
    Purpose:        Provide example of periodic Health Thermometer readout for all BLE modules.
    Description:    This is example shows how to use periodic health thermometer readout and notify values on "xgatt_temperature_celsius" characteristics.
                    Periodic readout is started/stopped by enabling/disabling characteristic "xgatt_temperature_celsius" indication. 
                    Temperature is periodically read from Temperature sensor, converted to Celsius scale and written to "xgatt_temperature_celsius" characteristic.

    Project:        Silicon Labs Keyboard Demo
    Folder name:    keyboard
    Purpose:        Provide example of HID keyboard for all BLE modules.
    Description:    This example base on HID Service operating as keyboard.
                    BG script start 1 second timer on every connection event and stop it after disconnection. 
                    Soft timer handler fills data buffer with the pattern and write it to "hid_keyboard_in" characteristic. 
                    Next, clear data buffer and writes it again to "hid_keyboard_in" characteristic. 
                    Those patterns are indicated when "hid_keyboard_in" characteristic indication is enabled.
	
    Project:        Silicon Labs OTA Demo
    Folder name:    ota
    Purpose:        Provide example of flashing images to internal and external flash via OTA.
    Description:    After booting into DFU mode bootloader reads flash contents and flashes contents to module.
                    More detail can be found in "ota256.bgs" and "ota.bgs" files for external and internal flash respectively.

    Project:        Silicon Labs UART Demo
    Folder name:    uartdemo
    Purpose:        Use UART1 as default interface for all BLE modules.
    Description:    Provide example of UART configuration. UART is configured as USART1 interface with 57600 b/s baud rate.
                    After reseting BLE module sends initialization string over the USART1. BLE module can be also managed by UART interface.
                    Note: BLE module is in sleep mode by default. Configuration of "wake up" pin allows to wake it up after pressing button P0_0.
                    Setting up <sleep enable="false" /> in hardware file disable sleep of BLE module.

    Project:        Silicon Labs BLED112
    Folder name:    usbcdc
    Purpose:        Use USB as default interface for BLE112
    Description:    Provide example of USB configuration. USB usage on development kit requires also switching "USB to UART converter" on.
                    BLE module is recognized by system as "Bluegiga Low Energy Dongle".

    Project:        Silicon Labs Find Me
    Folder name:    find_me
    Purpose:        This project base on Immediate Alert Service which is part of BLE GATT specification.
    Description:    Provide example of indication Immediate Alert. There is possible to indicate three level on alert: No Alert, Mild Alert and High Alert.
                    Alert level is displayed on development kit display after sending write command w/o response to Alert Level characteristic.

    Project:        Silicon Labs Mouse Demo
    Folder name:    mouse
    Purpose:        Provide example of HID mouse for BLE113 module.
    Description:    This example base on HID Service operating as mouse.
                    20 ms soft timer is started when Client Configuration characteristic is enabled for "hid_mouse" or "hid_boot_mouse" characteristic respectively. 
                    After reset BLE module operates as "report mode". This can be changed for "boot mode" when "hid_protocol_mode" characteristic value is set to 2.
					Current buttons status (pressed/release) and accelerometer value are written to the "hid_mouse" or "hid_boot_mouse"  depending on the value of "hid_protocol_mode" characteristic.

    Project:        DKBLE112 thermometer
    Folder name:    dkble112
    Purpose:        Provide example of periodic Health Thermometer readout for  DKBLE112 module. 
    Description:    This is example shows how to use periodic health thermometer readout, indicate values on "xgatt_temperature_celsius" characteristics
	                and write temperature to the display. Periodic readout is started after module boot up. Temperature is periodically read from Temperature sensor, 
                    converted to Celsius scale and written to "xgatt_temperature_celsius" characteristic and display.

    Project:        DKBLE112 thermometer
    Folder name:    dkble113
    Purpose:        Provide example of periodic Health Thermometer readout for  DKBLE113 module.
    Description:    This is example shows how to use periodic health thermometer readout, indicate values on "xgatt_temperature_celsius" characteristics
	                and write temperature to the display. Periodic readout is started after module boot up. Temperature is periodically read from Temperature sensor, 
                    converted to Celsius scale and written to "xgatt_temperature_celsius" characteristic and display.
