use occt_sys::Occt;

use std::path::Path;

pub(crate) struct Host {
    occt: Occt,
}

impl Host {
    pub(crate) fn new(occt_dir: &Path, shim_path: &Path) -> occt_sys::OcctResult<Self> {
        let occt = Occt::load(occt_dir, shim_path)?;
        Ok(Self { occt })
    }

    pub(crate) fn occt(&self) -> &Occt {
        &self.occt
    }
}
