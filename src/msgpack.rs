use crate::Serialize;
use serde::Deserialize;

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Message {
    #[serde(rename = "hello")]
    Hello {},
    #[serde(rename = "ack")]
    Ack {},
    #[serde(rename = "icp-pressed")]
    IcpButtonPressed {
        icp: String,
        button: String,
    },
    #[serde(rename = "icp-released")]
    IcpButtonReleased {
        icp: String,
        button: String,
    },
    #[serde(rename = "osb-pressed")]
    OsbButtonPressed {
        mfd: String,
        osb: String,
    },
    #[serde(rename = "osb-released")]
    OsbButtonReleased {
        mfd: String,
        osb: String,
    },
    #[serde(rename = "streamed-texture")]
    StreamedTextureRequest {
        identifier: String,
        command: Command,
        refresh_rate: Option<i64>,
        quality: Option<i64>,
    },
    Unknown,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Command {
    #[serde(rename = "start")]
    Start,
    #[serde(rename = "stop")]
    Stop,
}
