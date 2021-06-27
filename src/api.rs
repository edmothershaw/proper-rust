use reqwest::Error;

use serde::{Deserialize, Serialize};
use warp::reject::Reject;
use warp::Rejection;

#[derive(Deserialize, Serialize)]
pub struct Chuck {
    value: String
}

pub async fn make_call() -> Result<Chuck, Error> {
    let res = reqwest::get("https://api.chucknorris.io/jokes/random").await?;
    println!("Status: {}", res.status());
    println!("Headers:\n{:#?}", res.headers());

    let body: Chuck = res.json().await?;
    println!("Body:\n{}", body.value);
    Ok(body)
}
