#![allow(non_snake_case)]
use anyhow::{anyhow, Context, Result};
use dioxus::prelude::*;
use futures_util::StreamExt;
use log::{debug, error, LevelFilter};
use net::get_json;
use rand::{distributions::Alphanumeric, seq::IteratorRandom, thread_rng, Rng};
use serde::Deserialize;
use std::{collections::HashSet, env};

use crate::{
    audio_control::use_audio_control,
    shortcuts::{use_shortcuts, KeyState},
    starred_songs::use_starred_songs,
};

mod audio_control;
mod net;
mod shortcuts;
mod starred_songs;

#[derive(Deserialize, Debug, Clone, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
struct Song {
    id: String,
    cover_art: Option<String>,
    artist: String,
    title: String,
}

struct AudioState {
    playing: bool,
}

async fn random_song(credentials: &Credentials) -> Result<Song> {
    let val: serde_json::Value = get_json(credentials, "getRandomSongs", "&size=1").await?;
    serde_json::from_value(val["subsonic-response"]["randomSongs"]["song"][0].clone())
        .context("Failed to deserialize Song")
}

fn song_stream_url(id: &str, credentials: &Credentials) -> Result<String> {
    net::url(credentials, "stream", &format!("&id={id}"))
}

fn cover_art_url(song: &Song, credentials: &Credentials) -> Result<String> {
    let id = song
        .cover_art
        .as_ref()
        .ok_or_else(|| anyhow!("Missing cover art"))?;
    net::url(credentials, "getCoverArt", &format!("&id={id}"))
}

fn main() {
    dioxus_logger::init(LevelFilter::Debug).expect("failed to init logger");
    dioxus_desktop::launch_cfg(
        App,
        dioxus_desktop::Config::new().with_custom_head(
            r#"
            <link rel="stylesheet" href="public/tailwind.css">
            "#
            .to_string(),
        ),
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

#[derive(Clone)]
pub struct Credentials {
    salt: String,
    username: String,
    password: String,
}

async fn handle_shortcut_message(
    msg: shortcuts::Message,
    credentials: &UseSharedState<Credentials>,
    current_song: &UseState<Option<Result<Song>>>,
    audio_state: &UseRef<AudioState>,
    audio_control: &Coroutine<audio_control::Message>,
    starred_songs: &Coroutine<starred_songs::Message>,
) {
    match msg {
        shortcuts::Message::Next => {
            let new_song = random_song(&credentials.read()).await;
            debug!("{new_song:?}");
            current_song.set(Some(new_song));
            audio_state.write_silent().playing = true;
        }
        shortcuts::Message::PlayPause => {
            let playing = audio_state.read().playing;
            let msg = match playing {
                true => audio_control::Message::Pause,
                false => audio_control::Message::Play,
            };
            audio_control.send(msg);
            audio_state.write_silent().playing = !playing;
        }
        shortcuts::Message::ToggleStar => {
            let current_song_id = match &*current_song.current() {
                Some(Ok(song)) => song.id.clone(),
                other => {
                    error!("No current song, can't star it: {other:?}");
                    return;
                }
            };
            starred_songs.send(starred_songs::Message::ToggleStar(current_song_id))
        }
    }
}

fn create_credentials() -> Result<Credentials> {
    let username = env::var("SUBSONIC_USER").context("Failed to read SUBSONIC_USER")?;
    let password = env::var("SUBSONIC_PASSWORD").context("Failed to read SUBSONIC_PASSOWRD")?;
    let salt = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(36)
        .map(char::from)
        .collect();

    Ok(Credentials {
        username,
        password,
        salt,
    })
}

fn App(cx: Scope) -> Element {
    debug!("render");
    let audio_state = use_ref(cx, || AudioState { playing: false });

    let Ok(audio_control) = use_audio_control(cx) else {
        return render! { "Error initializing audio control" };
    };

    use_shared_state_provider::<Credentials>(cx, || match create_credentials() {
        Ok(c) => c,
        Err(e) => {
            error!("Error creating credentials: {e:#?}");
            panic!();
        }
    });

    let Some(credentials) = use_shared_state::<Credentials>(cx) else {
        return render! { "Not logged in" };
    };

    let key_state = use_ref(cx, || KeyState::default());
    let current_song = use_state(&cx, || None::<Result<Song>>);
    let starred_song_ids = use_state(cx, || None);

    let starred_songs = use_starred_songs(cx, credentials.read().clone(), starred_song_ids);

    let shortcut_listener = use_coroutine(cx, |mut rx: UnboundedReceiver<shortcuts::Message>| {
        to_owned![
            audio_state,
            audio_control,
            credentials,
            current_song,
            starred_songs
        ];
        async move {
            while let Some(msg) = rx.next().await {
                handle_shortcut_message(
                    msg,
                    &credentials,
                    &current_song,
                    &audio_state,
                    &audio_control,
                    &starred_songs,
                )
                .await;
            }
        }
    });

    use_shortcuts(cx, shortcut_listener.clone(), &key_state);

    let player = match &*current_song.get() {
        Some(Ok(song)) => Some(rsx! {
            Player {
                song: song.clone(),
                onplaychange: |playing| audio_state.write().playing = playing,
                onended: |_| shortcut_listener.send(shortcuts::Message::Next)
            }
        }),
        Some(Err(err)) => Some(rsx! { p { class: "whitespace-pre-wrap", "{err:?}" } }),
        _ => None,
    };

    let play_key_desc = if audio_state.read().playing {
        "pause"
    } else {
        "play"
    };

    let play_pause_button = if current_song.get().is_some() {
        Some(rsx! {
            KeyButton {
                class: "w-32",
                pressed: key_state.read().play_pause,
                onclick: |_| shortcut_listener.send(shortcuts::Message::PlayPause),
                key_label: "␣",
                play_key_desc
            }
        })
    } else {
        None
    };

    render! {
        div { class: "h-full flex flex-col justify-center items-center p-8",
            player,
            div { class: "flex mt-8",
                play_pause_button,
                ToggleStarButton {
                    current_song: current_song.get(),
                    starred_songs: starred_song_ids.get(),
                    key_state: key_state,
                    onclick: |_| shortcut_listener.send(shortcuts::Message::ToggleStar)
                }
                KeyButton {
                    pressed: key_state.read().next,
                    onclick: |_| shortcut_listener.send(shortcuts::Message::Next),
                    key_label: "L",
                    "next"
                }
            }
        }
    }
}

#[inline_props]
fn KeyButton<'a>(
    cx: Scope,
    pressed: bool,
    onclick: EventHandler<'a, MouseEvent>,
    key_label: &'a str,
    children: Element<'a>,
    class: Option<&'a str>,
) -> Element {
    let key_style = if *pressed {
        "border-b-2 mt-[2px]"
    } else {
        "border-b-4"
    };

    let class = class.unwrap_or_default();

    render! {
        button { class: "group px-4 py-1 {class}", onclick: |e| onclick.call(e),
            span { class: "inline-block w-6 h-6 mr-2
            box-content
            {key_style}
            bg-white border border-slate-300 rounded
            drop-shadow-sm 
            group-active:border-b-2 group-active:mt-[2px]",
                "{key_label}"
            }
            children
        }
    }
}

#[inline_props]
fn ToggleStarButton<'a>(
    cx: Scope,
    #[props(!optional)] current_song: &'a Option<Result<Song>>,
    #[props(!optional)] starred_songs: &'a Option<HashSet<String>>,
    key_state: &'a UseRef<KeyState>,
    onclick: EventHandler<'a, ()>,
) -> Element {
    let song_id = &current_song.as_ref()?.as_ref().ok()?.id;
    let starred_songs = starred_songs.as_ref()?;
    let description = match starred_songs.contains(song_id) {
        true => "unlike",
        false => "like",
    };
    render! {
        KeyButton {
            pressed: key_state.read().toggle_star,
            onclick: |_| onclick.call(()),
            key_label: "↵",
            "{description}"
        }
    }
}

#[inline_props]
fn Player<'a>(
    cx: Scope,
    song: Song,
    onplaychange: EventHandler<'a, bool>,
    onended: EventHandler<'a, ()>,
) -> Element {
    let Some(credentials) = use_shared_state::<Credentials>(cx) else {
        return render! { "Not logged in"};
    };
    let Song { title, artist, .. } = song;
    let cover_src = cover_art_url(&song, &credentials.read()).unwrap_or_default();
    let song_url = song_stream_url(&song.id, &credentials.read()).ok()?;
    render! {
        div { class: "w-80 flex flex-col gap-4",
            img { class: "w-80 h-80 bg-slate-400", src: "{cover_src}" }
            div { class: "flex flex-col text-center",
                p { class: "font-bold", "{title}" }
                p { "{artist}" }
            }
            Audio {
                song_url: song_url,
                onplaychange: |playing| onplaychange.call(playing),
                onended: |_| onended.call(())
            }
        }
    }
}

#[inline_props]
fn Audio<'a>(
    cx: Scope,
    song_url: String,
    onplaychange: EventHandler<'a, bool>,
    onended: EventHandler<'a, ()>,
) -> Element {
    render! {
        audio {
            class: "w-full",
            src: "{song_url}",
            autoplay: "true",
            controls: true,
            preload: "auto",
            onplay: |_| onplaychange.call(true),
            onpause: |_| onplaychange.call(false),
            onended: |_| onended.call(())
        }
    }
}
