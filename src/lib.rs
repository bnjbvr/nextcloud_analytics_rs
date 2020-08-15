use core::fmt;
use std::error::Error;

use chrono::{DateTime, Utc};
use reqwest as http;

static URL_PREFIX: &'static str = "apps/analytics/api/1.0/adddata/{COLLECTION_ID}";

pub struct Client {
    client: http::Client,
    url: String,
    user: String,
    passwd: String,
}

impl Client {
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

        let client = http::Client::builder()
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

    pub async fn send_data<S: Into<String>, F: Into<f64>>(
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

        let resp = req.body(data).send().await?;

        if resp.status() != http::StatusCode::OK {
            let status = resp.status();
            let message = resp.text().await?;
            return Err(Box::new(ApiError(format!(
                "unexpected status code: {:?}\n{}",
                status, message
            ))));
        }

        let json_resp = json::parse(&resp.text().await?)?;
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

    pub async fn send_timeline_data<S: Into<String>, F: Into<f64>>(
        &self,
        key: S,
        time: DateTime<Utc>,
        value: F,
    ) -> Result<(), Box<dyn Error>> {
        self.send_data(key.into(), time.to_rfc2822(), value.into())
            .await
    }

    pub async fn send_timeline_now_data<S: Into<String>, F: Into<f64>>(
        &self,
        key: S,
        value: F,
    ) -> Result<(), Box<dyn Error>> {
        self.send_timeline_data(key, Utc::now(), value).await
    }
}

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
