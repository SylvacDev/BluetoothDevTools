#pragma once

#include <string>
#include <Windows.h>

/**
 *	@file ConstanteVisible.h
 *
 *	File who contains all enum variable that can be used outside the DLL 
 *
 *	@author		Schaller Julien / Sylvac SA
 *	@version	SylvacInstrumentCommunicationDLL v1.00 - 20.03.2015
 *
 */

/**
 *	@enum
 *	Enum representing all type of message that can be pass through the DLL
 */
enum TypeData{
	/** Info Message*/
	INFO_MESSAGE,
	/** Respond to a Command*/
	DATA_MESSAGE,
	/** A new usb or dongle device has been connected*/
	NEW_DEVICE,
	/** An usb or dongle has been unplug */
	REMOVE_DEVICE,
	/** A bluetooth instrument was found by the scan */
	FOUND_INSTRUMENT,
	/** Conenction step */
	INSTR_CONNECTION_STEP,
	/** Connection to a BT instrument has start */
	CONNECT_START,
	/** A bluetooth instrument was connected successfully */
	CONNECT_SUCCESS,
	/** A bluetooth instrument has failed his connection */
	CONNECT_FAIL,
	/** A bluetooth instrument has been disconnected randomly */
	DISCONNECT_EVENT,
	/** A bluetooth instrument has been disconnected by the user */
	DESIRED_DISCONNECT_EVENT,
	/** A bluetooth scan has start on the dongle */
	SCAN_START,
	/** A bluetooth scan has stop on the dongle */
	SCAN_STOP,
	/** The DLL init has finished */
	END_INIT,
	/** Log message for debug */
	LOG_MESSAGE,
	/** Send an error when a connection fails because the dongle return an error */
	ERROR_DURING_BT_CONNECTION,
	/** An error has occured during the called function */
	ERROR_T,
	/** Data send from the button of the instrument */
	DATA_MESSAGE_FROM_BUTTON,
	/** Message receive when the stop scan command failed */
	ERROR_STOP_SCAN,
	/** Message receive when the start scan command failed */
	ERROR_START_SCAN
};

/**
 *	@enum
 *	Enum who give all type of devices that we can connect
 */
enum class TypeDevice{
	/** An usb device with an FTDI chip */
	USB_DEVICE,
	/** A dongle device from Bluegiga */
	DONGLE_DEVICE,
	/** A RS232 Device without FTDI */
	RS232_DEVICE,
	/** A MBRS Device */
	MBRS_DEVICE,
	/** Card Bluetooth */
	BLUETOOTH_CARD,
	/** Laird Bluetooth */
	LAIRD_BLUETOOTH,
	/** RPI rotary table */
	ROTARY_TABLE_RPI
};

/**
*	@enum
*	Enum to determine the type behind a BT connection
*/
enum class TypeDeviceBluetooth{
	/** Can't determine the type of the instrument */
	TYPE_INSTR_NONE,
	/** It's a real BT instrument supported by the DLL */
	TYPE_INSTRUMENT,
	/** The instrument is a BT pedal */
	TYPE_PEDAL
};

/**
 *	@enum
 *	Enum who give all decimal separator that we can use
 */
enum class TypeDecimalSeparator{
	/** All numerical data will use a point */
	DECIMAL_POINT,
	/** All numerical data will use a comma */
	DECIMAL_COMMA,
	/** All numerical data will use the decimal separator that the OS use */
	DEFINED_BY_OS
};

/**
 *	@enum
 *	Enum for all the scan that we can start
 */
enum class TypeScan{
	/** A scan only for one mac address */
	SCAN_BY_ADDRESS, 
	/** A scan during a defined number of time */
	SCAN_DURING_TIME,
	/** A scan who run all the time */
	SCAN_DURING_ALL_TIME 
};

/**
 *	@enum
 *	Enum representing the connecting state of a device 
 */
enum class ConnectionState{
	/** No state */
	CONNECTION_STATE_NONE,
	/** Instrument was not connect during the given time */
	CONNECTION_TIMEOUT,
	/** Instrument begin a connection */
	DEVICE_CONNECTING,
	/** Instrument has finished connecting and is connected */
	DEVICE_CONNECTED,
	/** Instrument was disconnected by the user */
	DESIRED_DISCONNECTION, 
	/** Instrument was disconnected randomly */
	NON_DESIRED_DISCONNECTION, 
	/** Instrument was disconnected during his connection */
	DISCONNECTION_DURING_CONNECTION,
	/** Instrument is disconnected */
	DEVICE_DISCONNECTED
};

/**
 *	@enum
 *	Enum representing the pairing state of a device
 */
enum class MethodEncryption {
	/** Unknown state */
	UNKNOWN,
	/** Device is already paired */
	ALREADY_PAIRED,
	/** Device need to be paired */
	PAIRED,
	/** Device that don't need a pairing (simple device) */
	NON_PAIRED
};

/**
 *	@enum
 *	Enum for possible BaudRate 
 */
enum ParamBaudRate{
	PARAM_110 = 110,
	PARAM_300 = 300,
	PARAM_600 = 600,
	PARAM_1200 = 1200,
	PARAM_2400 = 2400,
	PARAM_4800 = 4800,
	PARAM_9600 = 9600,
	PARAM_14400 = 14400,
	PARAM_19200 = 19200,
	PARAM_28800 = 28800,
	PARAM_38400 = 38400,
	PARAM_56000 = 56000,
	PARAM_57600 = 57600,
	PARAM_115200 = 115200,
	PARAM_128000 = 128000,
	PARAM_153600 = 153600,
	PARAM_230400 = 230400,
	PARAM_256000 = 256000,
	PARAM_460800 = 460800,
	PARAM_921600 = 921600
};

/**
 *	@enum
 *	Enum for possible parity
 */
enum ParamParity{
	NO_PARITY = 0,
	ODD_PARITY = 1,
	EVEN_PARITY = 2,
	MARK_PARITY = 3,
	SPACE_PARITY = 4
};

/**
 *	@enum
 *	Enum for possible stop bits
 */
enum ParamStopBit{
	ONE_STOP_BIT = 0,
	ONE_HALF_STOP_BIT = 1,
	TWO_STOP_BIT = 2
};

/**
 *	@enum
 *	Enum for possible byte size
 */
enum ParamByteSize{
	BITS_8 = 8,
	BITS_7 = 7,
	BITS_6 = 6,
	BITS_5 = 5
};

/**
 *  @enum
 *	Enum for Bluetooth PHY
 */
enum class BlePhy {

	/** @brief Unspecified PHY */
	PHY_UNSPECIFIED = 0,

	/** @brief Non-coded PHY 1Mb/s (Bluetooth 4 compatible) */
	PHY_1M = 1,

	/** @brief Non-coded PHY 2Mb/s */
	PHY_2M = 2,

	/** @brief Coded PHY 500 Kb/s (long range) */
	PHY_CODED_S2 = 3,

	/** @brief Coded PHY 125 Kb/s (long range) */
	PHY_CODED_S8 = 4,
};

/**
 *  @enum
 *	Error occurring when requesting to change Bluetooth PHY.
 */
enum class BlePhyError {

	/** PHY can be changed. */
	SUCCESS = 0,

	/** @brief PHY change is not supported by the central (dongle). */
	CENTRAL_UNSUPPORTED,

	/** @brief PHY change is not supported in the current state of the central (dongle). */
	CENTRAL_WRONG_STATE,

	/** @brief PHY change cannot be done because the central (dongle) is not found. */
	CENTRAL_NOT_FOUND,

	/** @brief PHY change is not supported by the peripheral (instrument). */
	PERIPHERAL_UNSUPPORTED,

	/** @brief PHY change is not supported in the current state of the peripheral (instrument). */
	PERIPHERAL_WRONG_STATE,

	/** @brief PHY change cannot be done because the periheral (instrument) is not found. */
	PERIPHERAL_NOT_FOUND,

	/** PHY change is not supported by the library that manages the central device (dongle). */
	LIB_UNSUPPORTED,

	/** @brief PHY change is refused for an unspecified reason. */
	OTHER,
};
