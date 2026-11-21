//! Text recognition with the engine built into the system, for reading the augment cards:
//! Windows.Media.Ocr on Windows, Vision on macOS. Runs on the calling thread and blocks; call it
//! from a blocking task.

#[cfg(target_os = "macos")]
mod macos;
#[cfg(windows)]
mod windows;

#[cfg(target_os = "macos")]
pub use macos::{available, read};
#[cfg(windows)]
pub use windows::{available, read};

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

impl Line {
    /// The line with its positions multiplied by `f` (pixels of a capture to screen units).
    pub fn scaled(mut self, f: f32) -> Line {
        (self.x, self.y, self.w, self.h) = (self.x * f, self.y * f, self.w * f, self.h * f);
        for w in &mut self.words {
            (w.x, w.y, w.w, w.h) = (w.x * f, w.y * f, w.w * f, w.h * f);
        }
        self
    }
}

/// Left edges of tiles at most `max` wide covering `w`, overlapping by a quarter tile.
pub(crate) fn tiles(w: u32, max: u32) -> Vec<u32> {
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
