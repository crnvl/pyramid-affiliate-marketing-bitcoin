use image::Rgba;
use rand::{random, rngs::StdRng, Rng, SeedableRng};

use crate::{
    edges::{Edge, Edges},
    Config, Pixel, RESTORE_DEBUG_COLOR,
};

use super::Filter;

const PRESET: [i32; 10] = [-3, -2, -1, 0, 0, 0, 0, 1, 2, 3];

pub struct Glitch {
    factor: i32,
    screen_x: u32,
    seed: u64,
    index: u64,
}

impl Glitch {
    pub fn new(config: &Config, factor: i32) -> Self {
        Self {
            factor,
            screen_x: config.canvas_size.0,
            seed: random(),
            index: 0,
        }
    }
}

impl Filter for Glitch {
    fn transform_buffer(
        &mut self,
        buffer: &mut Vec<crate::Pixel>,
        restore: &mut Option<Vec<crate::Pixel>>,
    ) {
        if self.index % 4 == 0 {
            self.seed += 1;
        }
        self.index += 1;
        let mut rng = StdRng::seed_from_u64(self.seed);

        let mut last_y = 0;
        let mut offset = PRESET[rng.random::<u8>() as usize % PRESET.len()] * self.factor;

        for px in buffer {
            if px.y > last_y {
                last_y = px.y;
                if rng.random_bool(1.0 / self.factor as f64) {
                    offset = PRESET[rng.random::<u8>() as usize % PRESET.len()] * self.factor;
                }
            }
            let val = px.x as i32 + offset;
            if val >= 0 && val < self.screen_x as i32 {
                px.x = val as u32;

                if let Some(restore) = restore {
                    if offset < 0 && px.edges.has_edge(Edge::Left) {
                        for i in 0..offset.abs() {
                            restore.push(Pixel {
                                x: px.x + i as u32,
                                y: px.y,
                                value: Rgba::from(RESTORE_DEBUG_COLOR),
                                edges: Edges::default(),
                            });
                        }
                    } else if offset > 0 && px.edges.has_edge(Edge::Right) {
                        for i in 0..offset {
                            restore.push(Pixel {
                                x: px.x - i as u32,
                                y: px.y,
                                value: Rgba::from(RESTORE_DEBUG_COLOR),
                                edges: Edges::default(),
                            });
                        }
                    }
                }
            }
        }
    }
}
