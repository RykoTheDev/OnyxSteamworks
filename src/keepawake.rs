pub struct KeepAwake;

impl KeepAwake {
    pub fn new() -> Self {
        #[cfg(windows)]
        {
            use windows_sys::Win32::System::Power::{
                ES_CONTINUOUS, ES_SYSTEM_REQUIRED, SetThreadExecutionState,
            };
            unsafe {
                SetThreadExecutionState(ES_CONTINUOUS | ES_SYSTEM_REQUIRED);
            }
        }
        Self
    }
}

impl Drop for KeepAwake {
    fn drop(&mut self) {
        #[cfg(windows)]
        {
            use windows_sys::Win32::System::Power::{ES_CONTINUOUS, SetThreadExecutionState};
            unsafe {
                SetThreadExecutionState(ES_CONTINUOUS);
            }
        }
    }
}
