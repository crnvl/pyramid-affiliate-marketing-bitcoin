use image::{Pixel, Rgba};

use super::Filter;

pub struct Blend {
    color: Rgba<u8>,
}

impl Blend {
    pub fn new(color: Rgba<u8>) -> Self {
        Self { color }
    }
}

impl Filter for Blend {
    fn transform_buffer(
        &mut self,
        buffer: &mut Vec<crate::Pixel>,
        _restore: &mut Option<Vec<crate::Pixel>>,
    ) {
        for px in buffer {
            px.value.blend(&self.color);
        }
    }
}
