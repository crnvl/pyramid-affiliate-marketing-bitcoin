use std::ops::Range;

use image::Rgba;
use rand::random_range;

use super::Filter;
use crate::{
    edges::{Edge, Edges},
    Area, Config, Pixel,
};

const VEC_RANGE: Range<i8> = 0..4;

pub struct Bounce {
    base_x: i32,
    base_y: i32,

    vec_x: i8,
    vec_y: i8,

    screen_x: u32,
    screen_y: u32,
    area: Area,

    speed: i8,
}

impl Bounce {
    pub fn new(config: &Config, speed: i8) -> Self {
        Self {
            base_x: 0,
            base_y: 0,
            vec_x: random_range(VEC_RANGE) + speed,
            vec_y: random_range(VEC_RANGE) + speed,
            screen_x: config.canvas_size.0,
            screen_y: config.canvas_size.1,
            area: config.image_area.clone(),
            speed,
        }
    }
}

impl Filter for Bounce {
    fn transform_buffer(
        &mut self,
        buffer: &mut Vec<crate::Pixel>,
        restore: &mut Option<Vec<crate::Pixel>>,
    ) {
        let (mut change_x, mut change_y) = (false, false);

        self.base_x += self.vec_x as i32;
        self.base_y += self.vec_y as i32;

        if (self.base_x + self.area.origin_x as i32) < 0 {
            self.base_x = -(self.area.origin_x as i32); // sums up to 0
            change_x = true;
        }
        if (self.base_x + self.area.size_x as i32) >= self.screen_x as i32 {
            self.base_x = (self.screen_x - self.area.size_x) as i32;
            change_x = true;
        }

        if (self.base_y + self.area.origin_y as i32) < 0 {
            self.base_y = -(self.area.origin_y as i32); // sums up to 0
            change_y = true;
        }
        if (self.base_y + self.area.size_y as i32) >= self.screen_y as i32 {
            self.base_y = (self.screen_y - self.area.size_y) as i32;
            change_y = true;
        }

        for px in buffer {
            px.x = (px.x as i32 + self.base_x) as u32;
            px.y = (px.y as i32 + self.base_y) as u32;

            if let Some(restore) = restore {
                let size = VEC_RANGE.end + self.speed;

                if (self.vec_x < 0 || change_x) && px.edges.has_edge(Edge::Right) {
                    for i in 0..size {
                        restore.push(Pixel {
                            x: px.x - i as u32,
                            y: px.y,
                            value: Rgba::from([0, 0, 0, 255]),
                            edges: Edges::default(),
                        });
                    }
                } else if (self.vec_x > 0 || change_x) && px.edges.has_edge(Edge::Left) {
                    for i in 0..size {
                        restore.push(Pixel {
                            x: px.x + i as u32,
                            y: px.y,
                            value: Rgba::from([0, 0, 0, 255]),
                            edges: Edges::default(),
                        });
                    }
                }
                if (self.vec_y < 0 || change_y) && px.edges.has_edge(Edge::Bottom) {
                    for i in 0..size {
                        restore.push(Pixel {
                            x: px.x,
                            y: px.y - i as u32,
                            value: Rgba::from([0, 0, 0, 255]),
                            edges: Edges::default(),
                        });
                    }
                } else if (self.vec_y > 0 || change_y) && px.edges.has_edge(Edge::Top) {
                    for i in 0..size {
                        restore.push(Pixel {
                            x: px.x,
                            y: px.y + i as u32,
                            value: Rgba::from([0, 0, 0, 255]),
                            edges: Edges::default(),
                        });
                    }
                }
            }
        }

        if change_x && change_y {
            self.vec_x = change_direction(self.vec_x, self.speed, true);
            self.vec_y = change_direction(self.vec_y, self.speed, true);
        } else if change_x {
            self.vec_x = change_direction(self.vec_x, self.speed, true);
            self.vec_y = change_direction(self.vec_y, self.speed, false);
        } else if change_y {
            self.vec_y = change_direction(self.vec_y, self.speed, true);
            self.vec_x = change_direction(self.vec_x, self.speed, false);
        }
    }
}

fn change_direction(direction: i8, speed: i8, invert: bool) -> i8 {
    let mut x = random_range(VEC_RANGE) + speed;

    if !invert {
        x = -x;
    }

    if direction > 0 {
        -x
    } else {
        x
    }
}
