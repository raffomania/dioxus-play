use std::rc::Rc;

use anyhow::Result;
use dioxus::prelude::{
    to_owned, use_coroutine, use_eval, Coroutine, EvalError, ScopeState, UseEval,
};
use futures_util::StreamExt;
use log::error;

const JS_AUDIO_ELEMENT_PLAY: &str = r#"
console.log(document.querySelector("audio"));
document.querySelector("audio").play();"#;
const JS_AUDIO_ELEMENT_PAUSE: &str = r#"document.querySelector("audio").pause();"#;

#[derive(Debug)]
pub enum Message {
    Play,
    Pause,
}

fn handle_message(msg: Message, create_eval: &Rc<dyn Fn(&str) -> Result<UseEval, EvalError>>) {
    let res = match msg {
        Message::Play => create_eval(JS_AUDIO_ELEMENT_PLAY),
        Message::Pause => create_eval(JS_AUDIO_ELEMENT_PAUSE),
    };

    if let Err(e) = res {
        error!("{e:#?}");
    }
}

pub fn use_audio_control(cx: &ScopeState) -> Result<&Coroutine<Message>> {
    let create_eval = use_eval(cx);

    let coroutine = use_coroutine(cx, |mut rx: dioxus::prelude::UnboundedReceiver<Message>| {
        to_owned![create_eval];
        async move {
            while let Some(msg) = rx.next().await {
                handle_message(msg, &create_eval);
            }
        }
    });

    Ok(coroutine)
}
