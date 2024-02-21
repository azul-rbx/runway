use std::{fmt::{self, Write}, marker::PhantomData};

use async_trait::async_trait;
use reqwest::{
    header::{HeaderValue, COOKIE},
    Client, Request, Response, StatusCode,
};
use anyhow::Result;
use secrecy::ExposeSecret;
use serde::Deserialize;
use tokio::sync::RwLock;

use crate::auth_cookie::get_csrf_token;

use super::{ImageUploadData, RobloxApiClient, RobloxApiError, RobloxCredentials, UploadResponse};

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
    _marker: PhantomData<&'a ()>
}

impl<'a> fmt::Debug for LegacyClient<'a> {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "RobloxApiClient")
    }
}

#[async_trait]
impl<'a> RobloxApiClient<'a> for LegacyClient<'a> {
    fn new(credentials: RobloxCredentials) -> Result<Self> {
        match &credentials.token {
            Some(token) => {
                let csrf_token = match get_csrf_token(token) {
                    Ok(value) => RwLock::new(Some(value)),
                    Err(err) => {
                        log::error!("Was unable to fetch CSRF token: {}", err.to_string());
                        RwLock::new(None)
                    }
                };

                Ok(Self {
                    credentials,
                    csrf_token,
                    client: Client::new(),
                    _marker: PhantomData::default()
                })
            }
            _ => Ok(Self {
                credentials,
                csrf_token: RwLock::new(None),
                client: Client::new(),
                _marker: PhantomData::default()
            }),
        }
    }

    async fn download_image(&self, id: u64) -> Result<Vec<u8>> {
        let url = format!("https://roblox.com/asset?id={}", id);

        let mut response =
            self.execute_with_csrf_retry(|client| Ok(client.get(&url).build()?)).await?;

        let mut buffer = Vec::new();
        response.copy_to(&mut buffer)?;

        Ok(buffer)
    }

    /// Upload an image, retrying if the asset endpoint determines that the
    /// asset's name is inappropriate. The asset's name will be replaced with a
    /// generic known-good string.
    // fn upload_image_with_moderation_retry(
    //     &mut self,
    //     data: &ImageUploadData,
    // ) -> Result<UploadResponse, RobloxApiError> {
    //     let response = self.upload_image_raw(data)?;

    //     // Some other errors will be reported inside the response, even
    //     // though we received a successful HTTP response.
    //     if response.success {
    //         let asset_id = response.asset_id.unwrap();
    //         let backing_asset_id = response.backing_asset_id.unwrap();

    //         Ok(UploadResponse {
    //             asset_id,
    //             backing_asset_id,
    //         })
    //     } else {
    //         let message = response.message.unwrap();

    //         // There are no status codes for this API, so we pattern match
    //         // on the returned error message.
    //         //
    //         // If the error message text mentions something being
    //         // inappropriate, we assume the title was problematic and
    //         // attempt to re-upload.
    //         if message.contains("inappropriate") {
    //             log::warn!(
    //                 "Image name '{}' was moderated, retrying with different name...",
    //                 data.name
    //             );

    //             let new_data = ImageUploadData {
    //                 name: "image",
    //                 ..data.to_owned()
    //             };

    //             self.upload_image(&new_data)
    //         } else {
    //             Err(RobloxApiError::ApiError { message })
    //         }
    //     }
    // }

    /// Upload an image, returning an error if anything goes wrong.
    async fn upload_image(&self, data: ImageUploadData::<'a>) -> Result<UploadResponse> {
        let response = self.upload_image_raw(data).await?;

        // Some other errors will be reported inside the response, even
        // though we received a successful HTTP response.
        if response.success {
            let asset_id = response.asset_id.unwrap();
            let backing_asset_id = response.backing_asset_id.unwrap();

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
    async fn upload_image_raw(
        &self,
        data: ImageUploadData::<'a>,
    ) -> Result<RawUploadResponse> {
        let mut url = "https://data.roblox.com/data/upload/json?assetTypeId=13".to_owned();

        if let Some(id) = &self.credentials.group_id {
            write!(url, "&groupId={}", id).unwrap();
        }

        let mut response = self.execute_with_csrf_retry(|client| {
            Ok(client
                .post(&url)
                .query(&[("name", data.name.clone()), ("description", data.description.clone())])
                .body(data.image_data.clone().into_owned())
                .build()?)
        }).await?;

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
            }.into())
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