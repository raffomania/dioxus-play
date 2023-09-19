use dioxus::prelude::{use_coroutine, use_eval, Coroutine, ScopeState, UseRef};
use log::{trace, warn};
use serde::Deserialize;

#[derive(Debug)]
pub enum Message {
    Next,
    PlayPause,
    ToggleStar,
}

#[derive(Default)]
pub struct KeyState {
    pub next: bool,
    pub play_pause: bool,
    pub toggle_star: bool,
}

#[derive(Debug, Deserialize)]
struct JsMessage {
    pressed: bool,
    key: String,
}

const JS_KEY_EVENT_LISTENER: &str = r#"
let isShortcutEvent = (e) => {
    let element = e.target || e.srcElement;
    let isTextInput = element.tagName == 'INPUT' || element.tagName == 'SELECT' || element.tagName == 'TEXTAREA' || element.isContentEditable; 
    return !isTextInput && !e.repeat;
};
document.addEventListener("keydown", (e) => {
    if (isShortcutEvent(e)) {
        dioxus.send({ pressed: true, key: event.key });
    }
});
document.addEventListener("keyup", (e) => {
    if (isShortcutEvent(e)) {
        dioxus.send({ pressed: false, key: event.key });
    }
});
"#;

pub fn use_shortcuts(cx: &ScopeState, sender: Coroutine<Message>, key_state: &UseRef<KeyState>) {
    let create_eval = use_eval(cx);

    let eval = create_eval(JS_KEY_EVENT_LISTENER).unwrap();

    let key_state = key_state.to_owned();

    use_coroutine(cx, |_: dioxus::prelude::UnboundedReceiver<()>| async move {
        loop {
            let msg = eval.recv().await;

            if let Ok(msg) = msg {
                trace!("{msg:?}");
                match serde_json::from_value(msg) {
                    Ok(JsMessage { key, pressed }) => match key.as_str() {
                        "l" => {
                            if pressed {
                                sender.send(Message::Next);
                            }
                            key_state.write().next = pressed;
                        }
                        " " => {
                            if pressed {
                                sender.send(Message::PlayPause);
                            }
                            key_state.write().play_pause = pressed;
                        }
                        "Enter" => {
                            if pressed {
                                sender.send(Message::ToggleStar);
                            }
                            key_state.write().toggle_star = pressed;
                        }
                        _key => {
                            // log::debug!("{key}");
                        }
                    },
                    err => warn!("Unknown message received: {err:?}"),
                }
            }
        }
    });
}
