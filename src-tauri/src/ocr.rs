//! Text recognition with the OCR engine built into Windows (Windows.Media.Ocr), for reading the
//! augment cards. Runs on the calling thread and blocks; call it from a blocking task.

use anyhow::{anyhow, Result};
use windows::core::HSTRING;
use windows::Globalization::Language;
use windows::Graphics::Imaging::{BitmapPixelFormat, SoftwareBitmap};
use windows::Media::Ocr::OcrEngine;
use windows::Storage::Streams::DataWriter;

/// One line of recognized text and where it is in the image (pixels), with its words.
#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    pub text: String,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    /// The words of the line and where each is. The engine's lines are not reliable: it can cut
    /// one title into two lines ("Infinite" and "Recursion").
    pub words: Vec<Word>,
}

/// One recognized word and where it is in the image (pixels).
#[derive(Clone, Debug, PartialEq)]
pub struct Word {
    pub text: String,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// "en" or "zh" to the OCR language tag.
fn tag(lang: &str) -> &'static str {
    if lang.starts_with("zh") {
        "zh-Hans-CN"
    } else {
        "en-US"
    }
}

/// Whether Windows can read this language (its OCR language pack is installed).
pub fn available(lang: &str) -> bool {
    Language::CreateLanguage(&HSTRING::from(tag(lang)))
        .and_then(|l| OcrEngine::IsLanguageSupported(&l))
        .unwrap_or(false)
}

fn engine(lang: &str) -> Result<OcrEngine> {
    let l = Language::CreateLanguage(&HSTRING::from(tag(lang)))?;
    OcrEngine::TryCreateFromLanguage(&l)
        .map_err(|e| anyhow!("no {} OCR on this PC: {e}", tag(lang)))
}

/// Lines of text in an RGBA image (rows top to bottom). Images wider than the engine's limit
/// are read in overlapping tiles; a line read twice is kept once.
pub fn read(rgba: &[u8], w: u32, h: u32, lang: &str) -> Result<Vec<Line>> {
    let engine = engine(lang)?;
    let max = OcrEngine::MaxImageDimension().unwrap_or(2600).max(400);
    if h > max {
        return Err(anyhow!("image too tall for OCR ({h} > {max})"));
    }
    let mut lines: Vec<Line> = Vec::new();
    for x0 in tiles(w, max) {
        let tw = w.min(max);
        let mut bgra = Vec::with_capacity((tw * h * 4) as usize);
        for row in 0..h {
            let start = ((row * w + x0) * 4) as usize;
            for p in rgba[start..start + (tw * 4) as usize].as_chunks::<4>().0 {
                bgra.extend_from_slice(&[p[2], p[1], p[0], 255]);
            }
        }
        let writer = DataWriter::new()?;
        writer.WriteBytes(&bgra)?;
        let bitmap = SoftwareBitmap::CreateCopyFromBuffer(
            &writer.DetachBuffer()?,
            BitmapPixelFormat::Bgra8,
            tw as i32,
            h as i32,
        )?;
        let result = engine.RecognizeAsync(&bitmap)?.join()?;
        for line in result.Lines()? {
            let (mut x1, mut y1, mut x2, mut y2) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
            let mut words = Vec::new();
            for word in line.Words()? {
                let r = word.BoundingRect()?;
                x1 = x1.min(r.X);
                y1 = y1.min(r.Y);
                x2 = x2.max(r.X + r.Width);
                y2 = y2.max(r.Y + r.Height);
                words.push(Word {
                    text: word.Text()?.to_string(),
                    x: r.X + x0 as f32,
                    y: r.Y,
                    w: r.Width,
                    h: r.Height,
                });
            }
            if x1 > x2 {
                continue;
            }
            let found = Line {
                text: line.Text()?.to_string(),
                x: x1 + x0 as f32,
                y: y1,
                w: x2 - x1,
                h: y2 - y1,
                words,
            };
            let dup = lines.iter().any(|l| {
                l.text == found.text && (l.x - found.x).abs() < 8.0 && (l.y - found.y).abs() < 8.0
            });
            if !dup {
                lines.push(found);
            }
        }
    }
    Ok(lines)
}

/// Left edges of tiles at most `max` wide covering `w`, overlapping by a quarter tile.
fn tiles(w: u32, max: u32) -> Vec<u32> {
    if w <= max {
        return vec![0];
    }
    let step = max * 3 / 4;
    let mut out: Vec<u32> = (0..)
        .map(|i| i * step)
        .take_while(|x| x + max < w)
        .collect();
    out.push(w - max);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tiles_cover_wide_images() {
        assert_eq!(tiles(1920, 2600), [0]);
        assert_eq!(tiles(2752, 2600), [0, 152]);
        assert_eq!(tiles(5120, 2600), [0, 1950, 2520]);
    }
}
