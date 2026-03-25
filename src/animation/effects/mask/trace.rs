//! Contains one-shot tracing helpers for the mask pipeline.
//! When mask debugging is enabled through environment variables, the functions
//! here ensure each interesting warning is logged once instead of flooding every
//! frame.
//!
//! 存放遮罩管线的一次性追踪辅助函数。开启遮罩调试环境变量后，这里的逻辑会
//! 保证每条关键告警只打印一次，而不是在每一帧里无限刷屏。

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
