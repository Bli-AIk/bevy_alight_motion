use std::{
    collections::HashSet,
    sync::{Mutex, OnceLock},
};

pub(super) fn trace_mask_once(key: impl Into<String>, message: impl FnOnce() -> String) {
    if std::env::var_os("AM_MASK_TRACE").is_none() {
        return;
    }

    static SEEN: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    let seen = SEEN.get_or_init(|| Mutex::new(HashSet::new()));
    let key = key.into();

    let should_log = {
        let mut guard = seen.lock().expect("mask trace mutex poisoned");
        guard.insert(key)
    };

    if should_log {
        bevy::log::warn!("{}", message());
    }
}
