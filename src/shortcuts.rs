use dioxus::prelude::{use_coroutine, use_eval, ScopeState};
use log::{debug, info};

pub fn use_shortcuts(cx: &ScopeState) {
    debug!("Listening for global keypresses...");
    let create_eval = use_eval(cx);

    let eval = create_eval(
            r#"
                dioxus.send("eval");
                document.addEventListener("keydown", (e) => {
                    let element = e.target || e.srcElement;
                    let isTextInput = element.tagName == 'INPUT' || element.tagName == 'SELECT' || element.tagName == 'TEXTAREA' || element.isContentEditable; 
                    if (isTextInput) {
                        return;
                    }
                    dioxus.send(event.key);
                });
            "#,
        )
        .unwrap();

    use_coroutine(cx, |_: dioxus::prelude::UnboundedReceiver<()>| async move {
        loop {
            let msg = eval.recv().await.unwrap();
            info!("{msg}");
        }
    });
}
