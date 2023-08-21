#![allow(non_snake_case)]
use anyhow::{anyhow, Context, Result};
use dioxus::prelude::*;
use log::{debug, LevelFilter};
use rand::{distributions::Alphanumeric, Rng};
use serde::Deserialize;
use std::env;

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

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Song {
    id: String,
    cover_art: Option<String>,
    artist: String,
    title: String,
}

async fn random_song() -> Result<Song> {
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
    serde_json::from_value(val["subsonic-response"]["randomSongs"]["song"][0].clone())
        .context("Failed to deserialize Song")
}

fn download_song_url(id: &str) -> Result<String> {
    Ok(format!(
        "{server_url}/rest/stream{params}&id={id}",
        params = params()?,
        server_url =
            env::var("SUBSONIC_SERVER_URL").context("Failed to read SUBSONIC_SERVER_URL")?
    ))
}

fn cover_art_url(song: &Song) -> Result<String> {
    let id = song
        .cover_art
        .as_ref()
        .ok_or_else(|| anyhow!("Missing cover art"))?;
    Ok(format!(
        "{server_url}/rest/getCoverArt{params}&id={id}",
        params = params()?,
        server_url =
            env::var("SUBSONIC_SERVER_URL").context("Failed to read SUBSONIC_SERVER_URL")?
    ))
}

fn main() {
    dioxus_logger::init(LevelFilter::Debug).expect("failed to init logger");
    dioxus_desktop::launch_cfg(
        App,
        dioxus_desktop::Config::new()
            .with_custom_head(r#"<link rel="stylesheet" href="public/tailwind.css">"#.to_string()),
    );
}

fn App(cx: Scope) -> Element {
    debug!("render");
    use_shortcuts(cx);
    let song_fut = use_future(cx, (), |_| async { random_song().await });

    let Some(song_id) = song_fut.value() else {
        return render! {"loading..."}
    };

    let song = match song_id {
        Ok(song) => song,
        Err(err) => return render! { pre { "{err:?}" } },
    };

    render! {
        div { class: "h-full flex flex-col justify-center items-center", Player { song: song } }
    }
}

#[inline_props]
fn Player<'a>(cx: Scope, song: &'a Song) -> Element {
    let cover_src = cover_art_url(&song).unwrap_or_default();
    let song_url = download_song_url(&song.id).ok()?;
    render! {
        div { class: "w-80 flex flex-col gap-4",
            img { class: "w-80 h-80 bg-slate-400", src: "{cover_src}" }
            div { class: "flex flex-col text-center",
                p { class: "font-bold", "{song.title}" }
                p { "{song.artist}" }
            }
            audio {
                class: "w-full",
                controls: true,
                onplay: |_| debug!("play"),
                onended: |_| debug!("song ended"),
                src: "{song_url}",
                preload: "auto"
            }
        }
    }
}
