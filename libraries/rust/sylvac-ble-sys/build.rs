//! Finds the prebuilt Sylvac BLE library and tells cargo to link it.
//!
//! `SYLVAC_BLE_SDK_DIR` names the folder of the SDK: the library is looked for there and in its
//! `lib`, `bin` and `<target triple>` sub-folders. It is copied into this crate's `OUT_DIR`, which
//! cargo puts on the loader path of `cargo run` and `cargo test`; a program started in another
//! way needs the library next to it (Windows) or on its loader path (Linux).

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const VARIABLE: &str = "SYLVAC_BLE_SDK_DIR";

fn main() {
    println!("cargo:rerun-if-env-changed={VARIABLE}");
    println!("cargo:rerun-if-changed=build.rs");

    let target = env::var("TARGET").unwrap_or_default();
    let windows = env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows");
    let out = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR"));

    let Some(root) = env::var_os(VARIABLE).map(PathBuf::from) else {
        fail(&format!(
            "{VARIABLE} is not set. Set it to the folder of the Sylvac BLE SDK for {target}, the \
             one that holds {}.",
            if windows {
                "sylvac_ble.dll and its import library (sylvac_ble.dll.lib or sylvac_ble.lib)"
            } else {
                "libsylvac_ble.so"
            }
        ));
    };
    let folders = [
        root.clone(),
        root.join("lib"),
        root.join("bin"),
        root.join(&target),
        root.join(&target).join("lib"),
        root.join(&target).join("bin"),
    ];
    let need = |names: &[&str]| -> PathBuf {
        folders
            .iter()
            .flat_map(|folder| names.iter().map(move |name| folder.join(name)))
            .find(|path| path.is_file())
            .unwrap_or_else(|| {
                fail(&format!(
                    "{VARIABLE}={} holds no {} (looked there and in its lib, bin and {target} \
                     sub-folders).",
                    root.display(),
                    names.join(" or "),
                ))
            })
    };

    if windows {
        let dll = need(&["sylvac_ble.dll"]);
        check_architecture(&dll, &target);
        // The linker looks for `sylvac_ble.lib`; cargo names the import library it writes
        // `sylvac_ble.dll.lib`.
        let import = need(&["sylvac_ble.dll.lib", "sylvac_ble.lib"]);
        copy(&import, &out.join("sylvac_ble.lib"));
        copy(&dll, &out.join("sylvac_ble.dll"));
    } else {
        let so = need(&["libsylvac_ble.so", "libsylvac_ble.so.0"]);
        check_architecture(&so, &target);
        // Linked as `libsylvac_ble.so`, loaded by its soname `libsylvac_ble.so.0`.
        copy(&so, &out.join("libsylvac_ble.so"));
        copy(&so, &out.join("libsylvac_ble.so.0"));
    }
    println!("cargo:rustc-link-search=native={}", out.display());
    println!("cargo:rustc-link-lib=dylib=sylvac_ble");
    // For the build script of a crate that depends on this one: DEP_SYLVAC_BLE_LIB_DIR.
    println!("cargo:lib_dir={}", out.display());
}

fn fail(message: &str) -> ! {
    eprintln!("\nerror: sylvac-ble-sys: {message}\n");
    std::process::exit(1);
}

fn copy(from: &Path, to: &Path) {
    println!("cargo:rerun-if-changed={}", from.display());
    if let Err(e) = fs::copy(from, to) {
        fail(&format!(
            "cannot copy {} to {}: {e}",
            from.display(),
            to.display()
        ));
    }
}

/// A library built for another processor only fails at link or load time, with an obscure
/// message: say it here. Reads the machine field of the PE or ELF header.
fn check_architecture(library: &Path, target: &str) {
    let Ok(bytes) = fs::read(library) else {
        return;
    };
    let u16_at = |i: usize| {
        Some(u16::from_le_bytes([
            *bytes.get(i)?,
            *bytes.get(i.checked_add(1)?)?,
        ]))
    };
    let machine = if bytes.starts_with(b"MZ") {
        bytes
            .get(0x3C..0x40)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize)
            .and_then(|pe| u16_at(pe.checked_add(4)?))
            .map(|machine| match machine {
                0x8664 => "x86_64",
                0x014C => "i686",
                0xAA64 => "aarch64",
                _ => "another processor",
            })
    } else if bytes.starts_with(b"\x7FELF") {
        u16_at(18).map(|machine| match machine {
            62 => "x86_64",
            3 => "i686",
            40 => "arm",
            183 => "aarch64",
            _ => "another processor",
        })
    } else {
        None
    };
    let wanted = match target.split('-').next().unwrap_or_default() {
        "i586" => "i686",
        arch if arch.starts_with("arm") || arch.starts_with("thumb") => "arm",
        arch => arch,
    };
    if let Some(machine) = machine {
        if machine != wanted {
            fail(&format!(
                "{} is built for {machine}, but the target is {target}. Point {VARIABLE} to the \
                 SDK for that target.",
                library.display()
            ));
        }
    }
}
