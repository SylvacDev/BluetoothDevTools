//! From nothing to live measurements: find the dongle, list the instruments around, connect one,
//! ask it two questions and show its value as it changes.
//!
//! ```text
//! cargo run --example quickstart                    choose among the instruments heard
//! cargo run --example quickstart -- D216182F6CCC    connect this one
//! ```
//!
//! No async code and no thread of its own: the calls of steps 1 to 3 wait for their result, and
//! step 4 gives the events to a closure.

use std::cmp::Reverse;
use std::error::Error;
use std::io::{self, Write};
use std::process::ExitCode;
use std::time::Duration;

use sylvac_ble::{BleConfig, BleManager, ConnectionStep, Event, Mac, SdsUnit};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    // 1. Start the library. It finds the dongles by itself, also those plugged in later.
    let manager = BleManager::start(BleConfig::default())?;
    println!("Looking for a Sylvac dongle...");
    let dongle = loop {
        match manager.next_event(Duration::from_secs(10)) {
            Some(Event::CentralAdded(dongle)) => break dongle,
            Some(Event::CentralFailed { port_name, error }) => println!("  {port_name}: {error}"),
            Some(_) => {}
            None => return Err("no dongle found: plug in a BLED112 or a Bt5Smart".into()),
        }
    };
    println!(
        "  {:?} on {}, firmware {}, up to {} instruments",
        dongle.kind, dongle.port_name, dongle.version, dongle.max_conn
    );

    // 2. Choose an instrument.
    let mac = match std::env::args().nth(1) {
        Some(text) => text.parse()?,
        None => choose(&manager)?,
    };

    // 3. Connect it and talk to it. From here on the library keeps it connected.
    println!("Connecting {mac}...");
    let instrument = manager.connect(mac, Duration::from_secs(30))?;
    println!("  connected");
    for command in ["ID?", "?"] {
        match instrument.send_and_wait(command, Duration::from_secs(3)) {
            Ok(reply) => println!("  {command:<3} -> {reply}"),
            Err(e) => println!("  {command:<3} -> ({e})"),
        }
    }

    // 4. Live values. The closure runs on the library thread, so it only prints.
    println!("Measure, press the Data button, switch the instrument off and on. Enter to quit.");
    let mut link_lost = false;
    manager.set_callback(move |event| match event {
        Event::NumericalValueChanged {
            value,
            measurement_params: p,
            ..
        } => {
            // The text the instrument displays, rewritten in place.
            print!("\r  {:>14} {:<4}", p.format(value), unit(p.unit));
            let _ = io::stdout().flush();
        }
        Event::InstrumentButtonPressed { value, .. } => note(&format!("Data button: {value}")),
        Event::Disconnected {
            reason,
            reconnecting: true,
            ..
        } => {
            link_lost = true;
            note(&format!("link lost ({reason:?}), reconnecting..."));
        }
        Event::ConnectionStepChanged {
            value: ConnectionStep::Subscribed,
            ..
        } if link_lost => {
            link_lost = false;
            note("connected again");
        }
        _ => {}
    })?;
    io::stdin().read_line(&mut String::new())?;

    // Disconnects the instrument and closes the dongle (dropping the manager does the same).
    manager.stop();
    println!();
    Ok(())
}

/// Lists the Sylvac instruments heard, nearest first, until one is chosen.
fn choose(manager: &BleManager) -> Result<Mac, Box<dyn Error>> {
    let scan = manager.scan();
    scan.set_sylvac_only(true)?;
    scan.start()?;
    let mac = loop {
        println!("Listening for instruments...");
        std::thread::sleep(Duration::from_secs(3));
        let mut heard = scan.advertisers();
        heard.sort_by_key(|a| Reverse(a.rssi));
        for (i, a) in heard.iter().enumerate() {
            let number = i + 1;
            let battery = a.battery.map(|b| format!("{b} %")).unwrap_or_default();
            print!(
                "{:>3}  {:<14} {}  {:>4} dBm  {:>5}  {:>10}",
                number,
                a.name.as_deref().unwrap_or("-"),
                a.mac,
                a.rssi,
                battery,
                a.value.as_deref().unwrap_or("")
            );
            if a.is_bonded_to_another() {
                print!("  bonded to another dongle");
            }
            println!();
        }
        if heard.is_empty() {
            println!("  none: wake an instrument up (press a button), it advertises for a while");
        }
        print!("Instrument number (Enter to listen again): ");
        io::stdout().flush()?;
        let mut line = String::new();
        if io::stdin().read_line(&mut line)? == 0 {
            return Err("no instrument chosen".into());
        }
        match line.trim().parse::<usize>() {
            Ok(n) if (1..=heard.len()).contains(&n) => break heard[n - 1].mac,
            _ => {}
        }
    };
    scan.stop()?;
    Ok(mac)
}

/// A line of its own, in place of the live value.
fn note(text: &str) {
    println!("\r  {text:<40}");
}

fn unit(unit: SdsUnit) -> &'static str {
    match unit {
        SdsUnit::Millimeter => "mm",
        SdsUnit::Inch => "in",
        SdsUnit::Radian => "rad",
        SdsUnit::Degree | SdsUnit::DegreeMinute => "°",
        SdsUnit::Undefined => "",
    }
}
