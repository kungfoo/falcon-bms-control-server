use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Config {
    pub log_level: String,
    pub listen_address: String,
    pub listen_port: u16,
    pub broadcast_port: u16,
    /// how long to wait with keypresses after raising the BMS window, in ms
    pub sleep_after_bms_focus: u16,
    /// how long to wait between key presses, in ms
    pub sleep_between_keypresses: u16,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            log_level: "info".to_string(),
            listen_address: "0.0.0.0".to_string(),
            listen_port: 9022,
            broadcast_port: 9020,
            sleep_after_bms_focus: 15,
            sleep_between_keypresses: 10,
        }
    }
}
