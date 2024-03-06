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

use clap::Args;
use fs_err as fs;

use crate::{
    auth_cookie::get_auth_cookie,
    options::Global,
    roblox_api::{get_preferred_client, RobloxCredentials},
};

#[derive(Debug, Args)]
pub struct DownloadImageOptions {
    /// The path to the image to upload.
    pub asset_id: u64,

    /// The resulting path for the image asset
    #[clap(long, short)]
    pub output: String,
}

pub async fn download_image(global: Global, options: DownloadImageOptions) -> anyhow::Result<()> {
    let client = get_preferred_client(RobloxCredentials {
        token: global.auth.or_else(get_auth_cookie),
        api_key: None,
        user_id: None,
        group_id: None,
    })?;

    let response = client.download_image(options.asset_id).await?;
    fs::write(options.output, response)?;

    Ok(())
}
