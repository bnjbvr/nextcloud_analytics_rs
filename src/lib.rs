//! # nextcloud_analytics_rs
//!
//! A small Rust wrapper to call the [Nextcloud Analytics
//! API](https://github.com/rello/analytics/wiki/API), for databases of type "internal database".
//!
//! Example of usage:
//!
//! ```
//!   let base_url = "https://example.com/nextcloud";
//!   let collection = 42;
//!   let user = "myself";
//!   let passwd = "hunter2";
//!
//!   let client = nextcloud_analytics_rs::SyncClient::new(base_url, collection, user, passwd);
//!   client.send_timeline_now_data("speed_kmh", 180).unwrap_or_else(|_| println!("api or network error"));
//!   client.send_timeline_now_data("power_level", 9001).unwrap_or_else(|_| println!("api or network error"));
//!
//!   let other_collection = 3;
//!   let client = nextcloud_analytics_rs::SyncClient::new(base_url, other_collection, user, passwd);
//!   client.send_data("age", "alice", 25).unwrap_or_else(|_| println!("api or network error"));
//!   client.send_data("age", "bob", 20).unwrap_or_else(|_| println!("api or network error"));
//! ```

use core::fmt;
use std::error::Error;

use chrono::{DateTime, Utc};
use reqwest as http;

static URL_PREFIX: &'static str = "apps/analytics/api/1.0/adddata/{COLLECTION_ID}";

/// A synchronous client to call the Nextcloud Analytics API.
pub struct SyncClient {
    client: http::blocking::Client,
    url: String,
    user: String,
    passwd: String,
}

impl SyncClient {
    /// Create a new synchronous client to call the Nextcloud Analytics API.
    ///
    /// - `nextcloud_url` is the base URL of the Nextcloud instance.
    /// - `collection` is the collection index, as presented by Nextcloud Analytics' interface
    /// (number in the URL).
    /// - `user` is the Nextcloud user's name.
    /// - `passwd` is an app password associaetd to the Nextcloud user's account.
    pub fn new<S: Into<String>>(nextcloud_url: &str, collection: u32, user: S, passwd: S) -> Self {
        let mut url = nextcloud_url.to_string();

        // Add trailing slash if necessary.
        if !url.ends_with("/") {
            url += "/";
        }

        url += &URL_PREFIX.replace("{COLLECTION_ID}", &collection.to_string());

        let mut headers = http::header::HeaderMap::new();

        headers.insert(
            http::header::CONTENT_TYPE,
            http::header::HeaderValue::from_static("application/json"),
        );

        let client = http::blocking::Client::builder()
            .default_headers(headers)
            .build()
            .unwrap();

        Self {
            client,
            url,
            user: user.into(),
            passwd: passwd.into(),
        }
    }

    /// Sends some data to the API, the two first dimensions must be formatted as text while the
    /// last dimension must be a numerical value.
    ///
    /// For timeline data, `dimension2` must be the date in the RFC2822 format.
    pub fn send_data<S: Into<String>, F: Into<f64>>(
        &self,
        dimension1: S,
        dimension2: S,
        dimension3: F,
    ) -> Result<(), Box<dyn Error>> {
        let data = format!(
            r#"{{
    "dimension1": {:?},
    "dimension2": {:?},
    "dimension3": "{}"
}}"#,
            dimension1.into(),
            dimension2.into(),
            dimension3.into()
        );

        let req = self
            .client
            .post(&self.url)
            .basic_auth(self.user.clone(), Some(self.passwd.clone()));

        let resp = req.body(data).send()?;

        if resp.status() != http::StatusCode::OK {
            let status = resp.status();
            let message = resp.text()?;
            return Err(Box::new(ApiError(format!(
                "unexpected status code: {:?}\n{}",
                status, message
            ))));
        }

        let json_resp = json::parse(&resp.text()?)?;
        if !json_resp["success"]
            .as_bool()
            .expect("There should be a success field in the API response")
        {
            return Err(Box::new(ApiError(format!(
                "unexpected API response: {}",
                json_resp["error"]["message"]
                    .as_str()
                    .expect("There should be an error.message in the API response")
            ))));
        }

        Ok(())
    }

    /// Sends some timeline data to the API: the `key` is the index of this piece of data,
    /// associated to the given `value` at the given `time`. for the given `time`.
    pub fn send_timeline_data<S: Into<String>, F: Into<f64>>(
        &self,
        key: S,
        time: DateTime<Utc>,
        value: F,
    ) -> Result<(), Box<dyn Error>> {
        self.send_data(key.into(), time.to_rfc2822(), value.into())
    }

    /// Sends some timeline data to the API: the `key` is the index of this piece of data,
    /// associated to the given `value` at the current UTC time.
    pub fn send_timeline_now_data<S: Into<String>, F: Into<f64>>(
        &self,
        key: S,
        value: F,
    ) -> Result<(), Box<dyn Error>> {
        self.send_timeline_data(key, Utc::now(), value)
    }
}

/// A simple error wrapper for API errors.
pub struct ApiError(String);

impl fmt::Debug for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Error for ApiError {}
