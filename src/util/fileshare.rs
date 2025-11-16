use std::{path::Path, time::Duration};

use tokio::{fs::File, io::AsyncWriteExt, time::sleep};

use crate::{client::error::ClientError, util::parse_key};

pub async fn generate_upload() -> Result<String, ClientError> {
    let doup = parse_key::<String>("DOUP_URL")?;
    let token = parse_key::<String>("TOKEN")?;
    let request_url = format!("{doup}/generate_upload");

    let timeout = Duration::new(5, 0);
    let client = reqwest::ClientBuilder::new().timeout(timeout).build()?;
    let response = client
        .post(&request_url)
        .json(&serde_json::json!({ "token": token }))
        .send()
        .await?;

    if response.status().is_success() {
        return Ok(response.text().await?);
    }

    Err(ClientError::Other("unable to generate upload".to_owned()))
}

pub async fn get_upload(uuid: String, id: i64) -> Result<(), ClientError> {
    let doup = parse_key::<String>("DOUP_URL")?;
    let token = parse_key::<String>("TOKEN")?;
    let request_url = format!("{doup}/give_upload");

    sleep(Duration::from_secs(10)).await;

    let timeout = Duration::new(5, 0);
    let client = reqwest::ClientBuilder::new().timeout(timeout).build()?;
    let mut response = client
        .post(&request_url)
        .json(&serde_json::json!({ "token": token, "uuid" : uuid }))
        .send()
        .await?;

    let mut i = 0;
    while response.status().as_u16() == 202 && i < 2 {
        sleep(Duration::from_secs(10)).await;
        response = client
            .post(&request_url)
            .json(&serde_json::json!({ "token": token, "uuid" : uuid }))
            .send()
            .await?;
        i += 1;
    }

    if response.status().as_u16() == 202 {
        sleep(Duration::from_secs(30)).await;
        response = client
            .post(&request_url)
            .json(&serde_json::json!({ "token": token, "uuid" : uuid }))
            .send()
            .await?;
    }

    while response.status().as_u16() == 202 {
        sleep(Duration::from_secs(60)).await;
        response = client
            .post(&request_url)
            .json(&serde_json::json!({ "token": token, "uuid" : uuid }))
            .send()
            .await?;
    }

    if response.status().is_success() {
        let bytes = response.bytes().await?;
        let file_path = Path::new("worlds").join(id.to_string()).join(uuid);
        let mut file = File::create(&file_path).await?;
        file.write_all(&bytes).await?;
        return Ok(());
    }

    Err(ClientError::Other("unable to get upload".to_owned()))
}
