use std::{sync::mpsc::Sender, thread, time::Duration};

use falcon_key_file::FalconKeyfile;
use log::{debug, error, trace};

use bms_sm::{StringData, StringId};
use std::fs::File;
use std::path::Path;

use std::io::{Read, Seek};

use crate::messages::Message;
use crate::state::State;

pub struct KeyfileWatcher {
    tx: Sender<Message>,
    state: State,
    last_hash: Option<u64>,
}

impl KeyfileWatcher {
    pub fn new(tx: Sender<Message>, state: State) -> Self {
        Self {
            tx,
            state,
            last_hash: None,
        }
    }

    pub fn run(&mut self) {
        loop {
            if self
                .state
                .cancellation_token
                .load(std::sync::atomic::Ordering::Relaxed)
            {
                debug!("Cancelling...");
                break;
            }

            if let Ok(string_data) = StringData::read() {
                let key_file_path = &string_data[&StringId::KeyFile];

                if key_file_path.len() > 0 {
                    trace!("About to read key file: {:?}", key_file_path);
                    let path = Path::new(key_file_path);
                    let mut file = File::open(&path).unwrap();
                    let file_name = String::from(path.file_name().unwrap().to_str().unwrap());

                    let mut buffer = Vec::new();
                    buffer.clear();
                    let _ = file.read_to_end(&mut buffer);

                    let hash = seahash::hash(&buffer);
                    match self.last_hash {
                        Some(old_hash) => {
                            if hash != old_hash {
                                debug!("Key file contents changed, re-reading it.");
                                Self::read_key_file_and_send_result(file_name, &file, &self.tx);
                                self.last_hash = Some(hash);
                            } else {
                                trace!("Keyfile unchanged, hash: {}", hash);
                            }
                        }
                        None => {
                            debug!("Reading key file for the first time.");
                            Self::read_key_file_and_send_result(file_name, &file, &self.tx);
                            self.last_hash = Some(hash);
                        }
                    }
                }
            }

            thread::sleep(Duration::from_secs(5));
        }
    }

    fn read_key_file_and_send_result(file_name: String, mut file: &File, tx: &Sender<Message>) {
        file.rewind().expect("Could not seek to beginning of file.");
        match falcon_key_file::parse(file_name, &file) {
            Ok(key_file) => {
                let message = Message::KeyfileRead { key_file };
                tx.send(message).unwrap();
            }
            Err(e) => error!("Could not read key file: {:?}", e),
        }
    }
}
