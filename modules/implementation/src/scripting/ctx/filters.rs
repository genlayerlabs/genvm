#[derive(Debug, Clone, serde::Deserialize)]
pub enum TextFilter {
    NFC,
    NFKC,
    NormalizeWS,
    RmZeroWidth,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub enum ImageFilter {
    Denoise(f32),
    Unsharpen(f32, f32),
    GuassianNoise(f32),
    JPEG(f32),
}

fn normalize_whitespace(input: &str) -> String {
    let mut result = String::new();
    let mut consecutive_spaces = 0;
    let mut consecutive_newlines = 0;
    let chars_iter = input.chars().peekable();

    for ch in chars_iter {
        match ch {
            '\n' => {
                if result.is_empty() {
                    continue;
                }

                while result.ends_with(' ') {
                    result.pop();
                }

                consecutive_newlines += 1;
                consecutive_spaces = 0;

                if consecutive_newlines <= 2 {
                    result.push('\n');
                }
            }
            c if c.is_whitespace() => {
                // Convert other whitespace to space (same logic as space)
                if !result.is_empty() && !result.ends_with('\n') {
                    consecutive_spaces += 1;
                    consecutive_newlines = 0;

                    if consecutive_spaces == 1 {
                        result.push(' ');
                    }
                }
            }
            _ => {
                // Regular character
                consecutive_spaces = 0;
                consecutive_newlines = 0;
                result.push(ch);
            }
        }
    }

    while result.ends_with(' ') {
        result.pop();
    }

    result
}

pub fn apply_filters(s: &str, filters: &[TextFilter]) -> String {
    use unicode_normalization::UnicodeNormalization;
    let mut result = s.to_string();
    for filter in filters {
        match filter {
            TextFilter::NFC => {
                result = result.nfc().collect();
            }
            TextFilter::NFKC => result = result.nfkc().collect(),
            TextFilter::NormalizeWS => {
                result = normalize_whitespace(&result);
            }
            TextFilter::RmZeroWidth => {
                result.retain(|c| {
                    use unicode_width::UnicodeWidthChar;
                    if c == ' ' || c == '\n' {
                        true
                    } else if let Some(w) = c.width() {
                        w > 0
                    } else {
                        false
                    }
                });
            }
        };
    }
    result
}

pub fn apply_image_filters(s: &[u8], filters: &[ImageFilter]) -> anyhow::Result<Vec<u8>> {
    let mut img = image::ImageReader::new(std::io::Cursor::new(s))
        .with_guessed_format()?
        .decode()?;

    for filter in filters {
        match filter {
            ImageFilter::Denoise(_strength) => {
                anyhow::bail!("todo");
            }
            ImageFilter::GuassianNoise(sigma) => {
                img = img.fast_blur(*sigma);
            }
            ImageFilter::Unsharpen(sigma, threshold) => {
                img = img.unsharpen(*sigma, *threshold as i32);
            }
            ImageFilter::JPEG(quality) => {
                let mut out = Vec::new();

                let q = (quality.clamp(0.0, 1.0) * 100.0) as u8;
                image::codecs::jpeg::JpegEncoder::new_with_quality(
                    &mut std::io::Cursor::new(&mut out),
                    q,
                )
                .encode_image(&img)?;

                img = image::ImageReader::new(std::io::Cursor::new(&out))
                    .with_guessed_format()?
                    .decode()?;
            }
        }
    }

    let mut bytes: Vec<u8> = Vec::new();
    img.write_to(
        &mut std::io::Cursor::new(&mut bytes),
        image::ImageFormat::Png,
    )?;
    Ok(bytes)
}
