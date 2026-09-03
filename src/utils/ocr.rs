use std::{path::PathBuf, sync::Arc};

use ocrs::{DecodeMethod, ImageSource, OcrEngine, OcrEngineParams};
use poise::serenity_prelude::Attachment;
use rten::Model;
use tokio::{fs, task::spawn_blocking};

use crate::{Error, utils::IsacError, utils::IsacInfo};

const DETECTION_MODEL_URL: &str =
    "https://ocrs-models.s3-accelerate.amazonaws.com/text-detection.rten";
const RECOGNITION_MODEL_URL: &str =
    "https://ocrs-models.s3-accelerate.amazonaws.com/text-recognition.rten";
const MODEL_DIR: &str = "./user_data/ocr";
const MAX_MODEL_BYTES: u64 = 512 * 1024 * 1024;

pub const MAX_IMAGE_BYTES: usize = 2 * 1024 * 1024;
pub const MAX_IMAGE_DIMENSION: u32 = 4096;
pub const MAX_IMAGE_PIXELS: u64 = 16 * 1024 * 1024;

#[derive(Clone)]
pub struct OcrService {
    engine: Arc<OcrEngine>,
}

impl OcrService {
    pub async fn new(client: &reqwest::Client) -> Result<Self, Error> {
        let model_dir = PathBuf::from(MODEL_DIR);
        fs::create_dir_all(&model_dir).await?;

        let detection_path = model_dir.join("text-detection.rten");
        let recognition_path = model_dir.join("text-recognition.rten");
        ensure_model(client, &detection_path, DETECTION_MODEL_URL).await?;
        ensure_model(client, &recognition_path, RECOGNITION_MODEL_URL).await?;

        let detection_model = load_model(detection_path).await?;
        let recognition_model = load_model(recognition_path).await?;
        let engine = spawn_blocking(move || {
            OcrEngine::new(OcrEngineParams {
                detection_model: Some(detection_model),
                recognition_model: Some(recognition_model),
                decode_method: DecodeMethod::BeamSearch { width: 10 },
                allowed_chars: Some(
                    " 0123456789_[]ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz"
                        .to_string(),
                ),
                ..Default::default()
            })
        })
        .await??;

        Ok(Self {
            engine: Arc::new(engine),
        })
    }

    pub async fn recognize_ign(&self, bytes: Vec<u8>) -> Result<String, IsacError> {
        if bytes.len() > MAX_IMAGE_BYTES {
            return Err(ocr_error(
                "image is too large for OCR (maximum size is 2 MiB)",
            ));
        }

        let engine = Arc::clone(&self.engine);
        spawn_blocking(move || {
            let image = image::load_from_memory(&bytes)
                .map_err(|err| ocr_error(format!("could not read image for OCR: {err}")))?
                .into_rgb8();
            let (width, height) = image.dimensions();
            validate_dimensions(width, height)?;

            let input = engine
                .prepare_input(
                    ImageSource::from_bytes(image.as_raw(), image.dimensions())
                        .map_err(|err| ocr_error(format!("could not read image for OCR: {err}")))?,
                )
                .map_err(|err| ocr_error(format!("could not prepare image for OCR: {err}")))?;
            let text = engine
                .get_text(&input)
                .map_err(|err| ocr_error(format!("could not recognize text in image: {err}")))?;
            extract_ign(&text)
        })
        .await
        .map_err(|err| ocr_error(format!("OCR task failed: {err}")))?
    }

    pub async fn recognize_attachment(&self, attachment: &Attachment) -> Result<String, IsacError> {
        if !attachment
            .content_type
            .as_deref()
            .is_some_and(|content_type| content_type.starts_with("image/"))
        {
            return Err(ocr_error("the attached file is not an image"));
        }
        if attachment.size as usize > MAX_IMAGE_BYTES {
            return Err(ocr_error(
                "image is too large for OCR (maximum size is 2 MiB)",
            ));
        }

        let bytes = attachment
            .download()
            .await
            .map_err(|err| ocr_error(format!("could not download image for OCR: {err}")))?;
        self.recognize_ign(bytes).await
    }
}

async fn ensure_model(client: &reqwest::Client, path: &PathBuf, url: &str) -> Result<(), Error> {
    if let Ok(metadata) = fs::metadata(path).await
        && metadata.len() > 0
    {
        return Ok(());
    }

    let response = client.get(url).send().await?.error_for_status()?;
    if response
        .content_length()
        .is_some_and(|size| size > MAX_MODEL_BYTES)
    {
        return Err(format!("OCR model from {url} is unexpectedly large").into());
    }
    let bytes = response.bytes().await?;
    if bytes.len() as u64 > MAX_MODEL_BYTES {
        return Err(format!("OCR model from {url} is unexpectedly large").into());
    }

    let temporary_path = path.with_extension("rten.download");
    fs::write(&temporary_path, &bytes).await?;
    fs::rename(temporary_path, path).await?;
    Ok(())
}

async fn load_model(path: PathBuf) -> Result<Model, Error> {
    Ok(spawn_blocking(move || Model::load_file(path)).await??)
}

fn validate_dimensions(width: u32, height: u32) -> Result<(), IsacError> {
    if width == 0
        || height == 0
        || width > MAX_IMAGE_DIMENSION
        || height > MAX_IMAGE_DIMENSION
        || u64::from(width) * u64::from(height) > MAX_IMAGE_PIXELS
    {
        return Err(ocr_error("image dimensions are too large for OCR"));
    }
    Ok(())
}

fn extract_ign(text: &str) -> Result<String, IsacError> {
    let lines = text
        .lines()
        .map(str::trim)
        .map(strip_clan_tag)
        .map(remove_ocr_whitespace)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();

    let Some(line) = lines.first() else {
        return Err(ocr_error("OCR could not find a player name in the image"));
    };

    if is_valid_ign(line) {
        Ok(line.to_string())
    } else {
        Err(IsacInfo::InvalidIgn {
            ign: line.to_string(),
        }
        .into())
    }
}

fn strip_clan_tag(line: &str) -> &str {
    if !line.starts_with('[') {
        return line;
    }

    line.find(']')
        .filter(|&end| end > 1)
        .map_or(line, |end| line[end + 1..].trim_start())
}

fn remove_ocr_whitespace(line: &str) -> String {
    line.chars()
        .filter(|character| !character.is_ascii_whitespace())
        .collect()
}

fn ocr_error(message: impl Into<String>) -> IsacError {
    IsacInfo::GeneralError {
        msg: message.into(),
    }
    .into()
}

fn is_valid_ign(ign: &str) -> bool {
    let length = ign.chars().count();
    (3..=24).contains(&length)
        && ign.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '_' | '[' | ']')
        })
}

#[cfg(test)]
mod tests {
    use super::{
        extract_ign, is_valid_ign, remove_ocr_whitespace, strip_clan_tag, validate_dimensions,
    };

    #[test]
    fn accepts_allowed_ign_characters() {
        assert!(is_valid_ign("[B2U]_01"));
        assert!(is_valid_ign("abc"));
    }

    #[test]
    fn rejects_invalid_ign_characters_and_lengths() {
        assert!(!is_valid_ign("ab"));
        assert!(!is_valid_ign("B2U-01"));
        assert!(!is_valid_ign("B2U name"));
        assert!(!is_valid_ign("玩家123"));
        assert!(!is_valid_ign("a".repeat(25).as_str()));
    }

    #[test]
    fn extracts_only_one_valid_line() {
        assert_eq!(extract_ign("[B2U]_01\n").unwrap(), "_01".to_string());
    }

    #[test]
    fn strips_leading_clan_tag_before_validation() {
        assert_eq!(strip_clan_tag("[NEVEN]gjh_ warrior"), "gjh_ warrior");
        assert_eq!(remove_ocr_whitespace("gjh_ warrior"), "gjh_warrior");
        assert_eq!(extract_ign("[NEVEN]gjh_ warrior").unwrap(), "gjh_warrior");
    }

    #[test]
    fn rejects_empty_and_uses_first_line() {
        assert_eq!(
            extract_ign("\n  \n").unwrap_err().to_string(),
            "IsacInfo: ❌ OCR could not find a player name in the image"
        );
        assert_eq!(extract_ign("B2U\nPlayer").unwrap(), "B2U");
    }

    #[test]
    fn rejects_invalid_ocr_line() {
        assert_eq!(
            extract_ign("B2U-01").unwrap_err().to_string(),
            "IsacInfo: ❌ Invalid ign: `B2U-01`"
        );
    }

    #[test]
    fn rejects_oversized_dimensions() {
        assert!(validate_dimensions(4096, 4096).is_ok());
        assert!(validate_dimensions(4097, 1).is_err());
        assert!(validate_dimensions(4096, 4097).is_err());
    }
}
