use falcon_key_file::FalconKeyfile;

use crate::msgpack;

#[derive(Debug)]
pub enum Message {
    /// Sent when we received a callback
    EnetReceived {
        message: msgpack::ProtocolMessage,
    },

    // Sent whenever a new kezfile has been read
    KeyfileRead {
        key_file: FalconKeyfile,
    },
}
