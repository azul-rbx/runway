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

use std::collections::BTreeMap;
use std::env;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

use anyhow::{bail, Result};
use clap::Args;
use fs_err as fs;
use resolve_path::PathResolveExt;

use crate::asset_name::AssetName;
use crate::auth_cookie::get_auth_cookie;
use crate::data::Manifest;
use crate::options::Global;
use crate::roblox_api::{get_preferred_client, RobloxCredentials};

#[derive(Debug, Args)]
pub struct CreateCacheMapOptions {
    pub project_path: Option<PathBuf>,

    /// A path to a directory to put any downloaded packed images.
    #[clap(long = "cache-dir")]
    pub cache_dir: PathBuf,

    /// A path to a file to contain the cache mapping.
    #[clap(long = "index-file")]
    pub index_file: PathBuf,
}

pub async fn create_cache_map(global: Global, options: CreateCacheMapOptions) -> Result<()> {
    let api_client = get_preferred_client(RobloxCredentials {
        token: global.auth.or_else(get_auth_cookie),
        api_key: None,
        user_id: None,
        group_id: None,
    })?;

    let project_path = match options.project_path {
        Some(path) => path,
        None => env::current_dir()?,
    };

    let manifest = Manifest::read_from_folder(&project_path)?;

    let index_file = options.index_file.try_resolve()?;

    let Some(index_dir) = index_file.parent() else {
        bail!("missing parent directory for index file - this should never happen");
    };

    fs::create_dir_all(index_dir)?;

    fs::create_dir_all(&options.cache_dir)?;

    let mut uploaded_inputs: BTreeMap<u64, Vec<&AssetName>> = BTreeMap::new();
    for (name, input_manifest) in &manifest.inputs {
        if let Some(id) = input_manifest.id {
            let paths = uploaded_inputs.entry(id).or_default();
            paths.push(name);
        }
    }

    let mut index: BTreeMap<u64, String> = BTreeMap::new();
    for (id, contributing_assets) in uploaded_inputs {
        if contributing_assets.len() == 1 {
            index.insert(id, contributing_assets[0].to_string());
        } else {
            let contents = api_client.download_image(id).await?;
            let path = options.cache_dir.join(id.to_string());
            fs::write(&path, contents)?;

            index.insert(id, path.display().to_string());
        }
    }

    let mut file = BufWriter::new(fs::File::create(&options.index_file)?);
    serde_json::to_writer_pretty(&mut file, &index)?;
    file.flush()?;

    Ok(())
}
