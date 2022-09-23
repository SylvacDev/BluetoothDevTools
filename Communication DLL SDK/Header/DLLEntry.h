/**
 *  @file DLLEntry.h
 *  
 * \mainpage Sylvac Communication DLL
 *
 * \section intro_sec Introduction
 *
 *	DLL who manage the communication between a computer and Sylvac instruments.
 *	The given functions will allow the communication with instruments connected by
 *	cable or by bluethooth connected through an USB dongle.
 *
 *	USB and Dongle as considered as a same entity for the users but there is some functions 
 *	that can only be called with a Dongle (scan, connect or list BTInstr).
 *
 * \section how_to_use_sec How to use the DLL
 * 
 *	Before using any of the function available, you must call the method:
 *		-  SY_init(void(__stdcall *dataRecover)(BSTR, BSTR, TypeData, long), TypeDecimalSeparator typeDecimal)
 *
 *		where dataRecover is a function pointer on a function who will serve as a Callback to send info from the DLL
 *
 * \subsection step1 Example 1a: Find and Connect BT Instrument
 *		
 *		//Run a Scan for 12sec
 *		SY_runAScanForTime(12000, 0);
 *      
 *		//Recover the number of instr found and iterate throught all the instrument
 *		for(int i=0; i < SY_getNumberInstrFound(0); i++){
 *			BTInstr* instr;
 *			//Recover the i th instrument found
 *			SY_getFoundBTInstr(0, i, out instr)
 *			//Try to connect it (you can only run one connection at a time on one dongle)
 *			SY_connectInstr(0, instr->instrNumber);
 *			break;
 *		}
 *
 *		//Another exemple:
 *		//Connect already paired Bt instrument
 *
 *		BSTR mac = SY_getPairedDeviceFromDongle(0)
 *		BSTR [] listMac = mac.Split(';');
 *
 *		foreach(BSTR s in listMac){
 *			SY_runAScanForAddress(s, -1, 0);
 *		}
 *
 *
 * \subsection step2 Example 2: Manage callback message from the DLL
 *		
 *		Each message is created in another thread to not be blocked in the function.
 *
 *		void manageMessage(string message, string id, TypeData type){
 *           switch (type){
 *              case TypeData.NEW_DEVICE:
 *                  //An USB device was found and added to the list (ddngle or FTDI)  
 *                  break;
 *              case TypeData.DATA_MESSAGE:
 *                  //All the answer from command will arrive here except the data send from the button
 *                  break;
 *				case TypeData.DATA_MESSAGE_FROM_BUTTON:
 *					//Receive data send from the instrument button
 *					break;
 *              case TypeData.REMOVE_DEVICE:
 *                  //An USB device was removed from the computer (dongle or FTDI)
 *                  break;
 *              case TypeData.FOUND_INSTRUMENT:
 *                  //The scan found a bt instrument
 *                  break;
 *              case TypeData.DISCONNECT_EVENT:
 *                  //A BT instr disconnected
 *                  break;
 *              case TypeData.DESIRED_DISCONNECT_EVENT:
 *					//A Bt instr was disconnected intentionally
 *                  break;
 *              case TypeData.CONNECT_SUCCESS:
 *                  //Bt Instrument was connected with success
 *                  break;
 *              case TypeData.CONNECT_FAIL:
 *                  //the connection on a BT Instrument failed
 *                  break;
 *              case TypeData.SCAN_START:
 *					//The scan has been successfully started
 *                  break;
 *              case TypeData.SCAN_STOP:
 *					//The scan has been successfully stopped
 *                  break;
 *              case TypeData.LOG_MESSAGE:
 *              case TypeData.ERROR_T:
 *              case TypeData.INFO_MESSAGE:
 *					Console.WriteLine(message);
 *                  break;
 *           }
 *       }
 *		
 * \subsection step3 Example 3: Recover connected BT Instrument from a dongle
 *		
 *		List<BTInstrument> listInstr = new List<BTInstrument>();
 *		
 *		//Recover the number of instrument connected on the dongle
 *      uint nbConnected = SY_getNumberInstrConnectedByDongle(dongle.deviceNumber);
 *		
 *		//Iterate on the number and add to the list or do something else
 *      for (uint i = 0; i < nbConnected; i++) {
 *          BTInstrument btInstr;
 *          SY_getBTInstrFromDongle(dongle.deviceNumber, i, out btInstr);
 *          listInstr.Add(btInstr);
 *      }
 *	
 * 
 * \subsection step4 Example 4: Send command on an Instrument
 *
 *		DeviceInfo info = listFTDI[0];
 *
 *      string cmd = "?";
 *
 *      if (info.type == TypeDevice.DONGLE){
 *			//- On Dongle:
 *			BTInstrument instr = getInstrFromDongle(info.deviceNumber, indexInstr);
 *			SY_sendCmd(cmd, ref info, (int)instr.instrNumber);
 *		}else{
 *			//- On FTDI:
 *			SY_sendCmd(cmd, ref info);
 *		}
 *  
 *
 *	@author		Schaller Julien / Sylvac SA
 *	@version	SylvacInstrumentCommunicationDLL v1.45 - 25.06.2019
 */

#pragma once

#define SYLVACCOMMUNICATIONDLL_EXPORTS
#ifdef SYLVACCOMMUNICATIONDLL_EXPORTS
#define SYLVACCOMMUNICATIONDLL_API __declspec(dllexport) 
#else
#define SYLVACCOMMUNICATIONDLL_API __declspec(dllimport) 
#endif

#ifdef __GNUC__
#define DEPRECATED __attribute__((deprecated))
#elif defined(_MSC_VER)
#define DEPRECATED __declspec(deprecated)
#else
#pragma message("WARNING: You need to implement DEPRECATED for this compiler")
#define DEPRECATED
#endif

#include <windows.h>
#include <vector>
#include <string>
#include "BleEvent.h"
#include "ConstanteVisible.h"

/**
 *  Struct who define a device (USB or Dongle)
 *
 *	@param deviceNumber	: The number of the device
 *	@param type			: The type of device
 *	@param uniqueID		: The ID of the device (macAddress for a BTInstr et SerialNumber for a FTDI instrument)
 *	@param description	: A description of the device (FTDI use the description of the cable, DongleDevice gives the mac address and the portCom)
 */
typedef struct{
	UINT deviceNumber;
	TypeDevice type;
	BSTR uniqueID;
	BSTR description;
}DeviceInfo;

/**
 *  Struct who define the bluetooth advertisement packet. 
 *  Not all the field would be define, depends on the firware version of the device
 *
 *	@param rssi			: The rssi
 *	@param name			: The advertisment name of the device
 *	@param saleNumber	: The Sylvac sale number of the device
 *	@param value		: The value displayed on the instrument that has been scan
 */
typedef struct{
	signed char rssi;
	BSTR name;
	BSTR saleNumber;
	BSTR value;
}AdvertisementPacket;

/**
 *	Struct who define a bluetooth instrument
 *
 *	@param	instrNumber		 The number of the instrument
 *	@param  bond			 The bond value of the instrument (if 0xFF => no bond)
 *	@param	dongleMacAddress The dongle mac address where this instrument is attached
 *	@param	macAddress		 The mac address of the instrument
 *	@param  name			 The advertisement name of the device
 *	@param	state			 The connection state of the instrument
 *  @param  typeDeviceBt	 The type of the Bluetooth device (no use for the moment)
 *	@param	type			 The type of the instrument (Sylvac, MTY or IBRBLE)
 *	@param  writeHandle		 The handle where to write on the bluetooth characteristic (you don't have to touch this)
 */
typedef struct{
	UINT instrNumber;
	USHORT bond;
	BSTR dongleAddress;
	BSTR macAddress;
	BSTR name;
	ConnectionState state;
	TypeDeviceBluetooth typeDeviceBt;
	USHORT writeHandle;
}BTInstrument;


/**
*	Struct who define a bluetooth instrument
*
*	@param	instrNumber		 The number of the instrument
*	@param  bond			 The bond value of the instrument (if 0xFF => no bond)
*	@param	dongleMacAddress The dongle mac address where this instrument is attached
*	@param	macAddress		 The mac address of the instrument
*	@param  name			 The advertisement name of the device
*	@param	state			 The connection state of the instrument
*   @param  typeDeviceBt	 The type of the Bluetooth device (no use for the moment)
*	@param	type			 The type of the instrument (Sylvac, MTY or IBRBLE)
*	@param  writeHandle		 The handle where to write on the bluetooth characteristic (you don't have to touch this)
*	@param	packet			 The Advertisement packet receive from the bluetooth device before the connection
*/
typedef struct{
	UINT instrNumber;
	USHORT bond;
	BSTR dongleAddress;
	BSTR macAddress;
	BSTR name;
	ConnectionState state;
	TypeDeviceBluetooth typeDeviceBt;
	USHORT writeHandle;
	signed char rssi;
	BSTR nameAdv;
	BSTR saleNumber;
	BSTR value;
	MethodEncryption methodEncrypt;
}BTInstrumentAdv;

/**
 *	Get the number of devices connected. This method is called to obtain the number of times
 *	we must call the method "SY_getDeviceInfo". 
 *  
 *	@return	The number of connected devices
 */
extern "C" SYLVACCOMMUNICATIONDLL_API UINT __stdcall SY_getNumberDeviceConnected();

/**
 *	Retrieve a DeviceInfo from an index number. All devices can be retrieve with a "for" loop from
 *	0 to SY_getNumberDeviceConnected.
 *
 *	@param indexNumber	An index within the range
 *	@param device		The DeviceInfo who will be construct by the index
 *
 */
extern "C" SYLVACCOMMUNICATIONDLL_API void __stdcall SY_getDeviceInfo(UINT indexNumber, DeviceInfo* device);

/**
 *	\deprecated Start a scan on the device with the corresponding deviceNumber. The meaning of the "arg" argument change depending
 *	of the TypeScan choosen. 
 *
 *	@param typeScan		Define which scan we want to launch
 *	@param arg			The arg for the scan => must be an integer if we launch a time scan (in ms) or must be a macaddress if we scan by address
 *	@param deviceNumber	The number of the device on which we want to run a scan. Return an error if the device can't run a scan.
 *
 */
DEPRECATED extern "C" SYLVACCOMMUNICATIONDLL_API void __stdcall SY_runAScan(TypeScan typeScan, BSTR arg, UINT deviceNumber);

/**
 *  
 *	Start a scan on the device with the corresponding address. You can start more than one scan with this one, that will just add
 *  The address to a list of device that the dongle must scan. The DLL will connect directly the instrument when it found it. 
 *  When you receive the connect_success event for an instrument, you must re-run this scan to connect other instruments.
 *
 *	@param address		Define the address to search for the scan
 *	@param time			The time for the scan (if -1 run indefinetly)
 *	@param dongleNumber	The number of the dongle on which we want to run a scan. Return an error if the device can't run a scan.
 *
 */
extern "C" SYLVACCOMMUNICATIONDLL_API void __stdcall SY_runAScanForAddress(BSTR address, int time, UINT dongleNumber);

/**
*
*	Start a scan on the device with the corresponding address. You can start more than one scan with this one, that will just add
*   The address to a list of device that the dongle must scan. The DLL will connect directly the instrument when it found it.
*   When you receive the connect_success event for an instrument, you must re-run this scan to connect other instruments.
*
*	@param address		Define the address to search for the scan
*	@param time			The time for the scan (if -1 run indefinetly)
*	@param dongleNumber	The number of the dongle on which we want to run a scan. Return an error if the device can't run a scan.
*	@param writeHandle  BT handle where we can write. It's used to reconnect fastly the BT device. The user must save this value from the first connection
*					    by using the field "writeHandle" from the structure BTInstrument.
*
*/
extern "C" SYLVACCOMMUNICATIONDLL_API void __stdcall SY_runAScanForAddressForReconnection(BSTR address, int time, UINT dongleNumber, UINT writeHandle);

/**
 *	Start a scan on the device during a defined time. Take all the Sylvac instrument found.
 *
 *	@param time			The time for the scan in ms (if -1 run indefinetly)
 *	@param dongleNumber	The number of the ddongle on which we want to run a scan. Return an error if the device can't run a scan.
 *
 */
extern "C" SYLVACCOMMUNICATIONDLL_API void __stdcall SY_runAScanForTime(int time, UINT dongleNumber);

/**
 *	Quit the DLL. Disconnect all devices and all bluetooth instrument who are connected. Freed all the memory used.
 */
extern "C" SYLVACCOMMUNICATIONDLL_API void __stdcall SY_quit();

/**
 *	Send a command to a device. The instrNumber is optional if we send the command on an USB.
 *
 *	@param cmd				The command to send
 *	@param device			The DeviceInfo where the user want to send a command
 *	@param instrNumber		The number of the instrument when we send the command on a dongle
 *
 */
extern "C" SYLVACCOMMUNICATIONDLL_API bool __stdcall SY_sendCmd(BSTR cmd, DeviceInfo* device, int instrNumber = -1);

/**
 *	Send a command to a device. The instrNumber is optional if we send the command on an USB.
 *
 *	@param cmd				The command to send
 *	@param device			The DeviceInfo where the user want to send a command
 *  @param regexEndOfTrame	The char/string who end the trame that we will receive
 *	@param instrNumber		The number of the instrument when we send the command on a dongle
 *
 */
extern "C" SYLVACCOMMUNICATIONDLL_API bool __stdcall SY_sendCmdWithDefineEndTrame(BSTR cmd, DeviceInfo* device, BSTR regexEndOfTrame, int instrNumber = -1);

/**
 *	Informations about the version of the DLL
 *
 *	@return		The DLL informations
 */
extern "C" SYLVACCOMMUNICATIONDLL_API BSTR __stdcall SY_getDLLVersion();

/**
 *	This function initialize the bind between the DLL and the program who use it. To use the DLL, the init
 *	must be called before all other functions otherwise the DLL will not be able to commuicate with the calling program.
 *	The function passed in parameter take three arguments. The first "BSTR" will be the message, The second "BSTR" will be the id of the device
 *	who send the message (SerialNumber for USB, MacAddress for BTInstr or Dongle), the "TypeData" who said the type of the message that the DLL transmit
 *	and the long is a Token to know in which order the message where send (if the value we receive is -1, the order isn't important and the message can be treat directly)
 *
 *	@param	dataRecover(BSTR, BSTR, TypeData, long)		A pointer on a function that the DLL will call to transmit message
 *	@param	typeDecimal									The decimal separator used to display the value
 *
 */
extern "C" SYLVACCOMMUNICATIONDLL_API void __stdcall SY_init(void(__stdcall *dataRecover)(BSTR, BSTR, TypeData, long), TypeDecimalSeparator typeDecimal);

/**
 *	Connect an instrument on the device corresponding to the dongleNumber. When the connection finished, we receive
 *	a CONNECT_SUCCESS event through the dataRecover function. 
 *
 *	@param dongleNumber		The number of the dongle device
 *	@param instrNumber		The number of the bluetooth instrument
 */
extern "C" SYLVACCOMMUNICATIONDLL_API bool __stdcall SY_connectInstr(UINT dongleNumber, UINT instrNumber, UINT writeHandle = 0);

/**
 *	Disconnect the device with the corresponding deviceNumber
 *
 *	@param device		The device to disconnect
 *
 */
extern "C" SYLVACCOMMUNICATIONDLL_API void __stdcall SY_disconnect(DeviceInfo* device);

/**
 *	Disconnect the BT instrument with the correct instrNumber from the dongle with the corresponding dongleNumber
 *	
 *	@param dongleNumber		The number of the dongle
 *	@param instrNumber		The number of the instrument
 *	@param withRST			True if you want a BT reset on the instrument
 *
 */
extern "C" SYLVACCOMMUNICATIONDLL_API void __stdcall SY_disconnectInstr(UINT dongleNumber, UINT instrNumber, bool withRST);

/**
 *	True, if you want that the DLL reconnect the BT instrument even if a BT RST was made manually on the instrument.
 *	By default the DLL will not do it.
 *
 *	@param state		True, if you want that the DLL reconnect the BT Instrument
 */
extern "C" SYLVACCOMMUNICATIONDLL_API void __stdcall SY_reconnectIfResetPressed(bool state);

/**
 *  Retrieve a String representation of a device with the correct deviceNumber. If we choose a dongle
 *	device, the function also retrieve the string representation of all instrument connected to it.
 *
 *	@param deviceNumber		The number of the device
 *
 *	@return					A string representation of a device under the form BSTR
 */
extern "C" SYLVACCOMMUNICATIONDLL_API BSTR __stdcall SY_toString(UINT deviceNumber);

/**
 *	Retrieve the number of bluetooth instrument founded by a scan on the specific dongle.
 *
 *	@param dongleNumber		The number of the dongle 
 *
 *	@return					The number of instrument founded from the last scan on the dongle
 */
extern "C" SYLVACCOMMUNICATIONDLL_API UINT __stdcall SY_getNumberInstrFound(UINT dongleNumber);

/**
 *	Retrieve the number of connected bluetooth instruments on the specific dongle
 *
 *	@param dongleNumber		The number of the dongle
 *
 *	@return					The number of instrument connected on the dongle
 */
extern "C" SYLVACCOMMUNICATIONDLL_API UINT __stdcall SY_getNumberInstrConnectedByDongle(UINT dongleNumber);

/**
 *	Retrieve a BTInstrument from a dongle by his index. First, call SY_getNumberInstrConnectedByDongle to get the number
 *	of connected instrument and avoid an index out of bound.
 *
 *	@param dongleNumber		The number of the dongle
 *	@param indexInstr		The index of the instrument
 * 	@param instr			An "IN" parameter to construct the instrument
 *
 */
DEPRECATED extern "C" SYLVACCOMMUNICATIONDLL_API bool __stdcall SY_getBTInstrFromDongle(UINT dongleNumber, UINT indexInstr, BTInstrument* instr);

/**
*	Retrieve a BTInstrument with his advertisement packet from a dongle by his index. First, call SY_getNumberInstrConnectedByDongle to get the number
*	of connected instrument and avoid an index out of bound.
*
*	@param dongleNumber		The number of the dongle
*	@param indexInstr		The index of the instrument
* 	@param instr			An "IN" parameter to construct the instrument
*
*/
extern "C" SYLVACCOMMUNICATIONDLL_API bool __stdcall SY_getBTInstrAdvFromDongle(UINT dongleNumber, UINT indexInstr, BTInstrumentAdv* instr);

/**
 *	Retrieve a BTInstrument from the found instrument. First, call SY_getNumberInstrFounded to get the number
 *	of founded instrument and avoid an index out of bound.
 *
 *	@param dongleNumber		The number of the dongle
 *	@param indexInstr		The index of the instrument
 *	@param instr			An "IN" parameter to construct the instrument
 *
 */
DEPRECATED extern "C" SYLVACCOMMUNICATIONDLL_API bool __stdcall SY_getFoundBTInstr(UINT dongleNumber, UINT indexInstr, BTInstrument* instr);

/**
 *	Retrieve a BTInstrument from the found instrument with his advertisment packet. First, call SY_getNumberInstrFounded to get the number
 *	of founded instrument and avoid an index out of bound.
 *
 *	@param dongleNumber		The number of the dongle
 *	@param indexInstr		The index of the instrument
 *	@param instr			An "IN" parameter to construct the instrument
 *
 */
extern "C" SYLVACCOMMUNICATIONDLL_API bool __stdcall SY_getFoundBTInstrAdv(UINT dongleNumber, UINT indexInstr, BTInstrumentAdv* instr);

/**
 *	Change the communication parameters of the device with a correponding deviceNumber
 *
 *	@param device			The DeviceInfo on which the user want to change parameter
 *	@param baudRate			The new baud rate number
 *	@param byteSize			The new byte size number
 *	@param parity			The new parity
 *	@param stopBits			The new stop bits number
 *
 *	@return					True if the params were correctly passed, otherwise false
 */
extern "C" SYLVACCOMMUNICATIONDLL_API bool __stdcall SY_changeParamCommunication(DeviceInfo* device, ParamBaudRate baudRate, ParamByteSize byteSize, ParamParity parity, ParamStopBit stopBits);

/**
 *	Reset the dongle with the corresponding dongleNumber. Clear the pairing and the memory flash of the dongle
 *
 *	@param dongleNumber		The number of the dongle
 */
extern "C" SYLVACCOMMUNICATIONDLL_API void __stdcall SY_resetDongle(UINT dongleNumber);

/**
 *	Stop the scan on the corresponding dongle
 *
 *	@param dongleNumber		The number of the dongle
 */
extern "C" SYLVACCOMMUNICATIONDLL_API void __stdcall SY_stopScan(UINT dongleNumber);

/**
 *	Delete the bonding for the instrument on the dongle
 *
 *	@param dongleNumber		The number of the dongle
 *	@param bond				The bond to erase on the dongle
 */
//extern "C" SYLVACCOMMUNICATIONDLL_API void __stdcall SY_deleteBond(UINT dongleNumber, USHORT bond);

/**
 *	Erase all instrument that was found during a scan on the specified dongle.
 *
 *	@param dongleNumber		The number of the dongle
 */
extern "C" SYLVACCOMMUNICATIONDLL_API void __stdcall SY_clearFoundInstrList(UINT dongleNumber);


/**
 *	Get a string of mac address corresponding of the address of instruments already paired on a dongle
 *
 *	@param dongleNumber		The number of the dongle
 *
 *	@return		A string under the form => mac1;mac2;mac3;mac4;...
 */
extern "C" SYLVACCOMMUNICATIONDLL_API BSTR __stdcall SY_getPairedDeviceFromDongle(UINT dongleNumber);

/**
 *	The user can erase a bond on the master if the instrument isn't connected on the dongle
 *
 *	@param dongleNumber	The number of the dongle
 *	@param strMac		The mac address that we want to erase
 *
 *	@return		True if the address is erased, false otherwise
 */
extern "C" SYLVACCOMMUNICATIONDLL_API bool __stdcall SY_eraseBondOnMasterOnly(UINT dongleNumber, BSTR strMac);

/**
 *	Function to change the advertisement name of a Bluetooth device.
 *  /!\ If you change the name, perhaps the DLL would not be able to connect it anymore. Depends if the new name is in the whitelist /!\
 *
 *	@param newName		The new advertisement name
 *	@param dongleNumber	The number of the dongle
 *	@param instrNumber	The number of the instrument on the dongle
 *
 *	@return		True if the name has changed, false otherwise
 */
extern "C" SYLVACCOMMUNICATIONDLL_API bool __stdcall SY_changeDeviceAdvertisementName(BSTR newName, UINT dongleNumber, UINT instrNumber);

/**
 *	Function to change the pairing (SIMPLE to PAIR or PAIR to SIMPLE) of a BT device
 *
 *	@param dongleNumber	The number of the dongle
 *	@param instrNumber	The number of the instrument on the dongle
 *
 *	@return	 True if the instrument exist and it pairing state exist
 */
extern "C" SYLVACCOMMUNICATIONDLL_API bool __stdcall SY_changeDevicePairing(UINT dongleNumber, UINT instrNumber);

/**
 * Function to register callbacks for Bluetooth Low Energy events.
 * @param callbacks BLE event callbacks
 */
extern "C" SYLVACCOMMUNICATIONDLL_API void __stdcall SY_registerBleCallbacks(BleCallbacks * callbacks);

/**
 *	Change the PHY of an exisiting connection.
 *
 *	@param dongleNumber		The number of the dongle
 *	@param instrNumber		The number of the instrument on the dongle
 *	@param phy				New PHY to use
 *
 *	@return Success if the PHY update procedure has started, otherwise an error code.
 */
extern "C" SYLVACCOMMUNICATIONDLL_API BlePhyError __stdcall SY_changeConnectionPhy(UINT dongleNumber, UINT instrNumber, BlePhy phy);
