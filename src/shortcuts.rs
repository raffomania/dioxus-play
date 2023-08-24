use dioxus::prelude::{use_coroutine, use_eval, Coroutine, ScopeState, UseState};
use log::{trace, warn};
use serde::Deserialize;

#[derive(Debug)]
pub enum Message {
    Next,
}

#[derive(Default)]
pub struct KeyState {
    pub next: bool,
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

pub fn use_shortcuts(cx: &ScopeState, sender: Coroutine<Message>, key_state: &UseState<KeyState>) {
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
                            key_state.set(KeyState {
                                next: pressed,
                                ..*key_state.current()
                            });
                            if pressed {
                                sender.send(Message::Next);
                            }
                        }
                        _ => {}
                    },
                    err => warn!("Unknown message received: {err:?}"),
                }
            }
        }
    });
}
