//! The real library without hardware: a fixed port that does not exist.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

use sylvac_ble::{
    BleConfig, BleManager, BlePeripheral, ConnectionStep, Error, Event, LogLevel, Mac,
    MeasuringParams, PairingStatus, RejectReason, SdsMode, SdsResolution, SdsUnit,
};

#[cfg(windows)]
const MISSING_PORT: &str = "COM250";
#[cfg(not(windows))]
const MISSING_PORT: &str = "/dev/ttyACM250";

fn missing_port() -> BleConfig {
    BleConfig {
        hot_plug: false,
        ports: vec![MISSING_PORT.into()],
        ..BleConfig::default()
    }
}

fn mac() -> Mac {
    "D609B57D4FF6".parse().unwrap()
}

#[test]
fn versions_and_defaults_come_from_the_library() {
    assert_eq!(sylvac_ble::library_abi_version(), sylvac_ble::ABI_VERSION);
    let version = sylvac_ble::library_version();
    assert_eq!(version.split('.').count(), 3, "{version}");
    let config = BleConfig::default();
    assert!(config.hot_plug && !config.auto_reassign && !config.sylvac_only);
    assert!(config.ports.is_empty());
    assert_eq!(config.response_timeout, Duration::from_secs(3));
    assert_eq!(config.queue_capacity, 10_000);
}

/// `MeasuringParams::format` is the library's `sylvac_ble_format_value`.
#[test]
fn values_are_formatted_by_the_library() {
    let mm = MeasuringParams {
        unit: SdsUnit::Millimeter,
        measuring_mode: SdsMode::Normal,
        numerical_resolution: 3,
        resolution_class: SdsResolution::Minus3,
    };
    assert_eq!(mm.format(8.002), "8.002");
    assert_eq!(mm.format(-0.0), "0.000");
    assert_eq!(mm.format(f64::NAN), "NaN");
    let inch = MeasuringParams {
        unit: SdsUnit::Inch,
        numerical_resolution: 5,
        ..mm
    };
    assert_eq!(inch.format(0.31504), "0.31504");
    // Wider than any instrument value: still written in full.
    assert_eq!(mm.format(1e20), "100000000000000000000.000");
}

#[test]
fn mac_text() {
    let mac: Mac = "d2:16:18:2f:6c:cc".parse().unwrap();
    assert_eq!(mac.0, [0xD2, 0x16, 0x18, 0x2F, 0x6C, 0xCC]);
    assert_eq!(mac.to_string(), "D216182F6CCC");
    assert_eq!(format!("{mac:?}"), "Mac(D216182F6CCC)");
    assert_eq!("D2-16-18-2F-6C-CC".parse::<Mac>(), Ok(mac));
    assert_eq!("D216182F6CCC".parse::<Mac>(), Ok(mac));
    for bad in ["D216182F6C", "", "D216182F6CCG", "D216182F6CC\0C"] {
        let error = bad.parse::<Mac>().unwrap_err();
        assert_eq!(error.0, bad);
        assert!(error.to_string().contains("12 hex digits"), "{error}");
    }
}

#[test]
fn start_errors() {
    let config = BleConfig {
        hot_plug: false,
        ..BleConfig::default()
    };
    assert_eq!(BleManager::start(config).unwrap_err(), Error::NoPorts);

    let config = BleConfig {
        ports: vec!["COM\0 5".into()],
        ..BleConfig::default()
    };
    assert!(matches!(
        BleManager::start(config),
        Err(Error::InvalidArgument(_))
    ));

    let config = BleConfig {
        response_timeout: Duration::from_secs(u64::from(u32::MAX)),
        ..missing_port()
    };
    let error = BleManager::start(config).unwrap_err();
    assert!(matches!(error, Error::InvalidArgument(_)), "{error:?}");
    assert!(error.to_string().contains("response_timeout"), "{error}");
}

#[test]
fn missing_port_fails_once_and_stop_is_bounded() {
    let manager = BleManager::start(missing_port()).unwrap();
    match manager.next_event(Duration::from_secs(5)) {
        Some(Event::CentralFailed { port_name, error }) => {
            assert_eq!(port_name, MISSING_PORT);
            assert!(error.contains(MISSING_PORT), "{error}");
        }
        other => panic!("{other:?}"),
    }
    assert!(manager.centrals().is_empty());
    assert!(manager.advertisers().is_empty());
    manager.start_scan().unwrap();
    manager.clear_scan().unwrap();
    manager.stop_scan().unwrap();
    // sylvac_only: as configured, then as set.
    assert!(!manager.sylvac_only());
    manager.set_sylvac_only(true).unwrap();
    assert!(manager.sylvac_only());
    // A dongle that is not ready: start_scan_on and reset_connections answer with a warning.
    let nowhere: Mac = "F0000000000B".parse().unwrap();
    manager.start_scan_on(nowhere).unwrap();
    manager.reset_connections(nowhere).unwrap();
    manager.stop_scan_on(nowhere).unwrap();
    for _ in 0..2 {
        match manager.next_event(Duration::from_secs(2)) {
            Some(Event::LibLog {
                level: LogLevel::Warning,
                message,
            }) => {
                assert!(message.contains("F0000000000B"), "{message}")
            }
            other => panic!("{other:?}"),
        }
    }

    // Commands still work and are answered by events.
    let instrument = manager
        .start_connect("D216182F6CCC".parse().unwrap())
        .unwrap();
    instrument.send_command("?").unwrap();
    // In the list at once, while it waits for a dongle.
    assert_eq!(
        manager.next_event(Duration::from_secs(1)),
        Some(Event::PeripheralAdded {
            peripheral: instrument.mac()
        })
    );
    assert_eq!(
        manager.next_event(Duration::from_secs(1)),
        Some(Event::Rejected {
            peripheral: instrument.mac(),
            reason: RejectReason::NotReady
        })
    );
    // On a dongle that is not ready (none is): refused, not kept.
    let other: Mac = "D216182F6CCD".parse().unwrap();
    manager
        .start_connect_on("F0000000000A".parse().unwrap(), other)
        .unwrap();
    assert_eq!(
        manager.next_event(Duration::from_secs(1)),
        Some(Event::Rejected {
            peripheral: other,
            reason: RejectReason::UnknownCentral
        })
    );
    let instruments = manager.peripherals();
    assert_eq!(instruments.len(), 1, "waits for a dongle");
    assert_eq!(instruments[0].mac(), instrument.mac());
    assert_eq!(instrument.central_to_connect_mac(), None);
    assert_eq!(instrument.connection_step(), ConnectionStep::None);
    assert!(!instrument.is_connected());
    assert_eq!(instrument.pairing_status(), PairingStatus::default());
    assert!(!instrument.is_paired() && !instrument.has_live_values());
    assert!(instrument.numerical_value().is_nan() && instrument.measurement_params().is_none());
    // auto_reassign: off by default, set through the library thread.
    assert!(!instrument.auto_reassign());
    instrument.set_auto_reassign(true).unwrap();
    let deadline = Instant::now() + Duration::from_secs(2);
    while !instrument.auto_reassign() {
        assert!(Instant::now() < deadline, "auto_reassign not set");
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(instrument.send_command("\r"), Err(Error::EmptyCommand));
    assert!(matches!(
        instrument.send_command("a\0b"),
        Err(Error::InvalidArgument(_))
    ));

    // A timeout is `None`, and the manager is not finished.
    let t = Instant::now();
    assert_eq!(manager.next_event(Duration::from_millis(150)), None);
    assert!(
        t.elapsed() >= Duration::from_millis(140),
        "{:?}",
        t.elapsed()
    );
    assert_eq!(manager.next_event(Duration::ZERO), None);
    assert!(!manager.is_finished());

    let t = Instant::now();
    manager.stop();
    assert!(t.elapsed() < Duration::from_secs(3), "{:?}", t.elapsed());
    assert!(manager.is_finished());
    // The release of the waiting instrument (it leaves the list), then the queue is closed.
    let rest: Vec<Event> =
        std::iter::from_fn(|| manager.next_event(Duration::from_millis(100))).collect();
    assert!(
        rest.iter().all(|e| matches!(
            e,
            Event::Disconnected { .. } | Event::PeripheralRemoved { .. }
        )),
        "{rest:?}"
    );
    assert_eq!(
        rest.last(),
        Some(&Event::PeripheralRemoved {
            peripheral: instrument.mac()
        })
    );
    let t = Instant::now();
    assert_eq!(manager.next_event(Duration::from_secs(5)), None);
    assert!(t.elapsed() < Duration::from_secs(1), "stopped: no wait");

    assert_eq!(instrument.send_command("?"), Err(Error::Stopped));
    assert_eq!(instrument.start_disconnect(), Err(Error::Stopped));
    assert_eq!(manager.start_scan(), Err(Error::Stopped));
    assert_eq!(manager.set_sylvac_only(false), Err(Error::Stopped));
    assert_eq!(manager.start_disconnect_all(), Err(Error::Stopped));
    assert_eq!(manager.start_connect(mac()).unwrap_err(), Error::Stopped);
    assert_eq!(manager.set_callback(|_| {}), Err(Error::Stopped));
    assert_eq!(manager.peripherals().len(), 0);
    manager.stop(); // idempotent
}

#[test]
fn callback_receives_events_on_the_library_thread() {
    let manager = BleManager::start(missing_port()).unwrap();
    let (tx, rx) = mpsc::channel();
    manager
        .set_callback(move |event| {
            let _ = tx.send((std::thread::current().id(), event));
        })
        .unwrap();
    manager.peripheral(mac()).start_disconnect().unwrap();
    // The port failure may come first; the rejection must come.
    let mut threads = Vec::new();
    let mut events = Vec::new();
    while !events.iter().any(|e| matches!(e, Event::Rejected { .. })) {
        let (thread, event) = rx.recv_timeout(Duration::from_secs(5)).unwrap();
        threads.push(thread);
        events.push(event);
    }
    assert!(events.contains(&Event::Rejected {
        peripheral: mac(),
        reason: RejectReason::NotConnected
    }));
    assert!(threads.iter().all(|t| *t != std::thread::current().id()));
    assert!(threads.iter().all(|t| *t == threads[0]), "one thread");
    // While a callback is set the queue stays empty (the port failure also went to it).
    assert_eq!(manager.next_event(Duration::from_millis(300)), None);

    // Back to the queue.
    manager.clear_callback().unwrap();
    manager.peripheral(mac()).start_disconnect().unwrap();
    assert_eq!(
        manager.next_event(Duration::from_secs(5)),
        Some(Event::Rejected {
            peripheral: mac(),
            reason: RejectReason::NotConnected
        })
    );
    assert!(
        rx.try_iter()
            .all(|(_, event)| matches!(event, Event::CentralFailed { .. })),
        "the second rejection did not go to the callback"
    );
    drop(manager); // stops
}

#[test]
fn a_callback_set_later_gets_the_queued_events_first() {
    let manager = BleManager::start(missing_port()).unwrap();
    manager.peripheral(mac()).start_disconnect().unwrap();
    // Let both events (port failure, rejection) reach the queue; nothing takes them.
    std::thread::sleep(Duration::from_millis(500));
    let (tx, rx) = mpsc::channel();
    manager
        .set_callback(move |event| {
            let _ = tx.send(event);
        })
        .unwrap();
    let first = rx.recv_timeout(Duration::from_secs(5)).unwrap();
    let second = rx.recv_timeout(Duration::from_secs(5)).unwrap();
    let both = [first, second];
    assert!(
        both.iter()
            .any(|e| matches!(e, Event::CentralFailed { .. })),
        "{both:?}"
    );
    assert!(
        both.contains(&Event::Rejected {
            peripheral: mac(),
            reason: RejectReason::NotConnected
        }),
        "{both:?}"
    );
    assert_eq!(manager.next_event(Duration::ZERO), None, "queue emptied");
}

#[test]
fn a_panicking_callback_falls_back_to_the_queue() {
    let manager = BleManager::start(missing_port()).unwrap();
    manager.set_callback(|_| panic!("user bug")).unwrap();
    manager.peripheral(mac()).start_disconnect().unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut warned = false;
    while Instant::now() < deadline && !warned {
        if let Some(Event::LibLog { level, message }) =
            manager.next_event(Duration::from_millis(200))
        {
            assert_eq!(level, LogLevel::Error);
            warned = message.contains("panicked") && message.contains("user bug");
        }
    }
    assert!(warned);
    // Events go to the queue again.
    manager.peripheral(mac()).start_disconnect().unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut rejected = false;
    while Instant::now() < deadline && !rejected {
        rejected = matches!(
            manager.next_event(Duration::from_millis(200)),
            Some(Event::Rejected { .. })
        );
    }
    assert!(rejected);
}

#[test]
fn calls_from_the_callback() {
    let manager = Arc::new(BleManager::start(missing_port()).unwrap());
    let (tx, rx) = mpsc::channel();
    let inside = manager.clone();
    manager
        .set_callback(move |event| {
            if matches!(event, Event::Rejected { .. }) {
                // Taking events here would hold up the library thread: refused.
                let none = inside.next_event(Duration::from_secs(5));
                let finished = inside.is_finished();
                let connect = inside
                    .start_connect("D216182F6CCC".parse().unwrap())
                    .map(|_| ());
                inside.stop(); // only asks
                let _ = tx.send((none, finished, connect));
            }
        })
        .unwrap();
    manager.peripheral(mac()).start_disconnect().unwrap();
    let (none, finished, connect) = rx.recv_timeout(Duration::from_secs(5)).unwrap();
    assert_eq!(none, None);
    assert!(!finished);
    assert_eq!(connect, Ok(()));
    // The thread ends by itself.
    let deadline = Instant::now() + Duration::from_secs(5);
    while !manager.is_finished() {
        assert!(Instant::now() < deadline, "not finished");
        std::thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(manager.next_event(Duration::from_secs(5)), None);
    assert_eq!(manager.start_scan(), Err(Error::Stopped));
    manager.stop(); // joins; frees the callback, which holds the other reference
    assert_eq!(Arc::strong_count(&manager), 1);
}

/// Counts the closures alive: every callback given to a manager is dropped when it stops, also
/// the replaced ones and the one that holds an `BlePeripheral` (a reference to the manager).
#[test]
fn callbacks_are_dropped_at_stop() {
    struct Alive(Arc<AtomicUsize>);
    impl Drop for Alive {
        fn drop(&mut self) {
            self.0.fetch_sub(1, Ordering::SeqCst);
        }
    }
    let alive = Arc::new(AtomicUsize::new(0));
    let manager = BleManager::start(missing_port()).unwrap();
    let instrument = manager.peripheral(mac());
    for _ in 0..3 {
        alive.fetch_add(1, Ordering::SeqCst);
        let (token, instrument) = (Alive(alive.clone()), instrument.clone());
        manager
            .set_callback(move |_| {
                let _ = (&token, instrument.mac());
            })
            .unwrap();
    }
    manager.clear_callback().unwrap();
    assert_eq!(
        alive.load(Ordering::SeqCst),
        3,
        "kept until the thread ends"
    );
    drop(instrument);
    drop(manager);
    assert_eq!(alive.load(Ordering::SeqCst), 0);
}

#[test]
fn stop_while_the_callback_is_busy() {
    let manager = BleManager::start(missing_port()).unwrap();
    let count = Arc::new(AtomicUsize::new(0));
    let seen = count.clone();
    manager
        .set_callback(move |_| {
            seen.fetch_add(1, Ordering::SeqCst);
            std::thread::sleep(Duration::from_millis(2));
        })
        .unwrap();
    let instrument = manager.peripheral(mac());
    for _ in 0..200 {
        instrument.start_disconnect().unwrap();
    }
    std::thread::sleep(Duration::from_millis(50));
    let t = Instant::now();
    manager.stop();
    assert!(t.elapsed() < Duration::from_secs(5), "{:?}", t.elapsed());
    let at_stop = count.load(Ordering::SeqCst);
    assert!(at_stop > 0);
    std::thread::sleep(Duration::from_millis(100));
    assert_eq!(count.load(Ordering::SeqCst), at_stop, "no call after stop");
}

#[test]
fn handles_work_from_other_threads() {
    fn check<T: Send + Sync>() {}
    check::<BleManager>();
    check::<BlePeripheral>();

    let manager = Arc::new(BleManager::start(missing_port()).unwrap());
    let instrument = manager.peripheral(mac());
    let workers: Vec<_> = (0..4)
        .map(|_| {
            let (manager, instrument) = (manager.clone(), instrument.clone());
            std::thread::spawn(move || {
                for _ in 0..25 {
                    instrument.start_disconnect().unwrap();
                    let _ = manager.peripherals();
                }
            })
        })
        .collect();
    let taker = {
        let manager = manager.clone();
        std::thread::spawn(move || {
            let mut rejected = 0;
            while rejected < 100 {
                match manager.next_event(Duration::from_secs(5)) {
                    Some(Event::Rejected { .. }) => rejected += 1,
                    Some(_) => {}
                    None => break,
                }
            }
            rejected
        })
    };
    for worker in workers {
        worker.join().unwrap();
    }
    assert_eq!(taker.join().unwrap(), 100);

    // Two threads stop at once: both return only when the library thread has ended.
    let stoppers: Vec<_> = (0..2)
        .map(|_| {
            let manager = manager.clone();
            std::thread::spawn(move || {
                manager.stop();
                manager.is_finished()
            })
        })
        .collect();
    for stopper in stoppers {
        assert!(stopper.join().unwrap());
    }
}

#[test]
fn two_managers_and_a_drop_without_stop() {
    let first = BleManager::start(missing_port()).unwrap();
    let second = BleManager::start(missing_port()).unwrap();
    for manager in [&first, &second] {
        assert!(matches!(
            manager.next_event(Duration::from_secs(5)),
            Some(Event::CentralFailed { .. })
        ));
    }
    let instrument = first.peripheral(mac());
    let t = Instant::now();
    drop(first);
    assert!(t.elapsed() < Duration::from_secs(3), "{:?}", t.elapsed());
    // The handle outlives its manager and says so.
    assert_eq!(instrument.start_disconnect(), Err(Error::Stopped));
    second.peripheral(mac()).start_disconnect().unwrap();
    assert!(matches!(
        second.next_event(Duration::from_secs(5)),
        Some(Event::Rejected { .. })
    ));
}

/// `BleConfig::log_level` is the threshold of `LibLog`: the panicked-callback message (level
/// `Error`) is sent with `Error`, not with `None`; the default threshold is `Warning`.
#[test]
fn log_level_filters_lib_log() {
    assert_eq!(BleConfig::default().log_level, LogLevel::Warning);
    for (threshold, sent) in [(LogLevel::Error, true), (LogLevel::None, false)] {
        let manager = BleManager::start(BleConfig {
            log_level: threshold,
            ..missing_port()
        })
        .unwrap();
        manager.set_callback(|_| panic!("user bug")).unwrap();
        manager.peripheral(mac()).start_disconnect().unwrap();
        let deadline = Instant::now() + Duration::from_secs(2);
        let mut logged = false;
        while Instant::now() < deadline && !logged {
            logged = matches!(
                manager.next_event(Duration::from_millis(100)),
                Some(Event::LibLog { .. })
            );
        }
        assert_eq!(logged, sent, "{threshold:?}");
    }
}

/// The waiting calls (`connect`, `disconnect`, `send_and_wait`) without hardware: refusals with
/// their reason, a timeout, refused from the callback, `Stopped`.
#[test]
fn waiting_calls() {
    let manager = BleManager::start(missing_port()).unwrap();
    assert!(matches!(
        manager.next_event(Duration::from_secs(5)),
        Some(Event::CentralFailed { .. })
    ));
    let wanted: Mac = "D216182F6CCC".parse().unwrap();
    let other: Mac = "D216182F6CCD".parse().unwrap();
    let second = Duration::from_secs(1);
    let p = manager.peripheral(other);
    assert_eq!(
        p.send_and_wait("?", second),
        Err(Error::Rejected(RejectReason::NotConnected))
    );
    assert_eq!(
        manager.next_event(second),
        Some(Event::Rejected {
            peripheral: other,
            reason: RejectReason::NotConnected
        })
    );
    assert_eq!(
        p.disconnect(second),
        Err(Error::Rejected(RejectReason::NotConnected))
    );
    assert_eq!(
        manager
            .connect_on("F0000000000A".parse().unwrap(), other, second)
            .unwrap_err(),
        Error::Rejected(RejectReason::UnknownCentral)
    );
    assert_eq!(
        manager
            .connect(wanted, Duration::from_millis(300))
            .unwrap_err(),
        Error::Timeout
    );
    let instrument = manager.peripheral(wanted);
    assert_eq!(
        instrument.send_and_wait_until("SET?", "\r\n>", second),
        Err(Error::Rejected(RejectReason::NotReady))
    );
    assert_eq!(instrument.disconnect(second), Ok(()));
    // delete_bond on a dongle that is not ready: refused with the reason.
    manager
        .delete_bond("F0000000000A".parse().unwrap(), wanted)
        .unwrap();
    loop {
        match manager.next_event(Duration::from_secs(2)) {
            Some(Event::Rejected {
                reason: RejectReason::UnknownCentral,
                ..
            }) => break,
            Some(_) => {}
            None => panic!("no rejection"),
        }
    }

    let (tx, rx) = std::sync::mpsc::channel();
    let handle = manager.peripheral(other);
    manager
        .set_callback(move |event| {
            if let Event::Rejected { .. } = event {
                let _ = tx.send(handle.send_and_wait("?", Duration::from_secs(1)));
            }
        })
        .unwrap();
    manager.peripheral(other).send_command("?").unwrap();
    assert_eq!(
        rx.recv_timeout(Duration::from_secs(2)).unwrap(),
        Err(Error::WrongThread)
    );
    manager.clear_callback().unwrap();
    manager.stop();
    assert_eq!(manager.connect(wanted, second).unwrap_err(), Error::Stopped);
}
