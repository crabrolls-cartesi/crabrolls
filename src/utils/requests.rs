use reqwest::{Client, Response};
use serde::{de::DeserializeOwned, Serialize};
use std::error::Error;

pub struct ClientWrapper {
    client: Client,
    base_url: String,
}

impl ClientWrapper {
    pub fn new(base_url: String) -> Self {
        Self {
            client: Client::new(),
            base_url,
        }
    }

    pub async fn post<T: Serialize>(
        &self,
        route: &str,
        request: &T,
    ) -> Result<Response, reqwest::Error> {
        let endpoint = format!("{}/{}", self.base_url, route);
        self.client.post(&endpoint).json(request).send().await
    }

    pub async fn parse_response<T: DeserializeOwned>(
        &self,
        response: Response,
    ) -> Result<T, Box<dyn Error>> {
        let body = response.json::<T>().await?;
        Ok(body)
    }
}
