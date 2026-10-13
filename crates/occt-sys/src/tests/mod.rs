mod occt;
mod runtime_loading;

// The artifact names come from the crate config through build.rs's
// compiled environment: single source of truth for per-OS naming.
pub(crate) fn shim_name() -> &'static str {
    env!("OCCT_SHIM_FILENAME")
}

pub(crate) fn kernel_name() -> String {
    env!("OCCT_RUNTIME_PATTERN").replace("{tk}", "TKernel")
}
