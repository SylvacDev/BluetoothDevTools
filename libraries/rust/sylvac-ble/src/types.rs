//! Public data types: plain data, the same names and fields as the library's own Rust API.

use std::ffi::CString;
use std::fmt;
use std::str::FromStr;
use std::time::SystemTime;

use sylvac_ble_sys as sys;

/// Bluetooth address of a dongle or an instrument, most significant byte first, printed as
/// 12 upper-case hex digits (`D216182F6CCC`, as on the instrument label).
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Mac(pub [u8; 6]);

impl fmt::Display for Mac {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(f, "{byte:02X}")?;
        }
        Ok(())
    }
}

impl fmt::Debug for Mac {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Mac({self})")
    }
}

/// `"D216182F6CCC"`, `"d2:16:18:2f:6c:cc"` or `"D2-16-…"` could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MacParseError(pub String);

impl fmt::Display for MacParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "invalid Bluetooth address {:?}: expected 12 hex digits, optionally separated by ':' or '-'",
            self.0
        )
    }
}

impl std::error::Error for MacParseError {}

impl FromStr for Mac {
    type Err = MacParseError;

    /// Read by the library (`sylvac_ble_mac_parse`), so every binding accepts the same forms.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let error = || MacParseError(text.to_owned());
        let c_text = CString::new(text).map_err(|_| error())?;
        let mut mac = sys::sylvac_ble_mac { bytes: [0; 6] };
        // SAFETY: a NUL-terminated string and a valid struct to write.
        let status = unsafe { sys::sylvac_ble_mac_parse(c_text.as_ptr(), &mut mac) };
        if status == sys::SYLVAC_BLE_OK {
            Ok(Self(mac.bytes))
        } else {
            Err(error())
        }
    }
}

/// Model of a Sylvac dongle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CentralKind {
    /// Silicon Labs BLED112 (Bluetooth 4), 8 connections.
    Bled112,
    /// Sylvac Bluetooth 5 SMART dongle (BL654, Bluetooth 5), 16 connections.
    Bt5Smart,
    /// A fixed port (board UART): the USB identity is unknown.
    Unknown,
}

/// A dongle that finished its initialisation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BleCentral {
    /// `COM5`, `/dev/ttyACM0`, `/dev/ttyS2`.
    pub port_name: String,
    pub mac: Mac,
    pub kind: CentralKind,
    /// Firmware version, `major.minor.patch`.
    pub version: String,
    /// Firmware build number.
    pub build: u16,
    /// Simultaneous connections the dongle supports.
    pub max_conn: u8,
    /// Instruments bonded to this dongle, kept up to date
    /// ([`Event::BondedMacsListChanged`]).
    pub bonded_macs: Vec<Mac>,
    /// Instruments placed on this dongle, linked or not, in MAC order: those
    /// whose last [`Event::CentralToConnectChanged`] named it. Empty in
    /// [`Event::CentralAdded`] (nothing is placed on a dongle before it is added).
    pub peripherals: Vec<Mac>,
}

impl BleCentral {
    /// As many instruments placed as connections; a further
    /// [`crate::BleManager::start_connect_on`] gets `Rejected { CentralFull }`.
    pub fn is_full(&self) -> bool {
        self.peripherals.len() >= usize::from(self.max_conn)
    }
}

/// What an advertiser is, from its name.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum DeviceUsageType {
    /// No name heard yet.
    #[default]
    Unknown = 0,
    /// Any other name.
    Instrument = 1,
    /// A name starting with `SYSW` (Sylvac foot switch).
    Pedal = 2,
}

/// Bond mode an instrument advertises.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BondMode {
    /// PAIR profile, bonded to some central (the name is then often only `SY`).
    BondableBonded,
    /// PAIR profile, not bonded yet.
    BondableNotBonded,
    /// SIMPLE profile.
    NonBondable,
    /// No flags byte heard yet, or a value the protocol does not define.
    Undefined,
}

/// A device heard by the scanning dongle.
#[derive(Debug, Clone, PartialEq)]
pub struct BleAdvertiser {
    pub mac: Mac,
    /// Last name advertised (scan responses often carry none; the last one is kept).
    pub name: Option<String>,
    /// Signal strength of the last packet, dBm.
    pub rssi: i8,
    /// Advertised value (`raw × 0.0001`, 2 to 4 decimals, `.` as separator), when the device
    /// sends one.
    pub value: Option<String>,
    /// The same value as a number.
    pub float_value: Option<f64>,
    /// Battery level, %, when advertised.
    pub battery: Option<u8>,
    /// Article number (DIS Model Number String) when advertised, the last one kept.
    pub article_number: Option<String>,
    pub bond_mode: BondMode,
    /// The scanning dongle holds a bond for this device.
    pub bonded_to_dongle: bool,
    /// When its last packet was heard. [`Event::AdvertiserChanged`] is not sent for this alone:
    /// [`crate::BleManager::advertisers`] has the latest.
    pub date_received: SystemTime,
}

impl BleAdvertiser {
    /// The device advertises a bond with some central
    /// ([`BondMode::BondableBonded`]).
    pub fn is_bonded(&self) -> bool {
        self.bond_mode == BondMode::BondableBonded
    }

    /// Bonded, but not to the scanning dongle; connecting it from this dongle cannot encrypt
    /// the link.
    pub fn is_bonded_to_another(&self) -> bool {
        self.is_bonded() && !self.bonded_to_dongle
    }

    /// [`DeviceUsageType::Pedal`] for a name starting with `SYSW`,
    /// [`DeviceUsageType::Unknown`] before a name is heard, else
    /// [`DeviceUsageType::Instrument`].
    pub fn device_usage_type(&self) -> DeviceUsageType {
        match self.name.as_deref() {
            None | Some("") => DeviceUsageType::Unknown,
            Some(name) if name.starts_with("SYSW") => DeviceUsageType::Pedal,
            Some(_) => DeviceUsageType::Instrument,
        }
    }

    /// A battery level is advertised, above 0 and below 20 %.
    pub fn is_low_battery(&self) -> bool {
        self.battery.is_some_and(|b| b > 0 && b < 20)
    }
}

/// Unit of an instrument value (SDS Parameters bits 15-12).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SdsUnit {
    /// Code 0, and also the reserved code 3 and codes out of range: a unit the protocol does
    /// not define. The value is then the raw measurement times 10^(CPFD exponent), i.e. metres
    /// for a length instrument.
    Undefined = 0,
    Millimeter = 1,
    Inch = 2,
    Radian = 4,
    Degree = 5,
    /// Degrees, shown as degrees and minutes by the instrument.
    DegreeMinute = 6,
}

/// Measuring mode of an instrument (SDS Parameters bits 1-0).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SdsMode {
    Normal = 0,
    Min = 1,
    Max = 2,
    Delta = 3,
}

/// Resolution class an instrument displays. A code the
/// protocol does not define is reported as `Undefined`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum SdsResolution {
    /// Not given by the instrument: its values are held until it is.
    #[default]
    Undefined = 0,
    /// 100 µm / 5 mil.
    Minus1 = 1,
    /// 10 µm / 0.5 mil / 0.01° / 0.0001 rad.
    Minus2 = 2,
    /// 1 µm / 0.05 mil.
    Minus3 = 3,
    /// 0.1 µm / 0.005 mil.
    Minus4 = 4,
    /// 0.01 µm.
    Minus5 = 5,
}

/// How the instrument measures, from its SDS Parameters, reported by
/// [`Event::MeasurementParamsChanged`] and [`crate::BlePeripheral::measurement_params`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MeasuringParams {
    /// The unit of [`Event::NumericalValueChanged`] values.
    pub unit: SdsUnit,
    pub measuring_mode: SdsMode,
    /// Decimals of the values: the resolution class, + 2 in inch and
    /// radian, at most 15.
    pub numerical_resolution: u32,
    pub resolution_class: SdsResolution,
}

impl MeasuringParams {
    /// `value` with exactly [`Self::numerical_resolution`] decimals, never `-0.000`: the text
    /// the instrument shows for a value of [`Event::NumericalValueChanged`]
    /// (`sylvac_ble_format_value`). A non-finite value gives `NaN`, `inf` or `-inf`.
    pub fn format(&self, value: f64) -> String {
        crate::convert::format_value(self, value)
    }
}

/// Encryption state of an instrument's link.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct PairingStatus {
    /// The link is encrypted.
    pub is_paired: bool,
    /// The dongle keeps the keys for the next links.
    pub is_bonded: bool,
}

/// Severity of an [`Event::LibLog`], and the threshold [`crate::BleConfig::log_level`]. A
/// message is sent when its level is at or below the threshold;
/// `None` as threshold sends none.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LogLevel {
    /// As a threshold: no messages. Never the level of a message.
    None = 0,
    Error = 1,
    #[default]
    Warning = 2,
    Info = 3,
    Debug = 4,
}

impl LogLevel {
    /// A message at `self` passes the threshold `threshold`.
    pub fn passes(self, threshold: LogLevel) -> bool {
        self != LogLevel::None && self <= threshold
    }
}

/// How far the link of an instrument has got.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ConnectionStep {
    /// Not connected.
    #[default]
    None = 0,
    /// Bluetooth link established.
    Established = 1,
    /// Discovering the services.
    Discovering = 2,
    /// Services discovered (bonding, when needed, comes next).
    Discovered = 3,
    /// Subscribing to the characteristics.
    Subscribing = 4,
    /// Subscribed: values arrive and commands can be sent.
    Subscribed = 5,
}

/// Why an instrument link went down.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DisconnectReason {
    /// [`crate::BlePeripheral::start_disconnect`] or [`crate::BleManager::stop`].
    Requested,
    /// The link dropped; BGAPI reason code (`0x0208` supervision timeout, `0x0213` remote
    /// user terminated, …).
    Link(u16),
    /// The dongle holding it went away (unplugged, port error).
    DongleLost,
    /// A reason added by a library newer than this crate (`SYLVAC_BLE_DISCONNECT_*` number).
    Other(u32),
}

/// Why a request about an instrument was not carried out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RejectReason {
    /// Every ready dongle already holds as many instruments as it can.
    AllCentralsFull,
    /// The instrument is not connected through this manager ([`crate::BleManager::start_connect`] first).
    NotConnected,
    /// The instrument is connected but not ready yet (not `Subscribed` since its last
    /// link), so the command was not sent.
    NotReady,
    /// The dongle refused the request (see the accompanying `LibLog`).
    CentralRefused,
    /// The dongle chosen with [`crate::BleManager::start_connect_on`] already holds as many
    /// instruments as it can ([`BleCentral::is_full`]).
    CentralFull,
    /// The dongle chosen with [`crate::BleManager::start_connect_on`] (or given to
    /// [`crate::BleManager::delete_bond`]) is not a ready dongle.
    UnknownCentral,
    /// [`crate::BleManager::delete_bond`]: the dongle holds no bond for the instrument.
    NoBond,
    /// A reason added by a library newer than this crate (`SYLVAC_BLE_REJECT_*` number).
    Other(u32),
}

/// Everything the manager reports. Delivered in order through
/// [`crate::BleManager::next_event`] or the callback.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Event {
    /// A dongle is ready to use.
    CentralAdded(BleCentral),
    /// A ready dongle went away; `error` says why when it was not a requested stop.
    CentralRemoved {
        port_name: String,
        mac: Mac,
        error: Option<String>,
    },
    /// A dongle port could not be opened or initialised (busy port, not a dongle…). Retried every
    /// 5 s while the port is present; reported again only when the error changes.
    CentralFailed { port_name: String, error: String },
    /// A device heard while scanning: new, changed (name, value, bond state), or refreshed at
    /// most once a second.
    AdvertiserChanged(BleAdvertiser),
    /// Not heard for 60 s.
    AdvertiserRemoved { mac: Mac },
    /// An instrument joined the manager's list ([`crate::BleManager::peripherals`]): a
    /// [`crate::BleManager::start_connect`] / [`crate::BleManager::start_connect_on`] accepted
    /// for an instrument not in it.
    /// Sent before any other event about the instrument.
    PeripheralAdded { peripheral: Mac },
    /// An instrument left the manager's list: [`crate::BlePeripheral::start_disconnect`],
    /// [`crate::BleManager::reset_connections`], [`crate::BleManager::stop`], or no dongle could
    /// take it ([`Event::Rejected`] with [`RejectReason::AllCentralsFull`] after its
    /// [`Event::PeripheralAdded`]). The last event about the instrument until it is asked for
    /// again.
    PeripheralRemoved { peripheral: Mac },
    /// A link attempt started (repeated by the automatic reconnection).
    Connecting { peripheral: Mac, central: Mac },
    /// The dongle the instrument is assigned to changed: its first placement, a reassignment
    /// after a dongle loss, or `None` while it waits for a free slot or for its own dongle to
    /// come back, and when it is disconnected. `old_value` is the
    /// previous one.
    CentralToConnectChanged {
        peripheral: Mac,
        old_value: Option<Mac>,
        new_value: Option<Mac>,
    },
    /// The bonds of a dongle changed: `central` is the dongle, `added` / `removed` the bonds
    /// gained and lost; the new list is its [`BleCentral::bonded_macs`].
    BondedMacsListChanged {
        central: Mac,
        added: Vec<Mac>,
        removed: Vec<Mac>,
    },
    /// The link reached `value`: `Established`, `Discovering`,
    /// `Discovered`, `Subscribing`, `Subscribed` in order, each once per link, then `None` when
    /// the link goes down (right before [`Event::Disconnected`], which is only sent for a link
    /// that reached `Subscribed`). At `Subscribed` values arrive and commands can be sent. The
    /// dongle is [`crate::BlePeripheral::central_to_connect_mac`]; live values:
    /// [`crate::BlePeripheral::has_live_values`].
    ConnectionStepChanged {
        peripheral: Mac,
        value: ConnectionStep,
    },
    /// The link's encryption state changed. Kept when the link goes down; a bonding failure sets
    /// both flags to false.
    PairingStatusChanged {
        peripheral: Mac,
        value: PairingStatus,
    },
    /// The instrument's measuring parameters became known or changed. Always before the first
    /// [`Event::NumericalValueChanged`] in them. Kept across links.
    MeasurementParamsChanged {
        peripheral: Mac,
        value: MeasuringParams,
    },
    /// Live value, in the unit of the last [`Event::MeasurementParamsChanged`] and rounded half
    /// to even to its `numerical_resolution`; [`MeasuringParams::format`] shows it as the
    /// instrument does. The value is not filtered: an SY289 with firmware r4.10r sends 100 x the
    /// real value as its first measurement after each connection.
    NumericalValueChanged { peripheral: Mac, value: f64 },
    /// The instrument's Data button was pressed: `value` is the line as received.
    InstrumentButtonPressed { peripheral: Mac, value: String },
    /// A reply line (`value`). `command` is the command it
    /// answers (the oldest one without a reply), or `None` when no command was waiting.
    InstrumentResponseReceived {
        peripheral: Mac,
        value: String,
        command: Option<String>,
    },
    /// No reply within the reply timeout. Setting commands (`MM`, `IN`, …) are normally silent,
    /// so this is their usual outcome.
    InstrumentResponseTimeout { peripheral: Mac, command: String },
    /// The link went down before the command was answered.
    InstrumentCommandDropped { peripheral: Mac, command: String },
    /// The link went down. `reconnecting`: the manager keeps trying (until
    /// [`crate::BlePeripheral::start_disconnect`]).
    Disconnected {
        peripheral: Mac,
        reason: DisconnectReason,
        reconnecting: bool,
    },
    /// A request about this instrument was not carried out.
    Rejected {
        peripheral: Mac,
        reason: RejectReason,
    },
    /// Something worth logging: `Error` (the event callback panicked, serial ports cannot be
    /// listed) or `Warning` (mount or bonding failure, dongle protocol error,
    /// dongle command without response, request refused). Only levels that pass
    /// [`crate::BleConfig::log_level`] are sent; `level` is never [`LogLevel::None`].
    LibLog { level: LogLevel, message: String },
    /// The event queue was full: `dropped` events were discarded (advertisements first).
    Overflow { dropped: u64 },
    /// An event added by a library newer than this crate (`SYLVAC_BLE_EVENT_*` number): update the
    /// crate to read it.
    Unknown { kind: u32 },
}
