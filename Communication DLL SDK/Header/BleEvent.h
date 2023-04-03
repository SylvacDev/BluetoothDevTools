#pragma once

#include <windows.h>
#include "Constante.h"

/**
 * @brief PHY update event information.
 */
typedef struct {
	/** PHY of TX direction */
	BlePhy rxPhy;

	/** PHY of RX direction */
	BlePhy txPhy;
} PhyUpdateEventInfo;

/**
 * @brief PHY error event information.
 */
typedef struct {
	/** Error code */
	UINT errorCode;
} PhyErrorEventInfo;

/**
 * @brief Identifies the type of a @c BleInfoEvent .
 */
enum class BleInfoType {
	/** @brief Identifies @c PhyUpdateEventInfo */
	PHY_UPDATE_EVENT,

	/** @brief Identifies @c PhyErrorEventInfo */
	PHY_ERROR_EVENT,
};

/**
 * @brief BLE info event.
 */
typedef struct {
	/** @brief Dongle number */
	UINT dongleNumber;

	/** @brief Instrument number */
	UINT instrNumber;

	/** @brief Identifies the type of the @c info field. */
	BleInfoType infoType;

	/** @brief Event-related information. */
	union {
		PhyUpdateEventInfo phyUpdate;
		PhyErrorEventInfo phyError;
	} info;
} BleInfoEvent;

/**
 * @brief BLE info event handler.
 */
typedef void(__stdcall* BleInfoCallback)(BleInfoEvent* evt);

/**
 * @brief BLE event callbacks.
 */
typedef struct {
	/**
	 * @brief Callback for BLE info events.
	 */
	BleInfoCallback onBleInfo;
} BleCallbacks;
