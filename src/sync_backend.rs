/*
 * Copyright (c) Paradoxum Games 2024
 * This file is licensed under the Mozilla Public License (MPL-2.0). A copy of it is available in the 'LICENSE' file at the root of the repository.
 * This file incorporates changes from rojo-rbx/tarmac, which is licensed under the MIT license.
 * 
 * Copyright 2019 Lucien Greathouse
 * Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:
 * The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
*/


use std::{borrow::Cow, io, marker::PhantomData, path::PathBuf, sync::Arc, thread, time::Duration};

use anyhow::Result;
use async_trait::async_trait;
use fs_err as fs;
use reqwest::StatusCode;
use roblox_install::RobloxStudio;
use thiserror::Error as ThisError;

use crate::{
    data::AssetId,
    roblox_api::{ImageUploadData, RobloxApiClient, RobloxApiError},
};

#[async_trait]
pub trait SyncBackend {
    async fn upload(&self, data: UploadInfo) -> Result<UploadResponse>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UploadResponse {
    pub id: AssetId,
}

#[derive(Clone, Debug)]
pub struct UploadInfo {
    pub name: String,
    pub contents: Vec<u8>,
    pub hash: String,
}

pub struct RobloxSyncBackend<'a, ApiClient>
where
    ApiClient: RobloxApiClient<'a> + Sync + Clone + Send,
{
    api_client: Arc<ApiClient>,
    _marker: PhantomData<&'a ()>,
}

impl<'a, ApiClient> RobloxSyncBackend<'a, ApiClient>
where
    ApiClient: RobloxApiClient<'a> + Sync + Clone + Send,
{
    pub fn new(api_client: ApiClient) -> Self {
        Self {
            api_client: Arc::new(api_client),
            _marker: PhantomData::default(),
        }
    }
}

#[async_trait]
impl<'a, ApiClient> SyncBackend for RobloxSyncBackend<'a, ApiClient>
where
    ApiClient: RobloxApiClient<'a> + Sync + Clone + Send,
{
    async fn upload(&self, data: UploadInfo) -> Result<UploadResponse> {
        log::info!("Uploading {} to Roblox", &data.name);

        let result = self
            .api_client
            .upload_image(ImageUploadData {
                image_data: Cow::Owned(data.contents),
                name: data.name.clone(),
                description: "Uploaded by Tarmac.".to_string(),
            })
            .await;

        match result {
            Ok(response) => {
                log::info!("Uploaded {} to ID {}", data.name, response.backing_asset_id);

                Ok(UploadResponse {
                    id: AssetId::Id(response.backing_asset_id),
                })
            }

            // Err(RobloxApiError::ResponseError {
            //     status: StatusCode::TOO_MANY_REQUESTS,
            //     ..
            // }) => Err(Error::RateLimited),
            Err(err) => {
                if err.is::<RobloxApiError>() {
                    let err = err.downcast::<RobloxApiError>()?;
                    if let RobloxApiError::ResponseError {
                        status: StatusCode::TOO_MANY_REQUESTS,
                        ..
                    } = err
                    {
                        Err(Error::RateLimited.into())
                    } else {
                        Err(err.into())
                    }
                } else {
                    Err(err.into())
                }
            }
        }
    }
}

pub struct LocalSyncBackend {
    content_path: PathBuf,
    scope: Option<String>,
}

impl LocalSyncBackend {
    pub fn new(scope: Option<String>) -> Result<LocalSyncBackend> {
        RobloxStudio::locate()
            .map(|studio| LocalSyncBackend {
                content_path: studio.content_path().into(),
                scope,
            })
            .map_err(|error| error.into())
    }

    fn get_asset_path(&self, data: &UploadInfo) -> PathBuf {
        let mut path = PathBuf::from(".tarmac");
        if let Some(scope) = &self.scope {
            path.push(scope);
        }
        path.push(self.get_asset_file_name(data));
        path
    }

    fn get_asset_file_name(&self, data: &UploadInfo) -> String {
        format!("{}.png", data.name)
    }
}

#[async_trait]
impl SyncBackend for LocalSyncBackend {
    async fn upload(&self, data: UploadInfo) -> Result<UploadResponse> {
        let asset_path = self.get_asset_path(&data);
        let file_path = self.content_path.join(&asset_path);
        let parent = file_path
            .parent()
            .expect("content folder should have a parent");

        fs::create_dir_all(parent)?;
        fs::write(&file_path, &data.contents)?;

        log::info!("Written {} to path {}", &data.name, file_path.display());

        Ok(UploadResponse {
            id: AssetId::Path(asset_path),
        })
    }
}

pub struct NoneSyncBackend;

#[async_trait]
impl SyncBackend for NoneSyncBackend {
    async fn upload(&self, _data: UploadInfo) -> Result<UploadResponse> {
        Err(Error::NoneBackend.into())
    }
}

pub struct DebugSyncBackend {
    last_id: u64,
}

impl DebugSyncBackend {
    pub fn new() -> Self {
        Self { last_id: 0 }
    }
}

#[async_trait]
impl SyncBackend for DebugSyncBackend {
    async fn upload(&self, data: UploadInfo) -> Result<UploadResponse> {
        todo!();
        // log::info!("Copying {} to local folder", &data.name);

        // self.last_id += 1;
        // let id = self.last_id;

        // let path = Path::new(".tarmac-debug");
        // fs::create_dir_all(path)?;

        // let file_path = path.join(id.to_string());
        // fs::write(&file_path, &data.contents)?;

        // Ok(UploadResponse {
        //     id: AssetId::Id(id),
        // })
    }
}

/// Performs the retry logic for rate limitation errors. The struct wraps a SyncBackend so that
/// when a RateLimited error occurs, the thread sleeps for a moment and then tries to reupload the
/// data.
///
#[derive(Clone, Debug)]
pub struct RetryBackend<InnerSyncBackend> {
    inner: InnerSyncBackend,
    delay: Duration,
    attempts: usize,
}

impl<InnerSyncBackend> RetryBackend<InnerSyncBackend> {
    /// Creates a new backend from another SyncBackend. The max_retries parameter gives the number
    /// of times the backend will try again (so given 0, it acts just as the original SyncBackend).
    /// The delay parameter provides the amount of time to wait between each upload attempt.
    pub fn new(inner: InnerSyncBackend, max_retries: usize, delay: Duration) -> Self {
        Self {
            inner,
            delay,
            attempts: max_retries + 1,
        }
    }
}

#[async_trait]
impl<InnerSyncBackend: SyncBackend + Clone + Sync> SyncBackend for RetryBackend<InnerSyncBackend> {
    async fn upload(&self, data: UploadInfo) -> Result<UploadResponse> {
        for index in 0..self.attempts {
            if index != 0 {
                log::info!(
                    "tarmac is being rate limited, retrying upload ({}/{})",
                    index,
                    self.attempts - 1
                );
                thread::sleep(self.delay);
            }
            let result = self.inner.upload(data.clone()).await;

            if let Ok(response) = result {
                return Ok(response);
            }
        }

        Err(Error::RateLimited.into())
    }
}

#[derive(Debug, ThisError)]
pub enum Error {
    #[error("Cannot upload assets with the 'none' target.")]
    NoneBackend,

    #[error("Tarmac was rate-limited trying to upload assets. Try again in a little bit.")]
    RateLimited,

    #[error(transparent)]
    StudioInstall {
        #[from]
        source: roblox_install::Error,
    },

    #[error(transparent)]
    Io {
        #[from]
        source: io::Error,
    },

    #[error(transparent)]
    RobloxError {
        #[from]
        source: RobloxApiError,
    },
}

// #[cfg(test)]
// mod test {
//     use super::*;

//     #[allow(unused_must_use)]
//     mod test_retry_backend {
//         use super::*;

//         struct CountUploads<'a> {
//             counter: &'a mut usize,
//             results: Vec<Result<UploadResponse, Error>>,
//         }

//         impl<'a> CountUploads<'a> {
//             fn new(counter: &'a mut usize) -> Self {
//                 Self {
//                     counter,
//                     results: Vec::new(),
//                 }
//             }

//             fn with_results(mut self, results: Vec<Result<UploadResponse, Error>>) -> Self {
//                 self.results = results;
//                 self.results.reverse();
//                 self
//             }
//         }

//         impl<'a> SyncBackend for CountUploads<'a> {
//             fn upload(&mut self, _data: UploadInfo) -> Result<UploadResponse, Error> {
//                 (*self.counter) += 1;
//                 self.results.pop().unwrap_or(Err(Error::NoneBackend))
//             }
//         }

//         fn any_upload_info() -> UploadInfo {
//             UploadInfo {
//                 name: "foo".to_owned(),
//                 contents: Vec::new(),
//                 hash: "hash".to_owned(),
//             }
//         }

//         fn retry_duration() -> Duration {
//             Duration::from_millis(1)
//         }

//         #[test]
//         fn upload_at_least_once() {
//             let mut counter = 0;
//             let mut backend =
//                 RetryBackend::new(CountUploads::new(&mut counter), 0, retry_duration());

//             backend.upload(any_upload_info());

//             assert_eq!(counter, 1);
//         }

//         #[test]
//         fn upload_again_if_rate_limited() {
//             let mut counter = 0;
//             let inner = CountUploads::new(&mut counter).with_results(vec![
//                 Err(Error::RateLimited),
//                 Err(Error::RateLimited),
//                 Err(Error::NoneBackend),
//             ]);
//             let mut backend = RetryBackend::new(inner, 5, retry_duration());

//             backend.upload(any_upload_info());

//             assert_eq!(counter, 3);
//         }

//         #[test]
//         fn upload_returns_first_success_result() {
//             let mut counter = 0;
//             let success = UploadResponse {
//                 id: AssetId::Id(10),
//             };
//             let inner = CountUploads::new(&mut counter).with_results(vec![
//                 Err(Error::RateLimited),
//                 Err(Error::RateLimited),
//                 Ok(success.clone()),
//             ]);
//             let mut backend = RetryBackend::new(inner, 5, retry_duration());

//             let upload_result = backend.upload(any_upload_info()).unwrap();

//             assert_eq!(counter, 3);
//             assert_eq!(upload_result, success);
//         }

//         #[test]
//         fn upload_returns_rate_limited_when_retries_exhausted() {
//             let mut counter = 0;
//             let inner = CountUploads::new(&mut counter).with_results(vec![
//                 Err(Error::RateLimited),
//                 Err(Error::RateLimited),
//                 Err(Error::RateLimited),
//                 Err(Error::RateLimited),
//             ]);
//             let mut backend = RetryBackend::new(inner, 2, retry_duration());

//             let upload_result = backend.upload(any_upload_info()).unwrap_err();

//             assert_eq!(counter, 3);
//             assert!(match upload_result {
//                 Error::RateLimited => true,
//                 _ => false,
//             });
//         }
//     }
// }
