#![allow(non_snake_case)]
use anyhow::{anyhow, Context, Result};
use dioxus::prelude::*;
use futures_util::StreamExt;
use log::{debug, LevelFilter};
use rand::{distributions::Alphanumeric, seq::IteratorRandom, thread_rng, Rng};
use serde::Deserialize;
use std::env;

use crate::shortcuts::{use_shortcuts, KeyState};

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

#[derive(Deserialize, Debug)]
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

fn song_stream_url(id: &str) -> Result<String> {
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

#[allow(unused)]
async fn random_test_song() -> Option<Song> {
    Vec::from([
        Song {
            artist: "ABBA".to_string(),
            cover_art: None,
            id: "0".to_string(),
            title: "Super Trouper".to_string(),
        },
        Song {
            artist: "Gorillaz".to_string(),
            cover_art: None,
            id: "2".to_string(),
            title: "A1M1".to_string(),
        },
        Song {
            artist: "Schnitzelpower".to_string(),
            cover_art: None,
            id: "3".to_string(),
            title: "Sicko".to_string(),
        },
        Song {
            artist: "ABBA".to_string(),
            cover_art: None,
            id: "4".to_string(),
            title: "Mamma Mia".to_string(),
        },
        Song {
            artist: "Röyksopp".to_string(),
            cover_art: None,
            id: "5".to_string(),
            title: "Speed King".to_string(),
        },
    ])
    .into_iter()
    .choose(&mut thread_rng())
}

fn App(cx: Scope) -> Element {
    debug!("render");

    let current_song = use_state(&cx, || None::<Result<Song>>);
    let shortcut_listener = use_coroutine(cx, |mut rx: UnboundedReceiver<shortcuts::Message>| {
        let current_song = current_song.to_owned();
        async move {
            while let Some(msg) = rx.next().await {
                match msg {
                    shortcuts::Message::Next => {
                        let new_song = random_song().await;
                        debug!("{new_song:?}");
                        current_song.set(Some(new_song));
                    }
                }
            }
        }
    });
    let key_state = use_state(cx, || KeyState::default());
    use_shortcuts(cx, shortcut_listener.clone(), &key_state);

    let player = match &*current_song.get() {
        Some(Ok(ref song)) => Some(
            rsx! {Player { song: &song, on_next: |_| shortcut_listener.send(shortcuts::Message::Next) }},
        ),
        Some(Err(err)) => return render! { pre { "{err:?}" } },
        _ => None,
    };

    let button_class = if key_state.next {
        "border font-bold"
    } else {
        "active:border active:font-bold"
    };

    render! {
        div { class: "h-full flex flex-col justify-center items-center",
            player,
            button {
                class: "{button_class} px-4 py-1 border-slate-400 rounded",
                onclick: move |_| shortcut_listener.send(shortcuts::Message::Next),
                "[l] next"
            }
        }
    }
}

#[inline_props]
fn Player<'a>(cx: Scope, song: &'a Song, on_next: EventHandler<'a, MediaEvent>) -> Element {
    let Song { title, artist, .. } = song;
    let cover_src = cover_art_url(&song).unwrap_or_default();
    let song_url = song_stream_url(&song.id).ok()?;
    render! {
        div { class: "w-80 flex flex-col gap-4",
            img { class: "w-80 h-80 bg-slate-400", src: "{cover_src}" }
            div { class: "flex flex-col text-center",
                p { class: "font-bold", "{title}" }
                p { "{artist}" }
            }
            audio {
                class: "w-full",
                controls: true,
                onplay: |_| debug!("play"),
                onended: |e| on_next.call(e),
                src: "{song_url}",
                preload: "auto"
            }
        }
    }
}
