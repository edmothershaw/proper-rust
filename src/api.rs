use reqwest::Error;
use serde::{Deserialize, Serialize};
use reqwest::header::HeaderMap;
use warp::http::HeaderValue;

#[derive(Deserialize, Serialize)]
pub struct Chuck {
    pub value: String
}

pub async fn make_call() -> Result<Chuck, Error> {
    let client = reqwest::ClientBuilder::new();
    let mut map = HeaderMap::new();
    map.insert("test", HeaderValue::from_static("test"));
    let client2 = client.default_headers(map);
    let client3 = client2.build().unwrap();
    let res = client3.get("https://api.chucknorris.io/jokes/random").send().await?;
    println!("Status: {}", res.status());
    println!("Headers:\n{:#?}", res.headers());

    let body: Chuck = res.json().await?;
    println!("Body:\n{}", body.value);
    Ok(body)
}
