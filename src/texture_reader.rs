use std::fs::read;
use turbojpeg::Image;

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

pub fn rtt_texture_read(texture_id: TextureId) -> Result<Image<Vec<u8>>, std::io::Error> {
    match texture_id {
        TextureId::LeftMfd => read_jpeg("images/left-mfd.jpeg"),
        TextureId::RightMfd => read_jpeg("images/right-mfd.jpeg"),
        TextureId::Ded => read_jpeg("images/ded.jpeg"),
        _ => {
            // error!("Unhandled texture id: {:?}", texture_id);
            Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, ""))
        }
    }
}

fn read_jpeg(path: &str) -> Result<Image<Vec<u8>>, std::io::Error> {
    let data = read(path).unwrap();

    let image = turbojpeg::decompress(&data, turbojpeg::PixelFormat::RGB).unwrap();
    Ok(image)
}
