use async_trait::async_trait;
use reqwest::Error;
use reqwest::header::HeaderMap;
use serde::{Deserialize, Serialize};
use warp::http::HeaderValue;

#[derive(Deserialize, Serialize)]
pub struct Chuck {
    pub value: String,
}

pub struct ChuckConfig {
    pub url: String,
}

#[async_trait]
pub trait ChuckApiService {
    async fn make_call(&self) -> Result<Chuck, Error>;
}

pub struct ChuckApiServiceImpl {
    config: ChuckConfig,
}

impl ChuckApiServiceImpl {
    pub fn new(config: ChuckConfig) -> Self {
        ChuckApiServiceImpl { config }
    }
}

#[async_trait]
impl ChuckApiService for ChuckApiServiceImpl {
    async fn make_call(&self) -> Result<Chuck, Error> {
        let client = reqwest::ClientBuilder::new();
        let mut map = HeaderMap::new();
        map.insert("test", HeaderValue::from_static("test"));
        let client2 = client.default_headers(map);
        let client3 = client2.build().unwrap();
        let res = client3.get(self.config.url.as_str()).send().await?;
        println!("Status: {}", res.status());
        let body: Chuck = res.json().await?;
        Ok(body)
    }
}

#[cfg(test)]
mod tests {
    use mockito::mock;

    use super::*;

    macro_rules! aw {
        ($e:expr) => {
            tokio_test::block_on($e)
        };
    }

    #[test]
    fn test_make_call() {
        let _m = mock("GET", "/jokes/random")
            .match_header("test", "test")
            .with_status(200)
            .with_header("content-type", "text/plain")
            .with_header("x-api-key", "1234")
            .with_body("{\"value\":\"blah\"}")
            .create();

        let url: &str = &[mockito::SERVER_URL, "/jokes/random"].join("");
        let config = ChuckConfig { url: url.to_string() };
        let service = ChuckApiServiceImpl { config };

        let res = aw!(service.make_call());
        match res {
            Ok(r) => assert_eq!(r.value, "blah"),
            Err(e) => assert_eq!(e.to_string(), ""),
        }
    }
}