// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/15 17:41:58.926129
//+ Revised: 	2026/11/15 17:41:58.926129
// -----------------------------------------------------------------------------

//! Prints an asset's name, made by the same function the updater uses (src/update/target.rs), so
//! the release workflow never spells one out itself.
//!
//!   avalio-lite-asset-name                              this release build's own asset
//!   avalio-lite-asset-name <target> [portable|setup] [--bridge]
//!
//! `--bridge` prints the old name 0.6.0 to 0.10.0 look for. The version is this checkout's.

use std::process::ExitCode;

use avalio_lite::update::target::{Form, TARGETS, asset_name, bridge_asset_name, current_version, this_build};

const USAGE: &str = "usage: avalio-lite-asset-name [<target> [portable|setup] [--bridge]]";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match name(&args) {
        Ok(name) => {
            println!("{name}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("avalio-lite-asset-name: {error}");
            ExitCode::FAILURE
        }
    }
}

fn name(args: &[&str]) -> Result<String, String> {
    let version = current_version();
    let (target, rest) = match args {
        [] => {
            return this_build()
                .asset_name(&version)
                .ok_or_else(|| "this is a build from source, not a release build".to_owned());
        }
        [target, rest @ ..] => (*target, rest),
    };
    if !TARGETS.contains(&target) {
        return Err(format!("{target:?} isn't a release target; the targets are {}", TARGETS.join(", ")));
    }
    let (form, bridge) = match rest {
        [] => (Form::Portable, false),
        [form] if *form == "--bridge" => (Form::Portable, true),
        [form] => (Form::from_word(form).ok_or(USAGE)?, false),
        [form, "--bridge"] => (Form::from_word(form).ok_or(USAGE)?, true),
        _ => return Err(USAGE.to_owned()),
    };
    Ok(if bridge { bridge_asset_name(&version, form) } else { asset_name(&version, target, form) })
}
