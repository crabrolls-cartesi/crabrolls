use reqwest::{Client, Response};
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::error::Error;

pub async fn send_post(
    client: &Client,
    url: &str,
    route: &str,
    request: &Value,
) -> Result<Response, reqwest::Error> {
    let endpoint = format!("{}/{}", url, route);
    client.post(&endpoint).json(request).send().await
}

pub async fn parse_response<T: DeserializeOwned>(response: Response) -> Result<T, Box<dyn Error>> {
    let body = response.json::<T>().await?;
    Ok(body)
}
