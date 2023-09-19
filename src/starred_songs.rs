use crate::{net::get_json, Credentials};
use anyhow::{anyhow, Result};
use dioxus::prelude::{to_owned, use_coroutine, Coroutine, ScopeState, UseState};
use futures_util::StreamExt;
use log::error;
use std::collections::HashSet;

#[derive(Debug)]
pub enum Message {
    ToggleStar(String),
}

async fn handle_message(
    msg: Message,
    starred_songs_state: &UseState<Option<HashSet<String>>>,
    credentials: &Credentials,
) -> Result<()> {
    match msg {
        Message::ToggleStar(song_id) => {
            let starred_songs = &*starred_songs_state.current();
            let starred_songs = starred_songs
                .as_ref()
                .ok_or(anyhow!("No starred songs found"))?;
            let starred = !starred_songs.contains(&song_id);
            set_song_starred(&credentials, starred, &song_id).await?;

            let songs = starred_song_ids(&credentials).await?;
            starred_songs_state.set(Some(songs.clone()));
        }
    };
    Ok(())
}

pub fn use_starred_songs<'cx>(
    cx: &'cx ScopeState,
    credentials: Credentials,
    starred_songs_state: &'cx UseState<Option<HashSet<String>>>,
) -> &'cx Coroutine<Message> {
    use_coroutine(cx, |mut rx: dioxus::prelude::UnboundedReceiver<Message>| {
        to_owned![starred_songs_state];
        async move {
            let res = starred_song_ids(&credentials).await;
            if let Ok(songs) = res {
                starred_songs_state.set(Some(songs.clone()));
            } else {
                error!("{res:?}");
                return;
            }

            while let Some(msg) = rx.next().await {
                if let Err(e) = handle_message(msg, &starred_songs_state, &credentials).await {
                    error!("{e:?}");
                }
            }
        }
    })
}

async fn set_song_starred(credentials: &Credentials, starred: bool, song_id: &str) -> Result<()> {
    let endpoint = match starred {
        true => "star",
        false => "unstar",
    };
    get_json::<serde_json::Value>(credentials, endpoint, &format!("&id={song_id}")).await?;

    Ok(())
}

async fn starred_song_ids(credentials: &Credentials) -> Result<HashSet<String>> {
    let val: serde_json::Value = get_json(credentials, "getStarred2", "").await?;
    val["subsonic-response"]["starred2"]["song"]
        .as_array()
        // If there are no liked songs at all, the "song" object will be empty
        .unwrap_or(&Vec::new())
        .into_iter()
        .map(|v| {
            v["id"]
                .as_str()
                .map(ToString::to_string)
                .ok_or(anyhow!("Song ID was not a string"))
        })
        .collect()
}
