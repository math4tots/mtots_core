use super::*;
use crate::ordie;

impl Globals {
    /// If there is any pending trampoline request, handle it.
    pub fn handle_trampoline(mut self) {
        if let Some(trampoline) = std::mem::replace(&mut self.trampoline, None) {
            trampoline(self)
        }
    }

    pub fn handle_trampoline_and_last_result<T>(mut self, r: Result<T>) {
        match r {
            Ok(_) => {}
            Err(error) => {
                if error.type_().str() == "TrampolineRequest" {
                    if let Some(trampoline) = std::mem::replace(&mut self.trampoline, None) {
                        trampoline(self)
                    }
                } else {
                    ordie::<()>(&mut self, Err(error));
                }
            }
        }
    }

    pub fn request_trampoline<R, F>(&mut self, trampoline: F) -> Result<R>
    where
        F: FnOnce(Globals) + 'static,
    {
        if self.trampoline.is_some() {
            Err(rterr!("There is already a pending trampoline request"))
        } else {
            self.trampoline = Some(Box::new(trampoline));
            Err(Error::new("TrampolineRequest".into(), "".into(), vec![]))
        }
    }
}
