use crate::{OcctResult, native_api::NativeApi, step_doc::StepDoc};

use std::{path::Path, rc::Rc};

pub struct Occt {
    api: Rc<NativeApi>,
}

impl Occt {
    pub fn load(library_dir: &Path, shim_path: &Path) -> OcctResult<Self> {
        Ok(Self {
            api: Rc::new(NativeApi::load(library_dir, shim_path)?),
        })
    }

    pub fn box_volume(&self, dx: f64, dy: f64, dz: f64) -> OcctResult<f64> {
        let mut volume = 0.0;
        // SAFETY: the output lives for the call; api owns the resolved function.
        let status = unsafe { (self.api.box_volume)(dx, dy, dz, &mut volume) };
        if status != 0 {
            return Err(self.api.native_error("box volume"));
        }
        Ok(volume)
    }

    pub fn open_step(&self, path: &Path) -> OcctResult<StepDoc> {
        StepDoc::open(Rc::clone(&self.api), path)
    }

    #[cfg(test)]
    pub(crate) fn weak_api(&self) -> std::rc::Weak<NativeApi> {
        Rc::downgrade(&self.api)
    }
}
