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

mod alpha_bleed;
mod asset_name;
mod auth_cookie;
mod codegen;
mod commands;
mod data;
mod dpi_scale;
mod glob;
mod lua_ast;
mod options;
mod roblox_api;
mod sync_backend;

use std::{env, panic, process};

use anyhow::{anyhow, Result};
use backtrace::Backtrace;
use clap::Parser;
use tokio::signal;

use crate::commands::Command;
use crate::options::Options;

async fn run(options: Options) -> Result<(), anyhow::Error> {
    let _ = match options.command {
        Command::UploadImage(sub_options) => {
            commands::upload_image(options.global, sub_options).await
        }
        Command::DownloadImage(sub_options) => {
            commands::download_image(options.global, sub_options).await
        }
        Command::Sync(_) => {
            // commands::sync(options.global, sync_options)?,
            Err(anyhow!("unfinished"))
        }
        Command::CreateCacheMap(sub_options) => {
            commands::create_cache_map(options.global, sub_options).await
        }
        Command::AssetList(sub_options) => commands::asset_list(options.global, sub_options).await,
    }?;

    Ok(())
}

#[tokio::main]
async fn main() {
    panic::set_hook(Box::new(|panic_info| {
        // PanicInfo's payload is usually a &'static str or String.
        // See: https://doc.rust-lang.org/beta/std/panic/struct.PanicInfo.html#method.payload
        let message = match panic_info.payload().downcast_ref::<&str>() {
            Some(&message) => message.to_string(),
            None => match panic_info.payload().downcast_ref::<String>() {
                Some(message) => message.clone(),
                None => "<no message>".to_string(),
            },
        };

        eprintln!("Tarmac crashed!");
        eprintln!("This is probably a Tarmac bug.");
        eprintln!("");
        eprintln!(
            "Please consider filing an issue: {}/issues",
            env!("CARGO_PKG_REPOSITORY")
        );
        eprintln!("");
        eprintln!("If you can reproduce this crash, try adding the -v, -vv, or -vvv flags.");
        eprintln!("This might give you more information to figure out what went wrong!");
        eprintln!("");
        eprintln!("Details: {}", message);

        if let Some(location) = panic_info.location() {
            eprintln!("in file {} on line {}", location.file(), location.line());
        }

        // When using the backtrace crate, we need to check the RUST_BACKTRACE
        // environment variable ourselves. Once we switch to the (currently
        // unstable) std::backtrace module, we won't need to do this anymore.
        let should_backtrace = env::var("RUST_BACKTRACE")
            .map(|var| var == "1")
            .unwrap_or(false);

        if should_backtrace {
            eprintln!("{:?}", Backtrace::new());
        } else {
            eprintln!(
                "note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace."
            );
        }

        process::exit(1);
    }));

    let options = Options::parse();

    let log_filter = match options.global.verbosity {
        0 => "info",
        1 => "info,tarmac=debug",
        2 => "info,tarmac=trace",
        _ => "trace",
    };

    let log_env = env_logger::Env::default().default_filter_or(log_filter);

    env_logger::Builder::from_env(log_env)
        .format_module_path(false)
        .format_timestamp(None)
        // Indent following lines equal to the log level label, like `[ERROR] `
        .format_indent(Some(8))
        .init();

    tokio::select! {
        result = run(options) => {
            if let Err(err) = result {
                log::error!("command exited with error {err:?}");
                process::exit(1);
            }
        },
        _ = signal::ctrl_c() => {
            log::info!("caught ctrl-c, exiting now");
            process::exit(0);
        }
    }
}
