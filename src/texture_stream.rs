use std::time::Duration;

use tokio_util::sync::CancellationToken;

use crate::state::StreamKey;

pub struct TextureStream {
    cancellation_token: CancellationToken,
    stream_key: StreamKey,
}

impl TextureStream {
    pub fn new(cancellation_token: CancellationToken, stream_key: StreamKey) -> Self {
        Self {
            cancellation_token,
            stream_key,
        }
    }

    pub async fn run(&self) {
        loop {
            if self.cancellation_token.is_cancelled() {
                break;
            }

            println!("Hello: {:?}", self.stream_key);
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    }
}
