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

use std::{
    fmt::{self, Write},
    marker::PhantomData,
};

use anyhow::Result;
use async_trait::async_trait;
use reqwest::{
    header::{HeaderValue, COOKIE},
    Client, Request, Response, StatusCode,
};
use secrecy::ExposeSecret;
use serde::Deserialize;
use tokio::sync::RwLock;


use super::{resolve_web_asset_id, ImageUploadData, RobloxApiClient, RobloxApiError, RobloxCredentials, UploadResponse};

/// Internal representation of what the asset upload endpoint returns, before
/// we've handled any errors.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct RawUploadResponse {
    success: bool,
    message: Option<String>,
    asset_id: Option<u64>,
    backing_asset_id: Option<u64>,
}

pub struct LegacyClient<'a> {
    credentials: RobloxCredentials,
    csrf_token: RwLock<Option<HeaderValue>>,
    client: Client,
    _marker: PhantomData<&'a ()>,
}

impl<'a> fmt::Debug for LegacyClient<'a> {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "RobloxApiClient")
    }
}

#[async_trait]
impl<'a> RobloxApiClient<'a> for LegacyClient<'a> {
    fn new(credentials: RobloxCredentials) -> Result<Self> {
        Ok(Self {
            credentials,
            csrf_token: RwLock::new(None),
            client: Client::new(),
            _marker: PhantomData,
        })
    }

    async fn download_image(&self, id: u64) -> Result<Vec<u8>> {
        let id = resolve_web_asset_id(id)?;
        let url = format!("https://assetdelivery.roblox.com/v1/asset/?id={}", id);

        let mut response = self
            .execute_with_csrf_retry(|client| Ok(client.get(&url).build()?))
            .await?;

        let mut buffer = Vec::new();
        response.copy_to(&mut buffer)?;

        Ok(buffer)
    }

    /// Upload an image, returning an error if anything goes wrong.
    async fn upload_image(&self, data: ImageUploadData<'a>) -> Result<UploadResponse> {
        let response = self.upload_image_raw(data).await?;

        // Some other errors will be reported inside the response, even
        // though we received a successful HTTP response.
        if response.success {
            let asset_id = response.asset_id.unwrap();
            let backing_asset_id = asset_id;
            let asset_id = resolve_web_asset_id(backing_asset_id)?;

            Ok(UploadResponse {
                asset_id,
                backing_asset_id,
            })
        } else {
            let message = response.message.unwrap();

            Err(RobloxApiError::ApiError { message }.into())
        }
    }
}

impl<'a> LegacyClient<'a> {
    /// Upload an image, returning the raw response returned by the endpoint,
    /// which may have further failures to handle.
    async fn upload_image_raw(&self, data: ImageUploadData<'a>) -> Result<RawUploadResponse> {
        let mut url = "https://data.roblox.com/data/upload/json?assetTypeId=13".to_owned();

        if let Some(id) = &self.credentials.group_id {
            write!(url, "&groupId={}", id).unwrap();
        }

        let mut response = self
            .execute_with_csrf_retry(|client| {
                Ok(client
                    .post(&url)
                    .query(&[
                        ("name", data.name.clone()),
                        ("description", data.description.clone()),
                    ])
                    .body(data.image_data.clone().into_owned())
                    .build()?)
            })
            .await?;

        let body = response.text()?;

        // Some errors will be reported through HTTP status codes, handled here.
        if response.status().is_success() {
            match serde_json::from_str(&body) {
                Ok(response) => Ok(response),
                Err(source) => Err(RobloxApiError::BadResponseJson { body, source }.into()),
            }
        } else {
            Err(RobloxApiError::ResponseError {
                status: response.status(),
                body,
            }
            .into())
        }
    }

    /// Execute a request generated by the given function, retrying if the
    /// endpoint requests that the user refreshes their CSRF token.
    async fn execute_with_csrf_retry<F>(&self, make_request: F) -> Result<Response>
    where
        F: Fn(&Client) -> Result<Request>,
    {
        let mut request = make_request(&self.client)?;
        self.attach_headers(&mut request).await;

        let response = self.client.execute(request)?;

        match response.status() {
            StatusCode::FORBIDDEN => {
                if let Some(csrf) = response.headers().get("X-CSRF-Token") {
                    log::debug!("Retrying request with X-CSRF-Token...");

                    let mut csrf_token = self.csrf_token.write().await;
                    *csrf_token = Some(csrf.clone());

                    let mut new_request = make_request(&self.client)?;
                    self.attach_headers(&mut new_request).await;

                    Ok(self.client.execute(new_request)?)
                } else {
                    // If the response did not return a CSRF token for us to
                    // retry with, this request was likely forbidden for other
                    // reasons.

                    Ok(response)
                }
            }
            _ => Ok(response),
        }
    }

    /// Attach required headers to a request object before sending it to a
    /// Roblox API, like authentication and CSRF protection.
    async fn attach_headers(&self, request: &mut Request) {
        if let Some(auth_token) = &self.credentials.token {
            let cookie_value = format!(".ROBLOSECURITY={}", auth_token.expose_secret());

            request.headers_mut().insert(
                COOKIE,
                HeaderValue::from_bytes(cookie_value.as_bytes()).unwrap(),
            );
        }

        let csrf_token = self.csrf_token.read().await;

        if let Some(csrf) = csrf_token.clone() {
            request.headers_mut().insert("X-CSRF-Token", csrf);
        }
    }
}
