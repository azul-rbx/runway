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

use anyhow::{bail, Result};
use async_trait::async_trait;
use std::marker::PhantomData;
use std::time::Duration;

use rbxcloud::rbx::{
    assets::{
        AssetCreation, AssetCreationContext, AssetCreator, AssetGroupCreator, AssetType,
        AssetUserCreator,
    },
    error::Error as RbxCloudError,
    CreateAssetWithContents, GetAsset, RbxAssets, RbxCloud,
};
use reqwest::StatusCode;
use secrecy::ExposeSecret;

use super::{
    legacy::LegacyClient, ImageUploadData, RobloxApiClient, RobloxApiError, RobloxCredentials,
    UploadResponse,
};

pub struct OpenCloudClient<'a> {
    credentials: RobloxCredentials,
    creator: AssetCreator,
    assets: RbxAssets,
    _marker: PhantomData<&'a ()>,
}

#[async_trait]
impl<'a> RobloxApiClient<'a> for OpenCloudClient<'a> {
    fn new(credentials: RobloxCredentials) -> Result<Self> {
        let creator = match (credentials.group_id, credentials.user_id) {
            (Some(id), None) => AssetCreator::Group(AssetGroupCreator {
                group_id: id.to_string(),
            }),
            (None, Some(id)) => AssetCreator::User(AssetUserCreator {
                user_id: id.to_string(),
            }),
            _ => unreachable!(),
        };

        let Some(api_key) = credentials.api_key.as_ref() else {
            bail!(RobloxApiError::MissingAuth);
        };

        let assets = RbxCloud::new(api_key.expose_secret()).assets();

        Ok(Self {
            creator,
            assets,
            credentials,
            _marker: PhantomData,
        })
    }

    async fn upload_image(&self, data: ImageUploadData<'a>) -> Result<UploadResponse> {
        self.upload_image_inner(data).await
    }

    async fn download_image(&self, id: u64) -> Result<Vec<u8>> {
        LegacyClient::new(self.credentials.clone())?
            .download_image(id)
            .await
    }
}

impl<'a> OpenCloudClient<'a> {
    async fn upload_image_inner(&self, data: ImageUploadData<'a>) -> Result<UploadResponse> {
        let asset_info = CreateAssetWithContents {
            asset: AssetCreation {
                asset_type: AssetType::DecalPng,
                display_name: data.name.to_string(),
                description: data.description.to_string(),
                creation_context: AssetCreationContext {
                    creator: self.creator.clone(),
                    expected_price: None,
                },
            },
            contents: &data.image_data,
        };

        let response = self.assets.create_with_contents(&asset_info).await?;

        let Some(operation_id) = response.path else {
            bail!(RobloxApiError::MissingOperationPath);
        };

        let Some(operation_id) = operation_id.strip_prefix("operations/") else {
            bail!(RobloxApiError::MissingOperationPath);
        };

        let operation_id = operation_id.to_string();

        const MAX_RETRIES: u32 = 5;
        const INITIAL_SLEEP_DURATION: Duration = Duration::from_millis(50);
        const BACKOFF: u32 = 2;

        let mut retry_count = 0;
        let operation = GetAsset { operation_id };
        let asset_id = async {
            loop {
                let res = self.assets.get(&operation).await?;
                let Some(response) = res.response else {
                    if retry_count > MAX_RETRIES {
                        bail!(RobloxApiError::AssetGetFailed);
                    }

                    retry_count += 1;
                    std::thread::sleep(INITIAL_SLEEP_DURATION * retry_count.pow(BACKOFF));
                    continue;
                };

                let Ok(asset_id) = response.asset_id.parse::<u64>() else {
                    bail!(RobloxApiError::AssetGetFailed);
                };

                return Ok(asset_id);
            }
        }
        .await?;

        Ok(UploadResponse {
            asset_id,
            backing_asset_id: asset_id,
        })
    }
}

impl From<RbxCloudError> for RobloxApiError {
    fn from(value: RbxCloudError) -> Self {
        match value {
            RbxCloudError::HttpStatusError { code, msg } => RobloxApiError::ResponseError {
                status: StatusCode::from_u16(code).unwrap_or_default(),
                body: msg,
            },
            _ => RobloxApiError::RbxCloud(value),
        }
    }
}
