use bms_sm::{FlightData2, MemoryFile, RttTextures};
use image::RgbImage;
use log::error;

/// an identifier and the channel to send on.
#[derive(Debug, Clone)]
#[repr(u8)]
pub enum TextureId {
    LeftMfd = 1,
    RightMfd = 2,
    Ded = 3,
    Rwr = 4,
    Unknown = 99,
}

impl From<&str> for TextureId {
    fn from(value: &str) -> Self {
        match value {
            "f16/left-mfd" => TextureId::LeftMfd,
            "f16/right-mfd" => Self::RightMfd,
            "f16/ded" => TextureId::Ded,
            "f16/rwr" => TextureId::Rwr,
            _ => TextureId::Unknown,
        }
    }
}

/// Keeps the `FlightData2` shared-memory mapping open across frames to avoid
/// re-opening/unmapping it 30-60 times per second per stream.
#[derive(Default)]
pub struct TextureReader {
    flight_data: Option<MemoryFile<'static, FlightData2>>,
}

impl TextureReader {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn read(&mut self, texture_id: TextureId) -> Result<RgbImage, std::io::Error> {
        let textures = match RttTextures::read() {
            Ok(t) => t,
            Err(e) => {
                // Drop cached FlightData2 handle so we re-open cleanly when BMS restarts.
                self.flight_data = None;
                return Err(std::io::Error::new(
                    std::io::ErrorKind::NotConnected,
                    format!("Failed to read RttTextures shared memory: {:?}", e),
                ));
            }
        };

        if self.flight_data.is_none() {
            let fd = FlightData2::new().map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::NotConnected,
                    format!("Failed to open FlightData2 shared memory: {:?}", e),
                )
            })?;
            self.flight_data = Some(fd);
        }

        let flight_data2 = self.flight_data.as_ref().unwrap().read();

        match texture_id {
            TextureId::LeftMfd => {
                let c = flight_data2.get_rtt_area(bms_sm::RttArea::MfdLeft);
                Ok(textures.get_image(c.left, c.top, c.right, c.bottom))
            }
            TextureId::RightMfd => {
                let c = flight_data2.get_rtt_area(bms_sm::RttArea::MfdRight);
                Ok(textures.get_image(c.left, c.top, c.right, c.bottom))
            }
            TextureId::Ded => {
                let c = flight_data2.get_rtt_area(bms_sm::RttArea::Ded);
                Ok(textures.get_image(c.left, c.top, c.right, c.bottom))
            }
            TextureId::Rwr => {
                let c = flight_data2.get_rtt_area(bms_sm::RttArea::Rwr);
                Ok(textures.get_image(c.left, c.top, c.right, c.bottom))
            }
            _ => {
                error!("Unhandled texture id: {:?}", texture_id);
                Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("Unhandled texture id: {:?}", texture_id),
                ))
            }
        }
    }
}
