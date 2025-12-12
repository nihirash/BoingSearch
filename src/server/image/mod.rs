use std::io::Cursor;

use image::ImageReader;
use log::debug;
use url::Url;

/// Just plain fetching image
async fn fetch_image_from_url(url_str: &str) -> anyhow::Result<Vec<u8>> {
    let url = Url::parse(url_str)?;

    debug!("Making request for image: {url_str}");

    let body = reqwest::Client::builder()
        .user_agent(crate::USER_AGENT)
        .build()?
        .get(url)
        .send()
        .await?
        .bytes()
        .await?
        .to_vec();

    Ok(body)
}

/// Converts fetched image
fn convert_image(bytes: Vec<u8>) -> anyhow::Result<Vec<u8>> {
    let reader = ImageReader::new(Cursor::new(bytes)).with_guessed_format()?;

    let mut img = reader.decode()?;

    if img.width() > 320 || img.height() > 240 {
        img = img.resize(320, 240, image::imageops::FilterType::Gaussian);
    }

    let outbuf = vec![];
    let mut cursor = Cursor::new(outbuf);

    img.write_to(&mut cursor, image::ImageFormat::Png)?;
    let result = cursor.get_ref().to_vec();

    Ok(result)
}

pub async fn get_converted_picture(url_str: &str) -> anyhow::Result<Vec<u8>> {
    let bytes = fetch_image_from_url(url_str).await?;

    convert_image(bytes)
}

#[cfg(test)]
mod tests {
    use crate::server::image::get_converted_picture;
    use std::io::Write;

    #[tokio::test]
    async fn test_converting_image() -> anyhow::Result<()> {
        let converted = get_converted_picture("https://cataas.com/cat").await?;

        let mut file = std::fs::File::create("test.png")?;
        let _ = file.write_all(&converted);

        Ok(())
    }
}
