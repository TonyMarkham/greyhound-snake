use std::{
    cell::RefCell,
    ffi::{CString, c_char},
};

thread_local! {
    static LAST_ERROR: RefCell<Option<CString>> = const { RefCell::new(None) };
}

pub(crate) fn set_error(message: impl Into<String>) {
    let message = message.into();
    let bytes = message.as_bytes();
    let end = bytes
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(bytes.len());
    let Ok(value) = CString::new(&bytes[..end]) else {
        return;
    };
    LAST_ERROR.with(|slot| *slot.borrow_mut() = Some(value));
}

pub(crate) fn clear_error() {
    LAST_ERROR.with(|slot| *slot.borrow_mut() = None);
}

pub(crate) fn last_error() -> *const c_char {
    LAST_ERROR.with(|slot| {
        let borrowed = slot.borrow();
        match borrowed.as_ref() {
            Some(value) => value.as_ptr(),
            None => std::ptr::null(),
        }
    })
}
