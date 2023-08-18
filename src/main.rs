#![allow(non_snake_case)]
use anyhow::Result;
use rand::{distributions::Alphanumeric, Rng};
use std::{env, iter};

use dioxus::prelude::*;

fn params() -> Result<String> {
    let username = env::var("SUBSONIC_USER")?;
    let password = env::var("SUBSONIC_PASSWORD")?;
    let mut rng = rand::thread_rng();
    let salt: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(36)
        .map(char::from)
        .collect();
    let pre_t = password + &salt;
    let token = format!("{:x}", md5::compute(pre_t.as_bytes()));

    let auth = format!("u={username}&p={token}");
    let format = "json";
    let crate_name = env!("CARGO_PKG_NAME");
    let version = "1.16.1";
    Ok(format!("?{auth}&v={version}&c={crate_name}&f={format}",))
}

fn main() {
    dioxus_desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    let server_url = env::var("SUBSONIC_SERVER_URL")?;
    let url = format!("{server_url}/rest/ping.view{params}", params = params()?);
    let res = use_future(cx, (), |_| reqwest::get(url));
    dbg!(res);
    render! { PlayButton {} }
}

enum PlayState {
    Playing,
    Paused,
}

#[inline_props]
fn PlayButton(cx: Scope) -> Element {
    let play_state = use_state(cx, || PlayState::Paused);
    let text = match **play_state {
        PlayState::Playing => "Pause",
        PlayState::Paused => "Play!",
    };
    render! {
        button { onclick: move |_| {
                play_state
                    .set(
                        match &*play_state.current() {
                            PlayState::Playing => PlayState::Paused,
                            PlayState::Paused => PlayState::Playing,
                        },
                    )
            },
            "{text}"
        }
    }
}
