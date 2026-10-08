use crate::last_error;

use std::panic::{self, AssertUnwindSafe};

pub(crate) fn guard<T>(fallback: T, body: impl FnOnce() -> T) -> T {
    match panic::catch_unwind(AssertUnwindSafe(body)) {
        Ok(value) => value,
        Err(payload) => {
            let message = if let Some(text) = payload.downcast_ref::<&str>() {
                (*text).to_owned()
            } else if let Some(text) = payload.downcast_ref::<String>() {
                text.clone()
            } else {
                "host panicked".to_owned()
            };
            last_error::set_error(format!("host panicked: {message}"));
            fallback
        }
    }
}
