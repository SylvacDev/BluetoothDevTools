//! Sylvac BLE SDK: connect Sylvac instruments through Sylvac dongles (BLED112, Bt5Smart)
//! from any program, without async code.
//!
//! This crate is a safe wrapper over the prebuilt library `sylvac_ble` (`sylvac_ble.dll`,
//! `libsylvac_ble.so`); see the README for how the build finds it (`SYLVAC_BLE_SDK_DIR`).
//!
//! [`BleManager::start`] starts the library on its own thread: it finds the dongles (hot-plug on
//! Windows, a rescan every second on Linux, or fixed ports), spreads instruments over them,
//! reconnects instruments that drop, and reports everything as [`Event`]s. Calls return at
//! once; their results come back as events, taken with [`BleManager::next_event`] or given to a
//! callback.
//!
//! ```no_run
//! use std::time::Duration;
//! use sylvac_ble::{BleManager, BleConfig, ConnectionStep, Event};
//!
//! let manager = BleManager::start(BleConfig::default())?;
//! let scan = manager.scan();
//! scan.set_sylvac_only(true)?;
//! scan.start()?;
//! let wanted = "D216182F6CCC".parse()?;
//! manager.start_connect(wanted)?;
//! while let Some(event) = manager.next_event(Duration::from_secs(30)) {
//!     match event {
//!         Event::ConnectionStepChanged { peripheral, value: ConnectionStep::Subscribed } => {
//!             manager.peripheral(peripheral).send_command("?")?;
//!         }
//!         Event::NumericalValueChanged { peripheral, value, measurement_params: p } => {
//!             println!("{peripheral}: {} {:?}", p.format(value), p.unit);
//!         }
//!         Event::InstrumentResponseReceived { peripheral, value, .. } => println!("{peripheral} answered {value}"),
//!         _ => {}
//!     }
//! }
//! manager.stop();
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! Threading: [`BleManager`] and [`BlePeripheral`] are `Send + Sync`. A callback runs on the library
//! thread: it must not block (a GUI marshals the event to its UI thread), and calls it makes on
//! the manager are carried out after it returns.

mod convert;
mod types;

use std::cell::Cell;
use std::collections::VecDeque;
use std::ffi::{c_char, c_void, CStr, CString};
use std::fmt;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use sylvac_ble_sys as sys;

pub use crate::types::{
    BleAdvertiser, BleCentral, BondMode, CentralKind, ConnectionStep, DeviceUsageType,
    DisconnectReason, Event, LogLevel, Mac, MacParseError, MacRangeError, MeasuringParams,
    PairingStatus, RejectReason, SdsMode, SdsResolution, SdsUnit,
};

/// ABI version of the C library this crate was written for (major << 16 | minor).
pub const ABI_VERSION: u32 = sys::SYLVAC_BLE_ABI_VERSION;

/// Version of the loaded library, e.g. `"0.1.0"`.
pub fn library_version() -> String {
    // SAFETY: no argument; the result is a static NUL-terminated string.
    unsafe { CStr::from_ptr(sys::sylvac_ble_version()) }
        .to_string_lossy()
        .into_owned()
}

/// ABI version of the loaded library (major << 16 | minor); [`ABI_VERSION`] is this crate's.
pub fn library_abi_version() -> u32 {
    // SAFETY: no argument.
    unsafe { sys::sylvac_ble_abi_version() }
}

/// How the manager runs.
#[derive(Debug, Clone)]
pub struct BleConfig {
    /// Ports that always hold a dongle (`COM5`, `/dev/ttyS2` for a board UART), opened at start
    /// and reopened every 5 s after a failure.
    pub ports: Vec<String>,
    /// Find USB dongles by themselves: hot-plug notifications on Windows, a rescan every second
    /// on Linux.
    pub hot_plug: bool,
    /// The default [`BlePeripheral::auto_reassign`] of each instrument asked for (default
    /// false).
    pub auto_reassign: bool,
    /// Time an instrument has to answer a command before [`Event::InstrumentResponseTimeout`]; the next
    /// command for that instrument is written then. Whole milliseconds.
    pub response_timeout: Duration,
    /// Events kept for [`BleManager::next_event`]; beyond, advertiser events are dropped first and
    /// [`Event::Overflow`] says how many.
    pub queue_capacity: usize,
    /// The most detailed [`Event::LibLog`] level sent; default
    /// [`LogLevel::Warning`], [`LogLevel::None`] sends none.
    pub log_level: LogLevel,
}

fn raw_default_config() -> sys::sylvac_ble_config {
    let mut raw = sys::sylvac_ble_config {
        struct_size: 0,
        hot_plug: 1,
        auto_reassign: 0,
        response_timeout_ms: 3000,
        queue_capacity: 10_000,
        port_count: 0,
        log_level: sys::SYLVAC_BLE_LOG_LEVEL_WARNING,
        reserved: 0,
        ports: ptr::null(),
    };
    // SAFETY: a valid struct to fill. It sets `struct_size` and the library's defaults.
    unsafe { sys::sylvac_ble_config_init(&mut raw) };
    raw
}

impl Default for BleConfig {
    /// The defaults of the loaded library (`sylvac_ble_config_init`).
    fn default() -> Self {
        let raw = raw_default_config();
        Self {
            ports: Vec::new(),
            hot_plug: raw.hot_plug != 0,
            auto_reassign: raw.auto_reassign != 0,
            response_timeout: Duration::from_millis(u64::from(raw.response_timeout_ms)),
            queue_capacity: raw.queue_capacity as usize,
            log_level: convert::log_level(raw.log_level),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// The manager is stopped.
    Stopped,
    /// `hot_plug` is off and no port is given: there is no dongle to look for.
    NoPorts,
    /// The command is empty (after removing a trailing CR/LF).
    EmptyCommand,
    /// The library thread, its runtime or the serial port watcher could not start.
    Start(String),
    /// The library refused an argument (a text with a NUL character, for instance).
    InvalidArgument(String),
    /// The loaded library does not have the ABI version this crate was written for: use the
    /// crate that comes with that library.
    AbiMismatch { library: u32, wrapper: u32 },
    /// A bug in the library, or a status this crate does not know.
    Internal(String),
    /// A waiting call ([`BleManager::connect`], [`BlePeripheral::send_and_wait`]...) from
    /// the event callback: it runs on the library thread, which would wait for itself.
    WrongThread,
    /// A waiting call got no outcome within its timeout. The request goes on.
    Timeout,
    /// The request was refused: [`Event::Rejected`] with this reason.
    Rejected(RejectReason),
    /// The instrument did not answer within [`BleConfig::response_timeout`].
    NoResponse,
    /// The command was dropped before its reply (link down or instrument released).
    CommandDropped,
    /// The instrument was released before it was connected.
    Released,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Stopped => f.write_str("the manager is stopped"),
            Self::NoPorts => {
                f.write_str("no dongle to look for: hot_plug is off and no port is given")
            }
            Self::EmptyCommand => f.write_str("empty command"),
            Self::Start(message) | Self::InvalidArgument(message) | Self::Internal(message) => {
                f.write_str(message)
            }
            Self::WrongThread => {
                f.write_str("cannot wait on the library thread (the event callback)")
            }
            Self::Timeout => f.write_str("no outcome within the timeout"),
            Self::Rejected(reason) => write!(f, "refused: {reason:?}"),
            Self::NoResponse => f.write_str("the instrument did not answer"),
            Self::CommandDropped => {
                f.write_str("the command was dropped (link down or instrument released)")
            }
            Self::Released => f.write_str("the instrument was released before it was connected"),
            Self::AbiMismatch { library, wrapper } => write!(
                f,
                "the sylvac_ble library has ABI version {}.{}, this crate needs {}.{}",
                library >> 16,
                library & 0xFFFF,
                wrapper >> 16,
                wrapper & 0xFFFF
            ),
        }
    }
}

impl std::error::Error for Error {}

/// The error of a call that returned `status`; reads the library's text for it, which is kept
/// per thread, so this runs right after the call.
fn error(status: sys::sylvac_ble_status) -> Error {
    let detail = || {
        // SAFETY: no argument; the text is valid until the next failing call on this thread
        // and is copied here.
        let text = unsafe { CStr::from_ptr(sys::sylvac_ble_last_error()) }
            .to_string_lossy()
            .into_owned();
        if text.is_empty() {
            format!("sylvac_ble status {status}")
        } else {
            text
        }
    };
    match status {
        sys::SYLVAC_BLE_ERR_STOPPED => Error::Stopped,
        sys::SYLVAC_BLE_ERR_NO_PORTS => Error::NoPorts,
        sys::SYLVAC_BLE_ERR_EMPTY_COMMAND => Error::EmptyCommand,
        sys::SYLVAC_BLE_ERR_START => Error::Start(detail()),
        sys::SYLVAC_BLE_ERR_INVALID_ARGUMENT => Error::InvalidArgument(detail()),
        sys::SYLVAC_BLE_ERR_WRONG_THREAD => Error::WrongThread,
        sys::SYLVAC_BLE_TIMEOUT => Error::Timeout,
        // SAFETY: no argument.
        sys::SYLVAC_BLE_ERR_REJECTED => Error::Rejected(convert::reject_reason(unsafe {
            sys::sylvac_ble_last_reject_reason()
        })),
        sys::SYLVAC_BLE_ERR_NO_RESPONSE => Error::NoResponse,
        sys::SYLVAC_BLE_ERR_COMMAND_DROPPED => Error::CommandDropped,
        sys::SYLVAC_BLE_ERR_RELEASED => Error::Released,
        _ => Error::Internal(detail()),
    }
}

fn check(status: sys::sylvac_ble_status) -> Result<(), Error> {
    if status == sys::SYLVAC_BLE_OK {
        Ok(())
    } else {
        Err(error(status))
    }
}

/// A wait in milliseconds for the C library, rounded up, below `SYLVAC_BLE_WAIT_FOREVER`.
fn wait_ms(timeout: Duration) -> u32 {
    u32::try_from(timeout.as_nanos().div_ceil(1_000_000))
        .unwrap_or(u32::MAX)
        .min(sys::SYLVAC_BLE_WAIT_FOREVER - 1)
}

/// The largest reply `sylvac_ble_send_and_wait` writes, with its NUL.
const REPLY_SIZE: usize = 4097;

fn c_text(text: &str, what: &str) -> Result<CString, Error> {
    CString::new(text).map_err(|_| Error::InvalidArgument(format!("{what} holds a NUL character")))
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|e| e.into_inner())
}

thread_local! {
    /// The manager whose callback is running on this thread (its library thread), else NULL.
    static IN_CALLBACK_OF: Cell<*mut sys::sylvac_ble_manager> = const { Cell::new(ptr::null_mut()) };
}

/// A callback given to the library: `user_data` of the C callback points to one of these.
/// Only the library thread reads it, one call at a time, until that thread has ended.
struct Slot {
    callback: Box<dyn FnMut(Event) + Send>,
    inner: *const Inner,
    panicked: bool,
}

/// Owner of the slots; a `Vec` of raw pointers because the C side holds the same pointers.
struct Slots(Vec<*mut Slot>);

// SAFETY: a slot holds a `Send` closure and a pointer to the `Sync` `Inner`; the list itself is
// only touched under its mutex.
unsafe impl Send for Slots {}

impl Slots {
    /// # Safety
    /// The library thread has ended (or never got these slots): nothing calls them any more.
    unsafe fn free(self) {
        for slot in self.0 {
            // SAFETY: made by `Box::into_raw` in `set_callback`, freed once (the list is taken
            // out of its mutex), and no longer used by the library per the caller's contract.
            drop(unsafe { Box::from_raw(slot) });
        }
    }
}

struct Inner {
    handle: *mut sys::sylvac_ble_manager,
    /// Every callback ever set, kept until the library thread has ended: after
    /// `sylvac_ble_set_callback` the previous callback may still be called for a moment.
    slots: Mutex<Slots>,
    /// Events held on this side: what a panicked callback left. Taken first by
    /// [`BleManager::next_event`].
    held: Mutex<VecDeque<Event>>,
    /// [`BleConfig::log_level`], for the messages this crate makes itself.
    log_level: LogLevel,
}

// SAFETY: every call of the C library on a manager is thread-safe, except
// `sylvac_ble_manager_free`, which `Drop` makes when no other reference is left. The other fields
// are `Sync` by themselves.
unsafe impl Send for Inner {}
// SAFETY: as above.
unsafe impl Sync for Inner {}

impl Inner {
    /// Every instrument asked for, with its state (`sylvac_ble_peripherals`).
    fn states(&self) -> Vec<(Mac, convert::PeripheralState)> {
        let mut list = ptr::null_mut();
        // SAFETY: a live handle and a valid place for the list pointer.
        if unsafe { sys::sylvac_ble_peripherals(self.handle, &mut list) } != sys::SYLVAC_BLE_OK {
            return Vec::new();
        }
        // SAFETY: the library handed out this list: `count` valid item pointers; it is read,
        // then freed once.
        unsafe {
            let states = items((*list).items, (*list).count)
                .iter()
                .map(|item| convert::peripheral_state(&**item))
                .collect();
            sys::sylvac_ble_peripheral_list_free(list);
            states
        }
    }

    fn in_own_callback(&self) -> bool {
        IN_CALLBACK_OF.with(Cell::get) == self.handle
    }

    fn stop(&self) {
        if self.in_own_callback() {
            // On the library thread: this only asks; the thread ends after the callback.
            // SAFETY: a live handle.
            unsafe { sys::sylvac_ble_manager_stop(self.handle) };
            return;
        }
        // SAFETY: a live handle. Outside the callback this returns once the library thread has
        // ended, for every caller when several stop at once: the callback is not called after
        // it.
        unsafe { sys::sylvac_ble_manager_stop(self.handle) };
        let slots = std::mem::replace(&mut *lock(&self.slots), Slots(Vec::new()));
        // SAFETY: the library thread has ended. The closures are dropped outside the lock:
        // one may hold an `BlePeripheral`, i.e. a reference to this `Inner`.
        unsafe { slots.free() };
    }
}

impl Drop for Inner {
    fn drop(&mut self) {
        // SAFETY: the handle is live and no other reference to it is left. Stops if needed.
        let status = unsafe { sys::sylvac_ble_manager_free(self.handle) };
        let slots = std::mem::replace(&mut *lock(&self.slots), Slots(Vec::new()));
        if status == sys::SYLVAC_BLE_OK {
            // SAFETY: the manager is freed, so its thread has ended.
            unsafe { slots.free() };
        }
        // Otherwise (the last reference was dropped inside the callback, where the library
        // refuses to free) the handle and the closures are leaked rather than freed under the
        // running thread.
    }
}

/// The C callback: converts the event and calls the closure of its slot.
unsafe extern "C" fn trampoline(event: *const sys::sylvac_ble_event, user_data: *mut c_void) {
    // SAFETY: `user_data` is a slot of `set_callback`, alive until the library thread (the
    // only caller, one call at a time) has ended; its `inner` outlives the manager handle.
    let slot = unsafe { &mut *user_data.cast::<Slot>() };
    // SAFETY: as above.
    let inner = unsafe { &*slot.inner };
    // SAFETY: the library lends a valid event for the call.
    let Some(event) = (unsafe { event.as_ref() }).map(|e| unsafe { convert::event(e) }) else {
        return;
    };
    if slot.panicked {
        // Until the library has taken the callback away.
        lock(&inner.held).push_back(event);
        return;
    }
    let previous = IN_CALLBACK_OF.with(|c| c.replace(inner.handle));
    let result = catch_unwind(AssertUnwindSafe(|| (slot.callback)(event)));
    IN_CALLBACK_OF.with(|c| c.set(previous));
    if let Err(panic) = result {
        slot.panicked = true;
        let message = panic
            .downcast_ref::<&str>()
            .map(|s| (*s).to_owned())
            .or_else(|| panic.downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "unknown panic".to_owned());
        // SAFETY: a live handle; allowed in the callback, carried out after it returns.
        unsafe { sys::sylvac_ble_set_callback(inner.handle, None, ptr::null_mut()) };
        if LogLevel::Error.passes(inner.log_level) {
            lock(&inner.held).push_back(Event::LibLog {
                level: LogLevel::Error,
                message: format!(
                    "the event callback panicked ({message}); events go to the queue again"
                ),
            });
        }
    }
}

/// The library: dongles, scan, instruments, events. Stopped by [`BleManager::stop`] or on drop.
pub struct BleManager {
    inner: Arc<Inner>,
}

impl BleManager {
    /// Starts the library thread, the dongle search and the fixed ports. Returns once they
    /// run; dongles report [`Event::CentralAdded`] about a second later.
    pub fn start(config: BleConfig) -> Result<Self, Error> {
        let (library, wrapper) = (library_abi_version(), ABI_VERSION);
        // Major 0 is the beta: any minor may break. From 1.0 a higher minor only adds.
        let compatible = if wrapper >> 16 == 0 {
            library == wrapper
        } else {
            library >> 16 == wrapper >> 16 && library >= wrapper
        };
        if !compatible {
            return Err(Error::AbiMismatch { library, wrapper });
        }

        let ports = config
            .ports
            .iter()
            .map(|port| c_text(port, "a port name"))
            .collect::<Result<Vec<CString>, Error>>()?;
        let pointers: Vec<*const c_char> = ports.iter().map(|port| port.as_ptr()).collect();
        let too_large = |what: &str| Error::InvalidArgument(format!("{what} is too large"));
        let mut raw = raw_default_config();
        raw.hot_plug = u32::from(config.hot_plug);
        raw.auto_reassign = u32::from(config.auto_reassign);
        raw.response_timeout_ms = u32::try_from(config.response_timeout.as_millis())
            .map_err(|_| too_large("response_timeout"))?;
        raw.queue_capacity =
            u32::try_from(config.queue_capacity).map_err(|_| too_large("queue_capacity"))?;
        raw.log_level = convert::raw_log_level(config.log_level);
        raw.port_count = u32::try_from(pointers.len()).map_err(|_| too_large("ports"))?;
        raw.ports = pointers.as_ptr();

        let mut handle = ptr::null_mut();
        // SAFETY: `raw` is initialised and its `ports` point to `port_count` NUL-terminated
        // strings alive for the call (the library copies them); `handle` is valid for a write.
        check(unsafe { sys::sylvac_ble_manager_start(&raw, &mut handle) })?;
        Ok(Self {
            inner: Arc::new(Inner {
                handle,
                slots: Mutex::new(Slots(Vec::new())),
                held: Mutex::new(VecDeque::new()),
                log_level: config.log_level,
            }),
        })
    }

    /// The next event, waiting up to `timeout` (to the millisecond). `None` on timeout, or
    /// once the manager has stopped and every event was taken. Not used while a callback is
    /// set; inside the callback it returns `None` at once.
    pub fn next_event(&self, timeout: Duration) -> Option<Event> {
        if let Some(event) = lock(&self.inner.held).pop_front() {
            return Some(event);
        }
        // Rounded up, so a wait shorter than a millisecond is still a wait; never the value
        // that means "forever".
        let ms = timeout.as_nanos().div_ceil(1_000_000);
        let ms = u32::try_from(ms)
            .unwrap_or(u32::MAX)
            .min(sys::SYLVAC_BLE_WAIT_FOREVER - 1);
        self.take(ms)
    }

    fn take(&self, timeout_ms: u32) -> Option<Event> {
        let mut raw = ptr::null_mut();
        // SAFETY: a live handle and a valid place for the event pointer.
        let status = unsafe { sys::sylvac_ble_next_event(self.inner.handle, timeout_ms, &mut raw) };
        match status {
            sys::SYLVAC_BLE_OK if !raw.is_null() => {
                // SAFETY: the library handed out this event; it is read, then freed once.
                let event = unsafe { convert::event(&*raw) };
                // SAFETY: as above.
                unsafe { sys::sylvac_ble_event_free(raw) };
                Some(event)
            }
            _ => None,
        }
    }

    /// The library thread has ended and no more events will come: [`BleManager::next_event`] only
    /// returns those still queued. `true` some time after [`BleManager::stop`], also when it was
    /// called from the callback.
    pub fn is_finished(&self) -> bool {
        let mut finished = 0;
        // SAFETY: a live handle and a valid place for the answer.
        let status =
            unsafe { sys::sylvac_ble_manager_is_finished(self.inner.handle, &mut finished) };
        status == sys::SYLVAC_BLE_OK && finished != 0
    }

    /// Events go to `callback`, on the library thread, instead of the queue. Events still in
    /// the queue when the callback takes over are given to it first, in order. If the callback
    /// panics it is removed, events go to the queue again and an [`Event::LibLog`] says so.
    pub fn set_callback(&self, callback: impl FnMut(Event) + Send + 'static) -> Result<(), Error> {
        let slot = Box::into_raw(Box::new(Slot {
            callback: Box::new(callback),
            inner: Arc::as_ptr(&self.inner),
            panicked: false,
        }));
        let mut slots = lock(&self.inner.slots);
        // SAFETY: a live handle; `slot` stays allocated until the library thread has ended.
        let status = unsafe {
            sys::sylvac_ble_set_callback(self.inner.handle, Some(trampoline), slot.cast::<c_void>())
        };
        if status == sys::SYLVAC_BLE_OK {
            slots.0.push(slot);
            Ok(())
        } else {
            drop(slots);
            // SAFETY: the library refused the callback, so it never got the slot.
            drop(unsafe { Box::from_raw(slot) });
            Err(error(status))
        }
    }

    /// Events go to the queue again.
    pub fn clear_callback(&self) -> Result<(), Error> {
        // SAFETY: a live handle.
        check(unsafe { sys::sylvac_ble_set_callback(self.inner.handle, None, ptr::null_mut()) })
    }

    /// [`BlePeripheral::start_disconnect`] for every instrument, in MAC order.
    pub fn start_disconnect_all(&self) -> Result<(), Error> {
        // SAFETY: a live handle.
        check(unsafe { sys::sylvac_ble_start_disconnect_all(self.inner.handle) })
    }

    /// Every instrument placed on the dongle `central` is
    /// released as by [`BlePeripheral::start_disconnect`], the dongle drops all its links and
    /// deletes all its bonds, and its [`BleCentral::bonded_macs`] is emptied. A dongle that is
    /// not ready: an [`Event::LibLog`] warning, nothing changes.
    pub fn reset_connections(&self, central: Mac) -> Result<(), Error> {
        let raw = sys::sylvac_ble_mac {
            le_bytes: central.to_le_bytes(),
        };
        // SAFETY: a live handle and a valid address.
        check(unsafe { sys::sylvac_ble_reset_connections(self.inner.handle, &raw) })
    }

    /// The dongle `central` deletes its bond with `peripheral`; once the dongle confirms,
    /// [`Event::BondedMacsListChanged`]. `Rejected { NoBond }` when it has none,
    /// `Rejected { UnknownCentral }` when `central` is not a ready dongle.
    pub fn delete_bond(&self, central: Mac, peripheral: Mac) -> Result<(), Error> {
        let (raw_central, raw) = (
            sys::sylvac_ble_mac {
                le_bytes: central.to_le_bytes(),
            },
            sys::sylvac_ble_mac {
                le_bytes: peripheral.to_le_bytes(),
            },
        );
        // SAFETY: a live handle and valid addresses.
        check(unsafe { sys::sylvac_ble_delete_bond(self.inner.handle, &raw_central, &raw) })
    }

    /// Connects the instrument on the dongle with the most free slots and keeps it connected
    /// (automatic reconnection) until [`BlePeripheral::start_disconnect`]. With no dongle ready yet it
    /// waits for the first one. Progress: [`Event::PeripheralAdded`] (when not in the list yet),
    /// [`Event::CentralToConnectChanged`],
    /// [`Event::Connecting`], [`Event::ConnectionStepChanged`] up to
    /// [`ConnectionStep::Subscribed`]; refusal: [`Event::Rejected`], then
    /// [`Event::PeripheralRemoved`] when it was added.
    pub fn start_connect(&self, mac: Mac) -> Result<BlePeripheral, Error> {
        let raw = sys::sylvac_ble_mac {
            le_bytes: mac.to_le_bytes(),
        };
        // SAFETY: a live handle and a valid address.
        check(unsafe { sys::sylvac_ble_start_connect(self.inner.handle, &raw) })?;
        Ok(self.peripheral(mac))
    }

    /// [`Self::start_connect`] on the dongle `central` chosen by the caller (an instrument
    /// already asked for stays where it is). Refusal:
    /// [`Event::Rejected`] with [`RejectReason::CentralFull`] when that dongle has no free slot,
    /// [`RejectReason::UnknownCentral`] when it is not a ready dongle.
    pub fn start_connect_on(&self, central: Mac, peripheral: Mac) -> Result<BlePeripheral, Error> {
        let (raw_central, raw) = (
            sys::sylvac_ble_mac {
                le_bytes: central.to_le_bytes(),
            },
            sys::sylvac_ble_mac {
                le_bytes: peripheral.to_le_bytes(),
            },
        );
        // SAFETY: a live handle and valid addresses.
        check(unsafe { sys::sylvac_ble_start_connect_on(self.inner.handle, &raw_central, &raw) })?;
        Ok(self.peripheral(peripheral))
    }

    /// [`BleManager::start_connect`], then waits up to `timeout` for
    /// [`ConnectionStep::Subscribed`] (at once when already subscribed). [`Error::Timeout`]:
    /// the instrument keeps connecting. [`Error::Rejected`], [`Error::Released`],
    /// [`Error::WrongThread`] from the callback.
    pub fn connect(&self, mac: Mac, timeout: Duration) -> Result<BlePeripheral, Error> {
        let raw = sys::sylvac_ble_mac {
            le_bytes: mac.to_le_bytes(),
        };
        // SAFETY: a live handle, NULL (any dongle) and a valid address.
        check(unsafe {
            sys::sylvac_ble_connect(self.inner.handle, ptr::null(), &raw, wait_ms(timeout))
        })?;
        Ok(self.peripheral(mac))
    }

    /// [`BleManager::connect`] on the dongle `central` ([`BleManager::start_connect_on`]).
    pub fn connect_on(
        &self,
        central: Mac,
        peripheral: Mac,
        timeout: Duration,
    ) -> Result<BlePeripheral, Error> {
        let (raw_central, raw) = (
            sys::sylvac_ble_mac {
                le_bytes: central.to_le_bytes(),
            },
            sys::sylvac_ble_mac {
                le_bytes: peripheral.to_le_bytes(),
            },
        );
        // SAFETY: a live handle and valid addresses.
        check(unsafe {
            sys::sylvac_ble_connect(self.inner.handle, &raw_central, &raw, wait_ms(timeout))
        })?;
        Ok(self.peripheral(peripheral))
    }

    /// A handle for an instrument already asked for (e.g. the `instrument` of an event).
    pub fn peripheral(&self, mac: Mac) -> BlePeripheral {
        BlePeripheral {
            mac,
            inner: self.inner.clone(),
        }
    }

    /// Ready dongles.
    pub fn centrals(&self) -> Vec<BleCentral> {
        let mut list = ptr::null_mut();
        // SAFETY: a live handle and a valid place for the list pointer.
        if unsafe { sys::sylvac_ble_centrals(self.inner.handle, &mut list) } != sys::SYLVAC_BLE_OK {
            return Vec::new();
        }
        // SAFETY: the library handed out this list: `count` valid item pointers, whose strings
        // live until the list is freed, once, below.
        unsafe {
            let dongles = items((*list).items, (*list).count)
                .iter()
                .map(|item| convert::central(&**item))
                .collect();
            sys::sylvac_ble_central_list_free(list);
            dongles
        }
    }

    /// The scan: its filter, its start and stop, the list of devices heard.
    pub fn scan(&self) -> BleScan {
        BleScan {
            inner: self.inner.clone(),
        }
    }

    /// Instruments asked for with [`BleManager::start_connect`] and not disconnected
    /// ([`Event::PeripheralAdded`] / [`Event::PeripheralRemoved`]).
    pub fn peripherals(&self) -> Vec<BlePeripheral> {
        self.inner
            .states()
            .into_iter()
            .map(|(mac, _)| BlePeripheral {
                mac,
                inner: self.inner.clone(),
            })
            .collect()
    }

    /// Disconnects every instrument, closes the dongles (bounded, about 1.5 s at most) and ends
    /// the library thread. The last events stay available to [`BleManager::next_event`]. Called
    /// from the callback, it only asks the thread to stop.
    pub fn stop(&self) {
        self.inner.stop();
    }
}

/// The `count` item pointers of a list.
///
/// # Safety
/// `items` is NULL (then `count` is ignored) or points to `count` pointers valid for `'a`.
unsafe fn items<'a, T>(items: *const *const T, count: u32) -> &'a [*const T] {
    if items.is_null() {
        return &[];
    }
    // SAFETY: per the caller's contract.
    unsafe { std::slice::from_raw_parts(items, count as usize) }
}

impl Drop for BleManager {
    fn drop(&mut self) {
        self.stop();
    }
}

/// The scan of a [`BleManager`] ([`BleManager::scan`]): what it lists, when, and the devices
/// heard. A cheap handle: clone it, keep it, send it to other threads. Its calls fail with
/// [`Error::Stopped`] once the manager is stopped.
#[derive(Clone)]
pub struct BleScan {
    inner: Arc<Inner>,
}

impl BleScan {
    /// Whether the scan lists only Sylvac devices (20 name prefixes). Off at start.
    pub fn sylvac_only(&self) -> bool {
        let mut on = 0;
        // SAFETY: a live handle and a valid place for the answer.
        let status = unsafe { sys::sylvac_ble_scan_get_sylvac_only(self.inner.handle, &mut on) };
        status == sys::SYLVAC_BLE_OK && on != 0
    }

    /// Sets [`BleScan::sylvac_only`], before or during a scan. The list is filtered again at
    /// once: an
    /// [`Event::AdvertiserRemoved`] for each device listed that no longer passes, an
    /// [`Event::AdvertiserChanged`] for each device heard in the last minute that now does.
    pub fn set_sylvac_only(&self, on: bool) -> Result<(), Error> {
        // SAFETY: a live handle.
        check(unsafe { sys::sylvac_ble_scan_set_sylvac_only(self.inner.handle, u32::from(on)) })
    }

    /// Lists the devices heard ([`Event::AdvertiserChanged`]), filtered by
    /// [`BleScan::sylvac_only`]. Dongles still scan by themselves while an instrument to connect has no link, whether or not the list is on.
    pub fn start(&self) -> Result<(), Error> {
        // SAFETY: a live handle.
        check(unsafe { sys::sylvac_ble_scan_start(self.inner.handle) })
    }

    /// Stops the list, on whichever dongle scans.
    pub fn stop(&self) -> Result<(), Error> {
        // SAFETY: a live handle.
        check(unsafe { sys::sylvac_ble_scan_stop(self.inner.handle) })
    }

    /// Empties the list of devices heard, with an [`Event::AdvertiserRemoved`]
    /// for each; a device still around is listed again at its next advertisement.
    pub fn clear(&self) -> Result<(), Error> {
        // SAFETY: a live handle.
        check(unsafe { sys::sylvac_ble_scan_clear(self.inner.handle) })
    }

    /// The dongle `central` scans and its advertisements make the list, instead of the dongle
    /// the library chooses. It keeps the scan while it is ready (a lost dongle hands it over and
    /// takes it back when it returns).
    /// [`BleScan::start`] or [`BleScan::stop`] give the choice back to the
    /// library. A dongle that is not ready: an [`Event::LibLog`] warning, nothing changes.
    pub fn start_on(&self, central: Mac) -> Result<(), Error> {
        let raw = sys::sylvac_ble_mac {
            le_bytes: central.to_le_bytes(),
        };
        // SAFETY: a live handle and a valid address.
        check(unsafe { sys::sylvac_ble_scan_start_on(self.inner.handle, &raw) })
    }

    /// When `central` is the scanning dongle, the list stops and the choice goes back to the
    /// library; otherwise nothing.
    pub fn stop_on(&self, central: Mac) -> Result<(), Error> {
        let raw = sys::sylvac_ble_mac {
            le_bytes: central.to_le_bytes(),
        };
        // SAFETY: a live handle and a valid address.
        check(unsafe { sys::sylvac_ble_scan_stop_on(self.inner.handle, &raw) })
    }

    /// Devices heard in the last 60 s while scanning.
    pub fn advertisers(&self) -> Vec<BleAdvertiser> {
        let mut list = ptr::null_mut();
        // SAFETY: a live handle and a valid place for the list pointer.
        if unsafe { sys::sylvac_ble_scan_advertisers(self.inner.handle, &mut list) }
            != sys::SYLVAC_BLE_OK
        {
            return Vec::new();
        }
        // SAFETY: as in `dongles`.
        unsafe {
            let advertisers = items((*list).items, (*list).count)
                .iter()
                .map(|item| convert::advertiser(&**item))
                .collect();
            sys::sylvac_ble_advertiser_list_free(list);
            advertisers
        }
    }
}

/// An instrument asked for with [`BleManager::start_connect`]. A cheap handle: clone it, keep it, send
/// it to other threads. Its calls fail with [`Error::Stopped`] once the manager is stopped.
#[derive(Clone)]
pub struct BlePeripheral {
    mac: Mac,
    inner: Arc<Inner>,
}

impl BlePeripheral {
    pub fn mac(&self) -> Mac {
        self.mac
    }

    /// The step its link has reached: the last
    /// [`Event::ConnectionStepChanged`]; [`ConnectionStep::None`] while not connected, and for
    /// an instrument not asked for (or disconnected).
    ///
    /// The getters read the state the library publishes before it hands out the events of each
    /// change: an event already taken (or given to the callback), or a waiting call that
    /// returned, always shows in them; they can be ahead, showing a change whose event is still
    /// queued.
    pub fn connection_step(&self) -> ConnectionStep {
        self.state().step
    }

    /// Subscribed: values arrive and commands can be sent.
    pub fn is_connected(&self) -> bool {
        self.connection_step() == ConnectionStep::Subscribed
    }

    /// The dongle it is assigned to: the last
    /// [`Event::CentralToConnectChanged`]; `None` while it waits for a dongle or once
    /// disconnected.
    pub fn central_to_connect_mac(&self) -> Option<Mac> {
        self.state().central
    }

    /// The link is encrypted (`pairing_status().is_paired`).
    pub fn is_paired(&self) -> bool {
        self.pairing_status().is_paired
    }

    /// The last [`Event::PairingStatusChanged`], kept when the link goes
    /// down; both false for an instrument not asked for.
    pub fn pairing_status(&self) -> PairingStatus {
        self.state().pairing_status
    }

    /// The last [`Event::MeasurementParamsChanged`], kept across links;
    /// `None` before the instrument sent its SDS Parameters.
    pub fn measurement_params(&self) -> Option<MeasuringParams> {
        self.state().measurement_params
    }

    /// The last [`Event::NumericalValueChanged`], kept across links; NaN
    /// before the first value.
    pub fn numerical_value(&self) -> f64 {
        self.state().numerical_value
    }

    /// The current link reached `Subscribed` and the instrument has the Simple Data Service, so
    /// it sends live values; false at the other steps.
    pub fn has_live_values(&self) -> bool {
        self.state().has_live_values
    }

    fn state(&self) -> convert::PeripheralState {
        self.inner
            .states()
            .into_iter()
            .find(|(mac, _)| *mac == self.mac)
            .map(|(_, state)| state)
            .unwrap_or_default()
    }

    /// Sends an instrument command (`?`, `ID?`, `MM`…; the `\r` is added). Commands to one
    /// instrument are written one at a time, each after the previous reply or its timeout.
    /// Results: [`Event::InstrumentResponseReceived`], [`Event::InstrumentResponseTimeout`], [`Event::InstrumentCommandDropped`], or
    /// [`Event::Rejected`] when the instrument is not ready.
    pub fn send_command(&self, command: &str) -> Result<(), Error> {
        let command = c_text(command, "the command")?;
        let raw = sys::sylvac_ble_mac {
            le_bytes: self.mac.to_le_bytes(),
        };
        // SAFETY: a live handle, a valid address and a NUL-terminated string.
        check(unsafe { sys::sylvac_ble_send_command(self.inner.handle, &raw, command.as_ptr()) })
    }

    /// [`BlePeripheral::send_command`], then waits
    /// up to `timeout` for its reply (also sent to the queue or callback, before this returns).
    /// [`Error::Rejected`] (`NotConnected`, `NotReady`), [`Error::NoResponse`],
    /// [`Error::CommandDropped`], [`Error::Timeout`] (the reply may still come as an event),
    /// [`Error::EmptyCommand`], [`Error::WrongThread`] from the callback.
    pub fn send_and_wait(&self, command: &str, timeout: Duration) -> Result<String, Error> {
        self.send_and_wait_with(command, None, timeout)
    }

    /// [`BlePeripheral::send_and_wait`] for a reply that ends with `end_of_frame` instead
    /// of `\r`: everything before it, as received.
    pub fn send_and_wait_until(
        &self,
        command: &str,
        end_of_frame: &str,
        timeout: Duration,
    ) -> Result<String, Error> {
        self.send_and_wait_with(command, Some(end_of_frame), timeout)
    }

    fn send_and_wait_with(
        &self,
        command: &str,
        end_of_frame: Option<&str>,
        timeout: Duration,
    ) -> Result<String, Error> {
        let command = c_text(command, "the command")?;
        let end_of_frame = end_of_frame
            .map(|e| c_text(e, "the end of frame"))
            .transpose()?;
        let raw = sys::sylvac_ble_mac {
            le_bytes: self.mac.to_le_bytes(),
        };
        let mut reply = vec![0u8; REPLY_SIZE];
        // SAFETY: a live handle, a valid address, NUL-terminated strings (or NULL) and a buffer
        // of `REPLY_SIZE` bytes.
        check(unsafe {
            sys::sylvac_ble_send_and_wait(
                self.inner.handle,
                &raw,
                command.as_ptr(),
                end_of_frame.as_ref().map_or(ptr::null(), |e| e.as_ptr()),
                wait_ms(timeout),
                reply.as_mut_ptr().cast(),
                REPLY_SIZE as u32,
            )
        })?;
        let end = reply.iter().position(|b| *b == 0).unwrap_or(reply.len());
        Ok(String::from_utf8_lossy(&reply[..end]).into_owned())
    }

    /// [`BlePeripheral::start_disconnect`], then waits up to `timeout` for its
    /// [`Event::Disconnected`]. [`Error::Rejected`] with `NotConnected` for an instrument
    /// not asked for; [`Error::WrongThread`] from the callback.
    pub fn disconnect(&self, timeout: Duration) -> Result<(), Error> {
        let raw = sys::sylvac_ble_mac {
            le_bytes: self.mac.to_le_bytes(),
        };
        // SAFETY: a live handle and a valid address.
        check(unsafe { sys::sylvac_ble_disconnect(self.inner.handle, &raw, wait_ms(timeout)) })
    }

    /// When its dongle goes away, it is reconnected on another dongle with a
    /// free slot, unless it is bonded (the other dongle could not encrypt the link); otherwise it
    /// waits for its dongle. Starts as [`BleConfig::auto_reassign`]; false for an instrument not
    /// asked for.
    pub fn auto_reassign(&self) -> bool {
        self.state().auto_reassign
    }

    /// Sets [`Self::auto_reassign`]; a request about an instrument not asked for is answered
    /// by [`Event::Rejected`] with [`RejectReason::NotConnected`].
    pub fn set_auto_reassign(&self, on: bool) -> Result<(), Error> {
        let raw = sys::sylvac_ble_mac {
            le_bytes: self.mac.to_le_bytes(),
        };
        // SAFETY: a live handle and a valid address.
        check(unsafe { sys::sylvac_ble_set_auto_reassign(self.inner.handle, &raw, u32::from(on)) })
    }

    /// Disconnects and stops reconnecting ([`Event::Disconnected`] with
    /// [`DisconnectReason::Requested`]).
    pub fn start_disconnect(&self) -> Result<(), Error> {
        let raw = sys::sylvac_ble_mac {
            le_bytes: self.mac.to_le_bytes(),
        };
        // SAFETY: a live handle and a valid address.
        check(unsafe { sys::sylvac_ble_start_disconnect(self.inner.handle, &raw) })
    }
}

impl fmt::Debug for BlePeripheral {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BlePeripheral")
            .field("mac", &self.mac)
            .finish()
    }
}

impl fmt::Debug for BleScan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BleScan")
            .field("sylvac_only", &self.sylvac_only())
            .finish_non_exhaustive()
    }
}

impl fmt::Debug for BleManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BleManager").finish_non_exhaustive()
    }
}
