//! From the C structs to the Rust types. The only place that reads the event union.

use std::ffi::{c_char, CStr};

use sylvac_ble_sys as sys;

use crate::types::{
    BleAdvertiser, BleCentral, BondMode, CentralKind, ConnectionStep, DisconnectReason, Event,
    LogLevel, Mac, MeasuringParams, PairingStatus, RejectReason, SdsMode, SdsResolution, SdsUnit,
};

/// # Safety
/// `p` is NULL or a NUL-terminated string valid for the call.
unsafe fn optional(p: *const c_char) -> Option<String> {
    if p.is_null() {
        return None;
    }
    // SAFETY: non-NULL and NUL-terminated per the caller's contract. The library only hands
    // out UTF-8; a lossy conversion keeps this free of panics whatever comes.
    Some(unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned())
}

/// # Safety
/// As [`optional`]. NULL (never sent for these fields) reads as an empty text.
unsafe fn text(p: *const c_char) -> String {
    // SAFETY: per the caller's contract.
    unsafe { optional(p) }.unwrap_or_default()
}

fn mac(m: sys::sylvac_ble_mac) -> Mac {
    Mac::from_le_bytes(m.le_bytes)
}

/// # Safety
/// The strings of `d` are NULL or valid NUL-terminated strings; `bonded_macs` is NULL or holds
/// `bonded_mac_count` addresses.
pub(crate) unsafe fn central(d: &sys::sylvac_ble_central) -> BleCentral {
    // SAFETY: per the caller's contract.
    let bonded_macs = unsafe { macs(d.bonded_macs, d.bonded_mac_count) };
    // SAFETY: per the caller's contract.
    let peripherals = unsafe { macs(d.peripherals, d.peripheral_count) };
    BleCentral {
        // SAFETY: per the caller's contract.
        port_name: unsafe { text(d.port_name) },
        mac: mac(d.mac),
        kind: match d.kind {
            sys::SYLVAC_BLE_CENTRAL_KIND_BLED112 => CentralKind::Bled112,
            sys::SYLVAC_BLE_CENTRAL_KIND_BT5_SMART => CentralKind::Bt5Smart,
            _ => CentralKind::Unknown,
        },
        // SAFETY: per the caller's contract.
        version: unsafe { text(d.version) },
        build: d.build,
        max_conn: u8::try_from(d.max_conn).unwrap_or(u8::MAX),
        bonded_macs,
        peripherals,
    }
}

/// # Safety
/// `array` is NULL or points to `count` addresses.
unsafe fn macs(array: *const sys::sylvac_ble_mac, count: u32) -> Vec<Mac> {
    if array.is_null() || count == 0 {
        return Vec::new();
    }
    // SAFETY: per the caller's contract.
    unsafe { std::slice::from_raw_parts(array, count as usize) }
        .iter()
        .map(|m| mac(*m))
        .collect()
}

/// # Safety
/// The strings of `a` are NULL or valid NUL-terminated strings.
pub(crate) unsafe fn advertiser(a: &sys::sylvac_ble_advertiser) -> BleAdvertiser {
    BleAdvertiser {
        mac: mac(a.mac),
        // SAFETY: per the caller's contract.
        name: unsafe { optional(a.name) },
        rssi: a.rssi.clamp(i32::from(i8::MIN), i32::from(i8::MAX)) as i8,
        // SAFETY: per the caller's contract.
        value: unsafe { optional(a.value) },
        float_value: (!a.float_value.is_nan()).then_some(a.float_value),
        battery: u8::try_from(a.battery).ok(),
        // SAFETY: per the caller's contract.
        article_number: unsafe { optional(a.article_number) },
        bond_mode: match a.bond_mode {
            sys::SYLVAC_BLE_BOND_MODE_BONDABLE_BONDED => BondMode::BondableBonded,
            sys::SYLVAC_BLE_BOND_MODE_BONDABLE_NOT_BONDED => BondMode::BondableNotBonded,
            sys::SYLVAC_BLE_BOND_MODE_NON_BONDABLE => BondMode::NonBondable,
            _ => BondMode::Undefined,
        },
        bonded_to_dongle: a.bonded_to_dongle != 0,
        date_received: std::time::UNIX_EPOCH + std::time::Duration::from_millis(a.date_received_ms),
    }
}

/// What the getters of [`crate::BlePeripheral`] read.
#[derive(Debug, Clone, Copy)]
pub(crate) struct PeripheralState {
    pub(crate) step: ConnectionStep,
    pub(crate) has_live_values: bool,
    pub(crate) central: Option<Mac>,
    pub(crate) pairing_status: PairingStatus,
    pub(crate) auto_reassign: bool,
    pub(crate) measurement_params: Option<MeasuringParams>,
    pub(crate) numerical_value: f64,
}

impl Default for PeripheralState {
    fn default() -> Self {
        Self {
            step: ConnectionStep::None,
            has_live_values: false,
            central: None,
            pairing_status: PairingStatus::default(),
            auto_reassign: false,
            measurement_params: None,
            numerical_value: f64::NAN,
        }
    }
}

pub(crate) fn peripheral_state(s: &sys::sylvac_ble_peripheral_state) -> (Mac, PeripheralState) {
    let state = PeripheralState {
        step: connection_step(s.step),
        has_live_values: s.has_live_values != 0,
        central: (s.has_central != 0).then(|| mac(s.central)),
        pairing_status: pairing_status(&s.pairing_status),
        auto_reassign: s.auto_reassign != 0,
        measurement_params: (s.has_measurement_params != 0)
            .then(|| measuring_params(&s.measurement_params)),
        numerical_value: s.numerical_value,
    };
    (mac(s.peripheral), state)
}

fn pairing_status(p: &sys::sylvac_ble_pairing_status) -> PairingStatus {
    PairingStatus {
        is_paired: p.is_paired != 0,
        is_bonded: p.is_bonded != 0,
    }
}

fn measuring_params(p: &sys::sylvac_ble_measuring_params) -> MeasuringParams {
    MeasuringParams {
        unit: match p.unit {
            sys::SYLVAC_BLE_SDS_UNIT_MILLIMETER => SdsUnit::Millimeter,
            sys::SYLVAC_BLE_SDS_UNIT_INCH => SdsUnit::Inch,
            sys::SYLVAC_BLE_SDS_UNIT_DEGREE => SdsUnit::Degree,
            sys::SYLVAC_BLE_SDS_UNIT_DEGREE_MINUTE => SdsUnit::DegreeMinute,
            sys::SYLVAC_BLE_SDS_UNIT_RADIAN => SdsUnit::Radian,
            _ => SdsUnit::Undefined,
        },
        measuring_mode: match p.measuring_mode {
            sys::SYLVAC_BLE_SDS_MODE_MIN => SdsMode::Min,
            sys::SYLVAC_BLE_SDS_MODE_MAX => SdsMode::Max,
            sys::SYLVAC_BLE_SDS_MODE_DELTA => SdsMode::Delta,
            // Normal, or a number a newer library adds.
            _ => SdsMode::Normal,
        },
        numerical_resolution: p.numerical_resolution,
        resolution_class: match p.resolution_class {
            sys::SYLVAC_BLE_SDS_RESOLUTION_MINUS1 => SdsResolution::Minus1,
            sys::SYLVAC_BLE_SDS_RESOLUTION_MINUS2 => SdsResolution::Minus2,
            sys::SYLVAC_BLE_SDS_RESOLUTION_MINUS3 => SdsResolution::Minus3,
            sys::SYLVAC_BLE_SDS_RESOLUTION_MINUS4 => SdsResolution::Minus4,
            sys::SYLVAC_BLE_SDS_RESOLUTION_MINUS5 => SdsResolution::Minus5,
            _ => SdsResolution::Undefined,
        },
    }
}

/// [`MeasuringParams::format`] through `sylvac_ble_format_value`, so every binding shows the
/// same text.
pub(crate) fn format_value(p: &MeasuringParams, value: f64) -> String {
    let params = sys::sylvac_ble_measuring_params {
        unit: match p.unit {
            SdsUnit::Millimeter => sys::SYLVAC_BLE_SDS_UNIT_MILLIMETER,
            SdsUnit::Inch => sys::SYLVAC_BLE_SDS_UNIT_INCH,
            SdsUnit::Degree => sys::SYLVAC_BLE_SDS_UNIT_DEGREE,
            SdsUnit::DegreeMinute => sys::SYLVAC_BLE_SDS_UNIT_DEGREE_MINUTE,
            SdsUnit::Radian => sys::SYLVAC_BLE_SDS_UNIT_RADIAN,
            SdsUnit::Undefined => sys::SYLVAC_BLE_SDS_UNIT_UNDEFINED,
        },
        measuring_mode: match p.measuring_mode {
            SdsMode::Normal => sys::SYLVAC_BLE_SDS_MODE_NORMAL,
            SdsMode::Min => sys::SYLVAC_BLE_SDS_MODE_MIN,
            SdsMode::Max => sys::SYLVAC_BLE_SDS_MODE_MAX,
            SdsMode::Delta => sys::SYLVAC_BLE_SDS_MODE_DELTA,
        },
        numerical_resolution: p.numerical_resolution,
        resolution_class: p.resolution_class as u32,
    };
    // Room for any f64 (309 integer digits, 15 decimals), not only an instrument's values.
    let mut buffer = [0 as c_char; 400];
    // SAFETY: valid parameters and a buffer of the size given.
    let status = unsafe {
        sys::sylvac_ble_format_value(&params, value, buffer.as_mut_ptr(), buffer.len() as u32)
    };
    if status != sys::SYLVAC_BLE_OK {
        return value.to_string();
    }
    // SAFETY: the library wrote a NUL-terminated string into `buffer`.
    unsafe { text(buffer.as_ptr()) }
}

fn connection_step(step: u32) -> ConnectionStep {
    match step {
        sys::SYLVAC_BLE_CONNECTION_STEP_ESTABLISHED => ConnectionStep::Established,
        sys::SYLVAC_BLE_CONNECTION_STEP_DISCOVERING => ConnectionStep::Discovering,
        sys::SYLVAC_BLE_CONNECTION_STEP_DISCOVERED => ConnectionStep::Discovered,
        sys::SYLVAC_BLE_CONNECTION_STEP_SUBSCRIBING => ConnectionStep::Subscribing,
        sys::SYLVAC_BLE_CONNECTION_STEP_SUBSCRIBED => ConnectionStep::Subscribed,
        _ => ConnectionStep::None,
    }
}

/// # Safety
/// `e` is an event handed out by the library (from `sylvac_ble_next_event` or lent to the
/// callback), not freed yet: the union member named by `kind` is initialised and its strings
/// are valid.
pub(crate) unsafe fn event(e: &sys::sylvac_ble_event) -> Event {
    let data = &e.data;
    // SAFETY: each arm reads the union member the library documents for that kind, and its
    // strings, valid per the caller's contract.
    unsafe {
        match e.kind {
            sys::SYLVAC_BLE_EVENT_CENTRAL_ADDED => {
                Event::CentralAdded(central(&data.central_added))
            }
            sys::SYLVAC_BLE_EVENT_CENTRAL_REMOVED => Event::CentralRemoved {
                port_name: text(data.central_removed.port_name),
                mac: mac(data.central_removed.mac),
                error: optional(data.central_removed.error),
            },
            sys::SYLVAC_BLE_EVENT_CENTRAL_FAILED => Event::CentralFailed {
                port_name: text(data.central_failed.port_name),
                error: text(data.central_failed.error),
            },
            sys::SYLVAC_BLE_EVENT_ADVERTISER_CHANGED => {
                Event::AdvertiserChanged(advertiser(&data.advertiser_changed))
            }
            sys::SYLVAC_BLE_EVENT_ADVERTISER_REMOVED => Event::AdvertiserRemoved {
                mac: mac(data.advertiser_removed.mac),
            },
            sys::SYLVAC_BLE_EVENT_PERIPHERAL_ADDED => Event::PeripheralAdded {
                peripheral: mac(data.peripheral_list.peripheral),
            },
            sys::SYLVAC_BLE_EVENT_PERIPHERAL_REMOVED => Event::PeripheralRemoved {
                peripheral: mac(data.peripheral_list.peripheral),
            },
            sys::SYLVAC_BLE_EVENT_CONNECTING => Event::Connecting {
                peripheral: mac(data.connecting.peripheral),
                central: mac(data.connecting.central),
            },
            sys::SYLVAC_BLE_EVENT_CONNECTION_STEP_CHANGED => Event::ConnectionStepChanged {
                peripheral: mac(data.connection_step_changed.peripheral),
                value: connection_step(data.connection_step_changed.value),
            },
            sys::SYLVAC_BLE_EVENT_CENTRAL_TO_CONNECT_CHANGED => {
                let d = &data.central_to_connect_changed;
                Event::CentralToConnectChanged {
                    peripheral: mac(d.peripheral),
                    old_value: (d.has_old_value != 0).then(|| mac(d.old_value)),
                    new_value: (d.has_new_value != 0).then(|| mac(d.new_value)),
                }
            }
            sys::SYLVAC_BLE_EVENT_PAIRING_STATUS_CHANGED => Event::PairingStatusChanged {
                peripheral: mac(data.pairing_status_changed.peripheral),
                value: pairing_status(&data.pairing_status_changed.value),
            },
            sys::SYLVAC_BLE_EVENT_BONDED_MACS_LIST_CHANGED => {
                let b = &data.bonded_macs_list_changed;
                Event::BondedMacsListChanged {
                    central: mac(b.central),
                    // The arrays hold their counts, per the C ABI.
                    added: macs(b.added, b.added_count),
                    removed: macs(b.removed, b.removed_count),
                }
            }
            sys::SYLVAC_BLE_EVENT_MEASUREMENT_PARAMS_CHANGED => Event::MeasurementParamsChanged {
                peripheral: mac(data.measurement_params_changed.peripheral),
                value: measuring_params(&data.measurement_params_changed.value),
            },
            sys::SYLVAC_BLE_EVENT_NUMERICAL_VALUE_CHANGED => Event::NumericalValueChanged {
                peripheral: mac(data.numerical_value_changed.peripheral),
                value: data.numerical_value_changed.value,
                measurement_params: measuring_params(
                    &data.numerical_value_changed.measurement_params,
                ),
            },
            sys::SYLVAC_BLE_EVENT_INSTRUMENT_BUTTON_PRESSED => Event::InstrumentButtonPressed {
                peripheral: mac(data.instrument_button_pressed.peripheral),
                value: text(data.instrument_button_pressed.value),
            },
            sys::SYLVAC_BLE_EVENT_INSTRUMENT_RESPONSE_RECEIVED => {
                Event::InstrumentResponseReceived {
                    peripheral: mac(data.instrument_response_received.peripheral),
                    value: text(data.instrument_response_received.value),
                    command: optional(data.instrument_response_received.command),
                }
            }
            sys::SYLVAC_BLE_EVENT_INSTRUMENT_RESPONSE_TIMEOUT => Event::InstrumentResponseTimeout {
                peripheral: mac(data.command.peripheral),
                command: text(data.command.command),
            },
            sys::SYLVAC_BLE_EVENT_INSTRUMENT_COMMAND_DROPPED => Event::InstrumentCommandDropped {
                peripheral: mac(data.command.peripheral),
                command: text(data.command.command),
            },
            sys::SYLVAC_BLE_EVENT_DISCONNECTED => Event::Disconnected {
                peripheral: mac(data.disconnected.peripheral),
                reason: match data.disconnected.reason {
                    sys::SYLVAC_BLE_DISCONNECT_REQUESTED => DisconnectReason::Requested,
                    sys::SYLVAC_BLE_DISCONNECT_LINK => DisconnectReason::Link(
                        u16::try_from(data.disconnected.link_code).unwrap_or(u16::MAX),
                    ),
                    sys::SYLVAC_BLE_DISCONNECT_DONGLE_LOST => DisconnectReason::DongleLost,
                    other => DisconnectReason::Other(other),
                },
                reconnecting: data.disconnected.reconnecting != 0,
            },
            sys::SYLVAC_BLE_EVENT_REJECTED => Event::Rejected {
                peripheral: mac(data.rejected.peripheral),
                reason: reject_reason(data.rejected.reason),
            },
            sys::SYLVAC_BLE_EVENT_LIB_LOG => Event::LibLog {
                level: log_level(data.lib_log.level),
                message: text(data.lib_log.message),
            },
            sys::SYLVAC_BLE_EVENT_OVERFLOW => Event::Overflow {
                dropped: data.overflow.dropped,
            },
            kind => Event::Unknown { kind },
        }
    }
}

/// `SYLVAC_BLE_LOG_LEVEL_*` to [`LogLevel`]; a level this crate does not know reads as `Debug`.
pub(crate) fn log_level(level: u32) -> LogLevel {
    match level {
        sys::SYLVAC_BLE_LOG_LEVEL_NONE => LogLevel::None,
        sys::SYLVAC_BLE_LOG_LEVEL_ERROR => LogLevel::Error,
        sys::SYLVAC_BLE_LOG_LEVEL_WARNING => LogLevel::Warning,
        sys::SYLVAC_BLE_LOG_LEVEL_INFO => LogLevel::Info,
        _ => LogLevel::Debug,
    }
}

/// [`LogLevel`] to `SYLVAC_BLE_LOG_LEVEL_*`.
pub(crate) fn raw_log_level(level: LogLevel) -> u32 {
    match level {
        LogLevel::None => sys::SYLVAC_BLE_LOG_LEVEL_NONE,
        LogLevel::Error => sys::SYLVAC_BLE_LOG_LEVEL_ERROR,
        LogLevel::Warning => sys::SYLVAC_BLE_LOG_LEVEL_WARNING,
        LogLevel::Info => sys::SYLVAC_BLE_LOG_LEVEL_INFO,
        LogLevel::Debug => sys::SYLVAC_BLE_LOG_LEVEL_DEBUG,
    }
}

/// A `SYLVAC_BLE_REJECT_*` number.
pub(crate) fn reject_reason(reason: u32) -> RejectReason {
    match reason {
        sys::SYLVAC_BLE_REJECT_ALL_CENTRALS_FULL => RejectReason::AllCentralsFull,
        sys::SYLVAC_BLE_REJECT_NOT_CONNECTED => RejectReason::NotConnected,
        sys::SYLVAC_BLE_REJECT_NOT_READY => RejectReason::NotReady,
        sys::SYLVAC_BLE_REJECT_CENTRAL_REFUSED => RejectReason::CentralRefused,
        sys::SYLVAC_BLE_REJECT_CENTRAL_FULL => RejectReason::CentralFull,
        sys::SYLVAC_BLE_REJECT_UNKNOWN_CENTRAL => RejectReason::UnknownCentral,
        sys::SYLVAC_BLE_REJECT_NO_BOND => RejectReason::NoBond,
        other => RejectReason::Other(other),
    }
}
