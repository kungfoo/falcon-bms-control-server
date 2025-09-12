use std::{ffi::CString, sync::mpsc::Receiver, thread, time::Duration};

use falcon_key_file::{Callback, FalconKeyfile};

use crate::{messages::Message, state::State};

use crate::keyboard_emulator;
use log::{debug, error, info};

pub struct CallbackSender {
    rx: Receiver<Message>,
    state: State,
    key_file: Option<FalconKeyfile>,
}

impl CallbackSender {
    pub fn new(rx: Receiver<Message>, state: State) -> Self {
        Self {
            rx,
            state,
            key_file: None,
        }
    }

    pub fn run(&mut self) {
        loop {
            let message = self.rx.recv();
            if let Ok(message) = message {
                match message {
                    Message::CallbackReceived { callback } => self.handle_callback(callback),
                    Message::KeyfileRead { key_file } => {
                        self.key_file.replace(key_file);
                    }
                }
            }
        }
    }

    fn handle_callback(&self, callback: String) {
        if let Some(ref kf) = self.key_file {
            if let Some(callback) = kf.callback(&callback) {
                info!("Received {:?}", callback);

                let window_name = CString::new("Falcon BMS").unwrap();

                unsafe {
                    let window_handle =
                        user32::FindWindowA(std::ptr::null_mut(), window_name.as_ptr());
                    // probably SetForegroundWindow is enough, it was in the other server code.
                    if window_handle == std::ptr::null_mut() {
                        error!("Have not found BMS window!");
                        return;
                    }
                    user32::SetForegroundWindow(window_handle);
                    user32::ShowWindow(window_handle, 9);
                }
                thread::sleep(Duration::from_millis(30));
                keyboard_emulator::invoke(callback);
            } else {
                error!("Received unknown callback '{}'", callback);
                error!("Did you mean {:?}?", kf.propose_callback_names(callback, 3));
            }
        }
    }
}
