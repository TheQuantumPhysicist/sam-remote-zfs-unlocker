use std::collections::BTreeMap;

use async_trait::async_trait;

use super::traits::HttpRequest;

pub struct WasmRequest {}

impl WasmRequest {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait(?Send)]
impl HttpRequest for WasmRequest {
    type Error = reqwasm::Error;

    async fn get(&self, url: &str) -> Result<reqwasm::http::Response, Self::Error> {
        reqwasm::http::Request::get(url).send().await
    }

    async fn post<T: serde::Serialize>(
        &self,
        url: &str,
        body: Option<T>,
        extra_headers: BTreeMap<String, String>,
    ) -> Result<reqwasm::http::Response, Self::Error> {
        let req = reqwasm::http::Request::post(url);
        let req = match body {
            Some(b) => {
                let json_body = serde_json::to_string(&b)?;
                req.body(json_body.as_str())
                    .header("Content-Type", "application/json")
            }
            None => req,
        };
        let req = extra_headers
            .into_iter()
            .fold(req, |req, (key, val)| req.header(&key, &val));
        req.send().await
    }
}
