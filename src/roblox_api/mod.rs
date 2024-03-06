/*
 * Copyright (c) Paradoxum Games 2024
 * This file is licensed under the Mozilla Public License (MPL-2.0). A copy of it is available in the 'LICENSE' file at the root of the repository.
 * This file incorporates changes from rojo-rbx/tarmac, which is licensed under the MIT license.
 *
 * Copyright (c) 2020 Roblox Corporation
 * Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:
 * The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
*/

mod legacy;
mod open_cloud;

use std::borrow::Cow;

use anyhow::Result;
use async_trait::async_trait;
use rbxcloud::rbx::error::Error as RbxCloudError;
use reqwest::StatusCode;
use secrecy::SecretString;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use self::{legacy::LegacyClient, open_cloud::OpenCloudClient};

#[derive(Debug, Clone)]
pub struct ImageUploadData<'a> {
    pub image_data: Cow<'a, [u8]>,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct UploadResponse {
    pub asset_id: u64,
    pub backing_asset_id: u64,
}

#[derive(Clone, Debug)]
pub struct RobloxCredentials {
    pub token: Option<SecretString>,
    pub api_key: Option<SecretString>,
    pub user_id: Option<u64>,
    pub group_id: Option<u64>,
}

#[async_trait]
pub trait RobloxApiClient<'a> {
    fn new(credentials: RobloxCredentials) -> Result<Self>
    where
        Self: Sized;

    async fn upload_image(&self, data: ImageUploadData<'a>) -> Result<UploadResponse>;

    async fn download_image(&self, id: u64) -> Result<Vec<u8>>;
}

#[derive(Debug, Error)]
pub enum RobloxApiError {
    #[error("Roblox API HTTP error")]
    Http {
        #[from]
        source: reqwest::Error,
    },

    #[error("Roblox API error: {message}")]
    ApiError { message: String },

    #[error("Roblox API returned success, but had malformed JSON response: {body}")]
    BadResponseJson {
        body: String,
        source: serde_json::Error,
    },

    #[error("Roblox API returned HTTP {status} with body: {body}")]
    ResponseError { status: StatusCode, body: String },

    #[error("Request for CSRF token did not return an X-CSRF-Token header.")]
    MissingCsrfToken,

    #[error("Failed to retrieve asset ID from Roblox cloud")]
    AssetGetFailed,

    #[error("Tarmac is unable to locate an authentication method")]
    MissingAuth,

    #[error("Group ID and user ID cannot both be specified")]
    AmbiguousCreatorType,

    #[error("Operation path is missing")]
    MissingOperationPath,

    #[error("Operation path is malformed")]
    MalformedOperationPath,

    #[error("Open Cloud API error")]
    RbxCloud(RbxCloudError),

    #[error("Failed to parse asset ID from asset get response")]
    MalformedAssetId(#[from] std::num::ParseIntError),
}

pub fn get_preferred_client<'a>(
    credentials: RobloxCredentials,
) -> Result<Box<dyn RobloxApiClient<'static>>> {
    match &credentials {
        RobloxCredentials {
            token: None,
            api_key: None,
            ..
        } => Err(RobloxApiError::MissingAuth.into()),

        RobloxCredentials {
            api_key: Some(_), ..
        } => Ok(Box::new(OpenCloudClient::new(credentials)?)),

        RobloxCredentials { token: Some(_), .. } => Ok(Box::new(LegacyClient::new(credentials)?)),
    }
}
