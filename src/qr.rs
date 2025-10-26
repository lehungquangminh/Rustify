use anyhow::Result;
use qrcode::QrCode;

pub fn qr_png(data: &str) -> Result<Vec<u8>> {
    let code = QrCode::new(data.as_bytes())?;
    let img = code.render::<image::Luma<u8>>().build();
    let mut buf = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut buf);
    let dyn_img = image::DynamicImage::ImageLuma8(img);
    dyn_img.write_to(&mut cursor, image::ImageFormat::Png)?;
    Ok(buf)
}
