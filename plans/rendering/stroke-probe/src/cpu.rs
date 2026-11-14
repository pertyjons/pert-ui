//! The CPU side of the prototype: the reference evaluated tile by tile and
//! blended once per pixel into a premultiplied linear frame.
use crate::stroke::{Prepared, coverage, segment_distance};

pub struct Canvas {
    pub width: u32,
    pub pixels: Vec<[f32; 4]>,
}

impl Canvas {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            pixels: vec![[0.0; 4]; (width * height) as usize],
        }
    }
    pub fn clear(&mut self, color: [f32; 4]) {
        self.pixels.fill(color);
    }
    pub fn draw(&mut self, prepared: &Prepared) {
        for tile in &prepared.tiles {
            let header = &prepared.headers[tile.stroke as usize];
            let list = &prepared.lists[tile.start as usize..(tile.start + tile.count) as usize];
            for y in tile.rect[1]..tile.rect[3] {
                let py = (y - header.origin[1]) as f32 + 0.5;
                let row = (y * self.width) as usize;
                for x in tile.rect[0]..tile.rect[2] {
                    let p = [(x - header.origin[0]) as f32 + 0.5, py];
                    let mut distance = f32::INFINITY;
                    for &i in list {
                        distance = distance.min(segment_distance(
                            &prepared.segments[i as usize],
                            header.h,
                            p,
                        ));
                    }
                    let c = coverage(distance, header.thin);
                    if c > 0.0 {
                        let pixel = &mut self.pixels[row + x as usize];
                        let keep = 1.0 - header.color[3] * c;
                        for (value, color) in pixel.iter_mut().zip(header.color) {
                            *value = color * c + *value * keep;
                        }
                    }
                }
            }
        }
    }
}
