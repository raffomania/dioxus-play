use std::env;

use anyhow::{Context, Result};
use serde::de::DeserializeOwned;

use crate::Credentials;

pub fn params(credentials: &Credentials) -> Result<String> {
    let Credentials {
        salt,
        username,
        password,
    } = credentials;

    let pre_t = password.clone() + salt;
    let token = format!("{:x}", md5::compute(pre_t.as_bytes()));

    let auth = format!("u={username}&t={token}&s={salt}");
    let format = "json";
    let crate_name = env!("CARGO_PKG_NAME");
    let version = "1.16.1";
    Ok(format!("?{auth}&v={version}&c={crate_name}&f={format}",))
}

pub fn url(credentials: &Credentials, endpoint: &str, extra_params: &str) -> Result<String> {
    Ok(format!(
        "{server_url}/rest/{endpoint}{params}{extra_params}",
        params = params(credentials)?,
        server_url =
            env::var("SUBSONIC_SERVER_URL").context("Failed to read SUBSONIC_SERVER_URL")?,
    ))
}

pub async fn get_json<T: DeserializeOwned>(
    credentials: &Credentials,
    endpoint: &str,
    extra_params: &str,
) -> Result<T> {
    reqwest::get(url(credentials, endpoint, extra_params)?)
        .await
        .context("Failed to fetch")?
        .json()
        .await
        .context("Failed to deserialize")
}
