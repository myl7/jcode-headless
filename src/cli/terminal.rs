//! Headless process panic handling.

use std::panic;

pub fn get_current_session() -> Option<String> {
    crate::get_current_session()
}

pub fn install_panic_hook() {
    let default_hook = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        default_hook(info);

        let Some(session_id) = get_current_session() else {
            return;
        };

        if let Ok(mut session) = crate::session::Session::load(&session_id) {
            session.mark_crashed(Some(format!("Panic: {info}")));
            let _ = session.save();
        }
    }));
}
