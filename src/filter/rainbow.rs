use super::Filter;
use hsl::HSL;
use image::{Pixel, Rgba};

pub struct Rainbow {
    alpha: u8,
    speed: usize,
    frame: usize,
}

impl Rainbow {
    pub fn new(alpha: u8, speed: usize) -> Self {
        Self {
            alpha,
            speed,
            frame: 0,
        }
    }
}

impl Filter for Rainbow {
    fn transform_buffer(
        &mut self,
        buffer: &mut Vec<crate::Pixel>,
        _restore: &mut Option<Vec<crate::Pixel>>,
    ) {
        let hue = (self.frame * self.speed) % 360;
        let mask = HSL {
            h: hue as f64,
            s: 1.0,
            l: 0.5,
        }
        .to_rgb();

        for px in buffer {
            px.value
                .blend(&Rgba::from([mask.0, mask.1, mask.2, self.alpha]));
        }

        self.frame += 1;
    }
}
