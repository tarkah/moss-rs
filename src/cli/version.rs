// SPDX-FileCopyrightText: Copyright © 2020-2023 Serpent OS Developers
//
// SPDX-License-Identifier: MPL-2.0

use clap::Command;

/// Construct the Version command
pub fn version_command() -> Command {
    Command::new("version").about("Display version and exit")
}

/// Print program version
pub fn print_version() {
    println!("TODO: Set a version");
}
