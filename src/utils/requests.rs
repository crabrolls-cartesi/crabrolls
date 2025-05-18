use serde::Serialize;
use std::error::Error;
use std::fmt::Debug;
use ureq;

pub struct ClientWrapper {
    base_url: String,
}

impl ClientWrapper {
    pub fn new(base_url: String) -> Self {
        Self { base_url }
    }

    pub async fn post<T: Serialize + Debug>(
        &self,
        route: &str,
        request: &T,
    ) -> Result<ureq::Response, Box<dyn Error>> {
        let url = format!("{}/{}", self.base_url, route);
        let response = ureq::post(&url).send_json(serde_json::to_value(request)?)?;
        Ok(response)
    }

    pub async fn parse_response(
        &self,
        response: ureq::Response,
    ) -> Result<serde_json::Value, Box<dyn Error>> {
        let response_json: serde_json::Value = response.into_json()?;
        Ok(response_json)
    }
}
