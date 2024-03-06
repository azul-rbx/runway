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

mod asset_list;
mod create_cache_map;
mod download_image;
mod sync;
mod upload_image;

pub use asset_list::*;
use clap::Subcommand;
pub use create_cache_map::*;
pub use download_image::*;
pub use sync::*;
pub use upload_image::*;

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Upload a single image to the Roblox cloud. Prints the asset ID of the
    /// resulting Image asset to stdout.
    UploadImage(UploadImageOptions),

    /// Sync your Tarmac project, uploading any assets that have changed.
    Sync(SyncOptions),

    /// Downloads any packed spritesheets, then generates a file mapping asset
    /// IDs to file paths. This command only works when logged into Roblox
    /// Studio or when a .ROBLOSECURITY token is passed via --auth.
    CreateCacheMap(CreateCacheMapOptions),

    /// Creates a file that lists all assets required by the project.
    AssetList(AssetListOptions),

    /// Downloads a single image from the Roblox cloud.
    DownloadImage(DownloadImageOptions),
}
