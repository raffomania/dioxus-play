#![allow(non_snake_case)]
use anyhow::{anyhow, Context, Result};
use log::{debug, LevelFilter};
use rand::{distributions::Alphanumeric, Rng};
use std::env;

use dioxus::prelude::*;

use crate::shortcuts::use_shortcuts;

mod shortcuts;

fn params() -> Result<String> {
    let username = env::var("SUBSONIC_USER").context("Failed to read SUBSONIC_USER")?;
    let password = env::var("SUBSONIC_PASSWORD").context("Failed to read SUBSONIC_PASSOWRD")?;
    let salt: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(36)
        .map(char::from)
        .collect();
    let pre_t = password + &salt;
    let token = format!("{:x}", md5::compute(pre_t.as_bytes()));

    let auth = format!("u={username}&t={token}&s={salt}");
    let format = "json";
    let crate_name = env!("CARGO_PKG_NAME");
    let version = "1.16.1";
    Ok(format!("?{auth}&v={version}&c={crate_name}&f={format}",))
}

async fn random_song_id() -> Result<String> {
    let url = format!(
        "{server_url}/rest/getRandomSongs{params}&size=1",
        params = params()?,
        server_url = env::var("SUBSONIC_SERVER_URL")?
    );
    let val: serde_json::Value = reqwest::get(url)
        .await
        .context("Failed to fetch")?
        .json()
        .await
        .context("Failed to deserialize")?;

    let id = val["subsonic-response"]["randomSongs"]["song"][0]["id"]
        .as_str()
        .ok_or(anyhow!("Did not find song ID in response"))?
        .to_string();
    Ok(id)
}

fn download_song_url(id: &str) -> Result<String> {
    Ok(format!(
        "{server_url}/rest/stream{params}&id={id}",
        params = params()?,
        server_url =
            env::var("SUBSONIC_SERVER_URL").context("Failed to read SUBSONIC_SERVER_URL")?
    ))
}

fn main() {
    dioxus_logger::init(LevelFilter::Debug).expect("failed to init logger");
    dioxus_desktop::launch(App);
}

fn App(cx: Scope) -> Element {
    debug!("render");
    use_shortcuts(cx);
    let song_id_fut = use_future(cx, (), |_| async { random_song_id().await });

    let Some(song_id) = song_id_fut.value() else {
        return render! {"loading..."}
    };

    let song_id = match song_id {
        Ok(id) => id,
        Err(err) => return render! { pre { "{err:?}" } },
    };

    render! { Player { song_id: song_id } }
}

#[inline_props]
fn Player<'a>(cx: Scope, song_id: &'a str) -> Element {
    let nodes = download_song_url(song_id).ok().map(|url| {
        rsx!(audio {
            controls: true,
            onplay: |_| println!("play"),
            width: "40em",
            display: "block",
            src: "{url}"
        })
    })?;
    cx.render(nodes)
}
