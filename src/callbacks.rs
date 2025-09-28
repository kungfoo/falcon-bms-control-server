use std::{ffi::CString, sync::mpsc::Receiver, thread, time::Duration};

use falcon_key_file::FalconKeyfile;

use crate::msgpack::ProtocolMessage;
use crate::{messages::Message, state::State};

use crate::keyboard_emulator;
use log::{debug, error, info};

use std::collections::HashMap;

pub struct CallbackSender {
    rx: Receiver<Message>,
    state: State,
    key_file: Option<FalconKeyfile>,
    icp_to_callback_names: HashMap<String, String>,
}

impl CallbackSender {
    pub fn new(rx: Receiver<Message>, state: State) -> Self {
        let icp_to_callback_names = [
            ("1".to_string(), "SimICPTILS".to_string()),
            ("2".to_string(), "SimICPALOW".to_string()),
            ("3".to_string(), "SimICPTHREE".to_string()),
            ("4".to_string(), "SimICPStpt".to_string()),
            ("5".to_string(), "SimICPCrus".to_string()),
            ("6".to_string(), "SimICPSIX".to_string()),
            ("7".to_string(), "SimICPMark".to_string()),
            ("8".to_string(), "SimICPEIGHT".to_string()),
            ("9".to_string(), "SimICPNINE".to_string()),
            ("0".to_string(), "SimICPZERO".to_string()),
            ("RCL".to_string(), "SimICPCLEAR".to_string()),
            ("ENTER".to_string(), "SimICPEnter".to_string()),
            ("COM1".to_string(), "SimICPCom1".to_string()),
            ("COM2".to_string(), "SimICPCom2".to_string()),
            ("IFF".to_string(), "SimICPIFF".to_string()),
            ("LIST".to_string(), "SimICPLIST".to_string()),
            ("A-A".to_string(), "SimICPAA".to_string()),
            ("A-G".to_string(), "SimICPAG".to_string()),
            ("icp-wpt-next".to_string(), "SimICPNext".to_string()),
            ("icp-wpt-previous".to_string(), "SimICPPrevious".to_string()),
            ("icp-ded-up".to_string(), "SimICPDEDUP".to_string()),
            ("icp-ded-down".to_string(), "SimICPDEDDOWN".to_string()),
            ("icp-ded-seq".to_string(), "SimICPDEDSEQ".to_string()),
            ("icp-ded-return".to_string(), "SimICPResetDED".to_string()),
        ]
        .into();

        Self {
            rx,
            state,
            key_file: None,
            icp_to_callback_names,
        }
    }

    pub fn run(&mut self) {
        loop {
            if self
                .state
                .cancellation_token
                .load(std::sync::atomic::Ordering::Relaxed)
            {
                break;
            }
            let message = self.rx.recv();
            if let Ok(message) = message {
                match message {
                    Message::EnetReceived { message } => self.handle_message_received(message),
                    Message::KeyfileRead { key_file } => {
                        self.key_file.replace(key_file);
                    }
                }
            }
        }
    }

    fn handle_message_received(&self, message: ProtocolMessage) {
        match message {
            ProtocolMessage::IcpButtonPressed { icp: _, button } => {
                if let Some(callback_name) = self.icp_to_callback_names.get(&button) {
                    self.invoke_callback(callback_name.clone());
                }
            }
            ProtocolMessage::OsbButtonPressed { mfd, osb } => {
                let mfd_suffix = match mfd.as_str() {
                    "f16/left-mfd" => Some("L"),
                    "f16/right-mfd" => Some("R"),
                    any => {
                        error!("Received unknown mfd identifier: {}", any);
                        None
                    }
                };
                if let Some(mfd_suffix) = mfd_suffix {
                    let callback_name = format!("SimCBE{}{}", osb, mfd_suffix);
                    self.invoke_callback(callback_name);
                }
            }
            ProtocolMessage::IcpButtonReleased { icp: _, button: _ } => {
                // intentionally left blank
            }
            ProtocolMessage::OsbButtonReleased { mfd: _, osb: _ } => {
                // intentionally left blank
            }
            any => debug!(
                "Received a msgpack message that was not supposed to invoke a callback: {:?}",
                any
            ),
        }
    }

    fn invoke_callback(&self, callback: String) {
        if let Some(ref kf) = self.key_file {
            if let Some(callback) = kf.callback(&callback) {
                info!("Received {:?}", callback);

                let window_name = CString::new("Falcon BMS").unwrap();

                unsafe {
                    let window_handle =
                        user32::FindWindowA(std::ptr::null_mut(), window_name.as_ptr());
                    // probably SetForegroundWindow is enough, it was in the other server code.
                    if window_handle.is_null() {
                        error!("Have not found BMS window!");
                        return;
                    }
                    user32::SetForegroundWindow(window_handle);
                    user32::ShowWindow(window_handle, 9);
                }
                thread::sleep(Duration::from_millis(15));
                keyboard_emulator::invoke(callback);
            } else {
                error!("Received unknown callback '{}'", callback);
                error!("Did you mean {:?}?", kf.propose_callback_names(callback, 3));
            }
        }
    }
}
