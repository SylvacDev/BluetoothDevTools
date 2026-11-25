//! Prints every event of the Sylvac BLE SDK, one timestamped line each, and connects the
//! instruments given on the command line. Its output is plain lines, so it can be redirected to
//! a file.
//!
//! ```text
//! cargo run --example monitor -- --scan --sylvac-only --connect D216182F6CCC
//! ```
//!
//! ```text
//! monitor [--connect <MAC>]... [--port <port>]... [--no-hot-plug] [--reassign]
//!         [--sylvac-only] [--scan] [--query-every <ms>] [--duration <s>]
//! ```
//!
//! While it runs, type a line and Enter:
//! `q` quit · `l` list dongles, instruments, advertisers · `scan on|off|clear` ·
//! `c <MAC>` connect · `d <MAC>` disconnect · `s <command>` send to every ready instrument.

use std::collections::{BTreeMap, BTreeSet};
use std::io::BufRead;
use std::process::ExitCode;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use sylvac_ble::{BleConfig, BleManager, ConnectionStep, Event, Mac, MeasuringParams};

const USAGE: &str = "usage: monitor [--connect <MAC>]... [--port <port>]... [--no-hot-plug] \
[--reassign] [--sylvac-only] [--scan] [--query-every <ms>] [--duration <s>]";

struct Args {
    config: BleConfig,
    connect: Vec<Mac>,
    scan: bool,
    query_every: Option<Duration>,
    duration: Option<Duration>,
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args {
        config: BleConfig::default(),
        connect: Vec::new(),
        scan: false,
        query_every: None,
        duration: None,
    };
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        let mut value = || it.next().ok_or_else(|| format!("{arg} needs a value"));
        match arg.as_str() {
            "--connect" => args
                .connect
                .push(value()?.parse().map_err(|e| format!("{e}"))?),
            "--port" => args.config.ports.push(value()?),
            "--no-hot-plug" => args.config.hot_plug = false,
            "--reassign" => args.config.auto_reassign = true,
            "--sylvac-only" => args.config.sylvac_only = true,
            "--scan" => args.scan = true,
            "--query-every" => {
                let ms: u64 = value()?
                    .parse()
                    .map_err(|_| "--query-every: milliseconds")?;
                args.query_every = Some(Duration::from_millis(ms));
            }
            "--duration" => {
                let s: u64 = value()?.parse().map_err(|_| "--duration: seconds")?;
                args.duration = Some(Duration::from_secs(s));
            }
            _ => return Err(format!("unknown argument {arg}")),
        }
    }
    Ok(args)
}

fn main() -> ExitCode {
    let args = match parse_args() {
        Ok(args) => args,
        Err(e) => {
            eprintln!("{e}\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    let start = Instant::now();
    let say = |text: &str| println!("{:9.3} {text}", start.elapsed().as_secs_f64());

    let manager = match BleManager::start(args.config) {
        Ok(manager) => manager,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };
    say("started; type q + Enter to quit");
    if args.scan {
        let _ = manager.start_scan();
    }
    for mac in &args.connect {
        let _ = manager.start_connect(*mac);
        say(&format!("connect {mac}"));
    }

    let (lines_tx, lines) = mpsc::channel::<String>();
    std::thread::Builder::new()
        .name("stdin".into())
        .spawn(move || {
            for line in std::io::stdin().lock().lines().map_while(Result::ok) {
                if lines_tx.send(line).is_err() {
                    break;
                }
            }
        })
        .expect("stdin thread");

    let mut ready: BTreeSet<Mac> = BTreeSet::new();
    // The parameters of each instrument's values (always reported before them).
    let mut params: BTreeMap<Mac, MeasuringParams> = BTreeMap::new();
    let mut last_query = Instant::now();
    loop {
        if args.duration.is_some_and(|d| start.elapsed() >= d) {
            say("duration reached");
            break;
        }
        if let Some(event) = manager.next_event(Duration::from_millis(100)) {
            let mut line = None;
            match &event {
                Event::ConnectionStepChanged {
                    peripheral: instrument,
                    value,
                } => {
                    if *value == ConnectionStep::Subscribed {
                        ready.insert(*instrument);
                    } else {
                        ready.remove(instrument);
                    }
                }
                Event::MeasurementParamsChanged { peripheral, value } => {
                    params.insert(*peripheral, *value);
                }
                Event::NumericalValueChanged { peripheral, value } => {
                    // The value as the instrument shows it, with its unit.
                    line = params.get(peripheral).map(|p| {
                        format!(
                            "NumericalValueChanged {peripheral} {} {:?} {:?}",
                            p.format(*value),
                            p.unit,
                            p.measuring_mode
                        )
                    });
                }
                Event::Disconnected {
                    peripheral: instrument,
                    ..
                } => {
                    ready.remove(instrument);
                }
                _ => {}
            }
            say(&line.unwrap_or_else(|| format!("{event:?}")));
        }
        if args
            .query_every
            .is_some_and(|every| last_query.elapsed() >= every)
        {
            last_query = Instant::now();
            for mac in &ready {
                let _ = manager.peripheral(*mac).send_command("?");
            }
        }
        match lines.try_recv() {
            Ok(line) => {
                if !command(&manager, &ready, line.trim(), &say) {
                    break;
                }
            }
            Err(mpsc::TryRecvError::Empty) => {}
            // stdin closed (redirected input ended): keep running until --duration or q.
            Err(mpsc::TryRecvError::Disconnected) => {}
        }
    }

    let t = Instant::now();
    manager.stop();
    while let Some(event) = manager.next_event(Duration::ZERO) {
        say(&format!("{event:?}"));
    }
    say(&format!("stopped in {} ms", t.elapsed().as_millis()));
    ExitCode::SUCCESS
}

/// One typed line; `false` to quit.
fn command(manager: &BleManager, ready: &BTreeSet<Mac>, line: &str, say: &impl Fn(&str)) -> bool {
    let (verb, rest) = line.split_once(' ').unwrap_or((line, ""));
    let mac = || rest.trim().parse::<Mac>();
    match verb {
        "" => {}
        "q" => return false,
        "l" => {
            for d in manager.centrals() {
                say(&format!("dongle {d:?}"));
            }
            for i in manager.peripherals() {
                say(&format!(
                    "instrument {} step={:?} central={:?} {:?} live_values={} value={} {:?}",
                    i.mac(),
                    i.connection_step(),
                    i.central_to_connect_mac(),
                    i.pairing_status(),
                    i.has_live_values(),
                    i.numerical_value(),
                    i.measurement_params()
                ));
            }
            for a in manager.advertisers() {
                say(&format!("advertiser {a:?}"));
            }
        }
        "scan" => {
            let _ = match rest.trim() {
                "off" => manager.stop_scan(),
                "clear" => manager.clear_scan(),
                _ => manager.start_scan(),
            };
        }
        "c" => match mac() {
            Ok(mac) => {
                let _ = manager.start_connect(mac);
            }
            Err(e) => say(&format!("{e}")),
        },
        "d" => match mac() {
            Ok(mac) => {
                let _ = manager.peripheral(mac).start_disconnect();
            }
            Err(e) => say(&format!("{e}")),
        },
        "s" => {
            for mac in ready {
                if let Err(e) = manager.peripheral(*mac).send_command(rest) {
                    say(&format!("{mac}: {e}"));
                }
            }
        }
        _ => say("q | l | scan on|off|clear | c <MAC> | d <MAC> | s <command>"),
    }
    true
}
