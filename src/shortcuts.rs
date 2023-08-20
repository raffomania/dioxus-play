use dioxus::prelude::ScopeState;
use dioxus_desktop::{
    tao::event::{Event, WindowEvent},
    use_window, DesktopContext, WryEventHandlerId,
};
use log::{debug, info};

pub fn use_shortcuts(cx: &ScopeState) {
    let window = use_window(cx);
    cx.use_hook(|| ShortcutListener::new(window.clone()));
}

struct ShortcutListener {
    id: WryEventHandlerId,
    window: DesktopContext,
}

impl ShortcutListener {
    fn new(window: DesktopContext) -> ShortcutListener {
        debug!("Listening for global keypresses...");
        let listener_id = window.create_wry_event_handler(|event, _target| match event {
            Event::WindowEvent {
                event: WindowEvent::KeyboardInput { event, .. },
                ..
            } => {
                let key = event.key_without_modifiers();
                let state = event.state;
                info!("{state:?} {key:?}");
                // info!("{event:#?}")
            }
            _ => {}
        });

        ShortcutListener {
            id: listener_id,
            window: window,
        }
    }
}

impl Drop for ShortcutListener {
    fn drop(&mut self) {
        info!("Drop");
        self.window.remove_wry_event_handler(self.id);
    }
}
