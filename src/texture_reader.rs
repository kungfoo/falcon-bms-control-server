use bms_sm::{FlightData2, RttTextures};
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

pub fn rtt_texture_read(texture_id: TextureId) -> Result<RgbImage, std::io::Error> {
    let tx_result = RttTextures::read();
    let fd_result = FlightData2::new();

    if let (Ok(textures), Ok(flight_data)) = (tx_result, fd_result) {
        let flight_data2 = flight_data.read();

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
                Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, ""))
            }
        }
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::NotConnected,
            "BMS is not running or exporting.",
        ))
    }
}
