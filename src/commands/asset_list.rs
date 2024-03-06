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

use std::collections::BTreeSet;
use std::env;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

use clap::Args;
use fs_err as fs;

use anyhow::Result;

use crate::data::Manifest;
use crate::options::Global;

#[derive(Debug, Args)]
pub struct AssetListOptions {
    pub project_path: Option<PathBuf>,

    /// A path to a file to put the asset list.
    #[clap(long = "output")]
    pub output: PathBuf,
}

pub async fn asset_list(_: Global, options: AssetListOptions) -> Result<()> {
    let project_path = match options.project_path {
        Some(path) => path,
        None => env::current_dir()?,
    };

    let manifest = Manifest::read_from_folder(&project_path)?;

    let mut asset_list = BTreeSet::new();
    for input_manifest in manifest.inputs.values() {
        if let Some(id) = input_manifest.id {
            asset_list.insert(id);
        }
    }

    let mut file = BufWriter::new(fs::File::create(&options.output)?);
    for id in asset_list {
        writeln!(file, "{}", id)?;
    }
    file.flush()?;

    Ok(())
}
