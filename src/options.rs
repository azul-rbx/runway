/*
 * Copyright (c) 2024 Paradoxum Games
 * This file is licensed under the Mozilla Public License (MPL-2.0). A copy of it is available in the 'LICENSE' file at the root of the repository.
 * This file incorporates changes from rojo-rbx/tarmac, which is licensed under the MIT license.
 *
 * Copyright (c) 2020 Roblox Corporation
 * Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:
 * The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
*/

use crate::commands::Command;
use clap::Parser;
use secrecy::SecretString;

#[derive(Debug, Parser)]
#[clap(about = env!("CARGO_PKG_DESCRIPTION"))]
pub struct Options {
    #[command(flatten)]
    pub global: Global,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Parser)]
pub struct Global {
    /// The authentication cookie for Tarmac to use. If not specified, Tarmac
    /// will attempt to use the cookie from the Roblox Studio installation on
    /// the system.
    #[clap(long, global(true), conflicts_with("api_key"))]
    pub auth: Option<SecretString>,

    /// The Open Cloud API key for Tarmac to use. If not specified, Tarmac
    /// will attempt to use the API key stored in the environment variable
    /// 'TARMAC_API_KEY'.
    #[clap(
        long,
        global(true),
        env("TARMAC_API_KEY"),
        hide_env_values(true),
        conflicts_with("auth")
    )]
    pub api_key: Option<SecretString>,

    /// Sets verbosity level. Can be specified multiple times to increase the verbosity
    /// of this program.
    #[clap(long = "verbose", short, global(true), action(clap::ArgAction::Count))]
    pub verbosity: u8,
}
