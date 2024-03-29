use std::iter::FromIterator;

use image::{Rgba, Rgba32FImage, RgbaImage};

use crate::{prelude::BoundingBox, prelude::Color, prelude::Point};

/// Point cloud encoder to generate images from point cloud data
pub struct Encoder {
    /// Normalized points in the range of 0.0 ~ 1.0 for x, y, z coordinates based on the bounding box
    normalized: Vec<Point>,
}

impl Encoder {
    pub fn new(points: &[Point], bbox: Option<BoundingBox>) -> Self {
        let bbox = bbox.unwrap_or(BoundingBox::from_iter(points.iter().map(|p| p.position)));
        let min = bbox.min();
        let size = bbox.size();
        let normalized: Vec<_> = points
            .iter()
            .map(|pt| {
                let p = pt.position - min;
                let normalized = p.component_div(&size);
                // x, y, z -> 0.0 ~ 1.0, 0.0 ~ 1.0, 0.0 ~ 1.0
                Point {
                    position: normalized.into(),
                    color: pt.color,
                }
            })
            .collect();

        Self { normalized }
    }

    /// Encode point cloud data to 8-bit image.
    /// The 1st image is for position and the 2nd image is for color.
    pub fn encode_8bit(&self) -> (RgbaImage, RgbaImage) {
        let n = self.normalized.len();
        let side = (n as f64).sqrt().ceil() as u32;

        let mut position = RgbaImage::new(side, side);
        let mut color = RgbaImage::new(side, side);
        self.normalized.iter().enumerate().for_each(|(idx, p)| {
            let y = idx as u32 / side;
            let x = idx as u32 % side;
            let pos = p.position;

            let ix = (pos.x * (u8::MAX as f64)).floor() as u8;
            let iy = (pos.y * (u8::MAX as f64)).floor() as u8;
            let iz = (pos.z * (u8::MAX as f64)).floor() as u8;
            let c = p.color.unwrap_or(Color::white());

            position.put_pixel(x, y, Rgba([ix, iy, iz, u8::MAX]));
            color.put_pixel(x, y, Rgba([c.r(), c.g(), c.b(), u8::MAX]));
        });

        (position, color)
    }

    /// Encode point cloud data to 8-bit quad image.
    /// f32 value divided into 4 channels each with 8-bit.
    /// 1st quadrant: lowest 8-bit of x, y, z, and alpha channel has color.r() (if use_alpha_channel_as_color is true)
    /// 2nd quadrant: 2nd 8-bit of x, y, z, and alpha channel has color.g() (if use_alpha_channel_as_color is true)
    /// 3rd quadrant: 3rd 8-bit of x, y, z, and alpha channel has color.b() (if use_alpha_channel_as_color is true)
    /// 4th quadrant: highest 8-bit of x, y, z, and alpha channel has 255
    pub fn encode_8bit_quad(&self, use_alpha_channel_as_color: bool) -> RgbaImage {
        let n = self.normalized.len();
        let side = (n as f64).sqrt().ceil() as u32;
        let mut img8u = RgbaImage::new(side * 2, side * 2);
        self.normalized.iter().enumerate().for_each(|(idx, p)| {
            let y = idx as u32 / side;
            let x = idx as u32 % side;
            let pos = p.position;

            // convert f64 to f32, and split into 4 channels each with 8-bit
            let ix = encode_8bit_4channels(pos.x);
            let iy = encode_8bit_4channels(pos.y);
            let iz = encode_8bit_4channels(pos.z);

            if use_alpha_channel_as_color {
                let color = p.color.unwrap_or(Color::white());
                img8u.put_pixel(x, y, Rgba([ix.0, iy.0, iz.0, color.r()]));
                img8u.put_pixel(x + side, y, Rgba([ix.1, iy.1, iz.1, color.g()]));
                img8u.put_pixel(x, y + side, Rgba([ix.2, iy.2, iz.2, color.b()]));
            } else {
                img8u.put_pixel(x, y, Rgba([ix.0, iy.0, iz.0, u8::MAX]));
                img8u.put_pixel(x + side, y, Rgba([ix.1, iy.1, iz.1, u8::MAX]));
                img8u.put_pixel(x, y + side, Rgba([ix.2, iy.2, iz.2, u8::MAX]));
            }
            img8u.put_pixel(x + side, y + side, Rgba([ix.3, iy.3, iz.3, u8::MAX]));
        });

        img8u
    }

    /// Encode point cloud data to 32-bit image.
    /// The 1st image is for position and the 2nd image is for color.
    pub fn encode_32bit(&self) -> (Rgba32FImage, RgbaImage) {
        let n = self.normalized.len();
        let side = (n as f64).sqrt().ceil() as u32;

        let mut position = Rgba32FImage::new(side, side);
        let mut color = RgbaImage::new(side, side);

        self.normalized.iter().enumerate().for_each(|(idx, p)| {
            let y = idx as u32 / side;
            let x = idx as u32 % side;
            let pos = p.position;
            let cast = pos.cast::<f32>();
            position.put_pixel(x, y, Rgba([cast.x, cast.y, cast.z, 1.0]));

            let c = p.color.unwrap_or(Color::white());
            color.put_pixel(x, y, Rgba([c.r(), c.g(), c.b(), u8::MAX]));
        });

        (position, color)
    }
}

/// Convert f64 to 4 u8 channels
fn encode_8bit_4channels(v01: f64) -> (u8, u8, u8, u8) {
    let iu = (v01 * (u32::MAX as f64)).floor() as u32;
    let p3 = ((iu >> 24) & 0xff) as u8;
    let p2 = ((iu >> 16) & 0xff) as u8;
    let p1 = ((iu >> 8) & 0xff) as u8;
    let p0 = (iu & 0xff) as u8;
    (p0, p1, p2, p3)
}
