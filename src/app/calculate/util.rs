use crate::app::calculate::ProgressMsg;

use serde::Deserialize;
use serde::Serialize;
use uuid::Uuid;

use std::error::Error;

// pub(crate) fn save_result(
//     target: image::SourceImg,
//     base_name: String,
//     source: image::SourceImg,
//     assignments: Vec<usize>,
//     img: image::SourceImg,
// ) -> Result<String, Box<dyn Error>> {
//     let mut dir_name = base_name.clone();
//     let mut counter = 1;
//     while std::path::Path::new(&format!("./presets/{}", dir_name)).exists() {
//         dir_name = format!("{}_{}", base_name, counter);
//         counter += 1;
//     }
//     std::fs::create_dir_all(format!("./presets/{}", dir_name))?;
//     img.save(format!("./presets/{}/output.png", dir_name))?;
//     source.save(format!("./presets/{}/source.png", dir_name))?;
//     target.save(format!("./presets/{}/target.png", dir_name))?;
//     std::fs::write(
//         format!("./presets/{}/assignments.json", dir_name),
//         serialize_assignments(assignments),
//     )?;
//     Ok(dir_name)
// }

pub trait ProgressSink {
    fn send(&mut self, msg: ProgressMsg);
}
// Native-friendly adapter
impl ProgressSink for std::sync::mpsc::SyncSender<ProgressMsg> {
    fn send(&mut self, msg: ProgressMsg) {
        let _ = std::sync::mpsc::SyncSender::send(self, msg);
    }
}

// Allow using closures as progress sinks in WASM
impl<T> ProgressSink for T
where
    T: FnMut(crate::app::calculate::ProgressMsg),
{
    fn send(&mut self, msg: crate::app::calculate::ProgressMsg) {
        self(msg);
    }
}

#[allow(clippy::type_complexity)]
pub(crate) fn get_images(
    source: SourceImg,
    settings: &GenerationSettings,
) -> Result<(Vec<(u8, u8, u8)>, Vec<(u8, u8, u8)>, Vec<i64>), Box<dyn Error>> {
    let source = settings.source_crop_scale.apply(&source, settings.sidelen);
    let source_pixels = source
        .pixels()
        .map(|p| (p[0], p[1], p[2]))
        .collect::<Vec<_>>();

    let (target, weights) = settings.get_target()?;
    let target_pixels = target
        .pixels()
        .map(|p| (p[0], p[1], p[2]))
        .collect::<Vec<_>>();
    assert_eq!(source_pixels.len(), target_pixels.len());
    Ok((source_pixels, target_pixels, weights))
}

fn default_rotation() -> f32 {
    0.0
}

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct CropScale {
    pub x: f32,     // -1..1: pan left/right
    pub y: f32,     // -1..1: pan up/down
    pub scale: f32, // >1: zoom in, <1: zoom out (image smaller than frame, gaps filled)
    #[serde(default = "default_rotation")]
    pub rotation: f32, // degrees, CCW
}

impl CropScale {
    pub fn identity() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            scale: 1.0,
            rotation: 0.0,
        }
    }

    pub fn apply(&self, img: &SourceImg, sidelen: u32) -> SourceImg {
        let (w, h) = img.dimensions();
        let base_side = w.min(h) as f32;

        let avg = {
            let n = (w * h) as u64;
            let (r, g, b) = img.pixels().fold((0u64, 0u64, 0u64), |(r, g, b), p| {
                (r + p[0] as u64, g + p[1] as u64, b + p[2] as u64)
            });
            image::Rgb([(r / n) as u8, (g / n) as u8, (b / n) as u8])
        };

        let effective_scale = self.scale.max(0.05);
        // source pixels per output pixel
        let pix_per_out = base_side / (effective_scale * sidelen as f32);

        let xn = (self.x.clamp(-1.0, 1.0) + 1.0) * 0.5;
        let yn = (self.y.clamp(-1.0, 1.0) + 1.0) * 0.5;

        // Center of the source region we want to display, and where in the output it sits
        let (center_out_x, center_out_y, src_cx, src_cy) = if effective_scale >= 1.0 {
            // Zoom in: pan within source
            let crop_side = base_side / effective_scale;
            let max_x = (w as f32 - crop_side).max(0.0);
            let max_y = (h as f32 - crop_side).max(0.0);
            let x0 = (xn * max_x).floor();
            let y0 = (yn * max_y).floor();
            (
                sidelen as f32 / 2.0,
                sidelen as f32 / 2.0,
                x0 + crop_side / 2.0,
                y0 + crop_side / 2.0,
            )
        } else {
            // Zoom out: image is smaller than output; pan its placement
            let max_pan = sidelen as f32 * (1.0 - effective_scale) / 2.0;
            let out_cx = sidelen as f32 / 2.0 + self.x.clamp(-1.0, 1.0) * max_pan;
            let out_cy = sidelen as f32 / 2.0 + self.y.clamp(-1.0, 1.0) * max_pan;
            (out_cx, out_cy, w as f32 / 2.0, h as f32 / 2.0)
        };

        let theta = self.rotation.to_radians();
        let cos_t = theta.cos();
        let sin_t = theta.sin();

        let mut out = SourceImg::new(sidelen, sidelen);

        for oy in 0..sidelen {
            for ox in 0..sidelen {
                // Vector from image center in output space
                let cx = ox as f32 + 0.5 - center_out_x;
                let cy = oy as f32 + 0.5 - center_out_y;

                // Inverse-rotate to align with (unrotated) source axes
                let rx = cx * cos_t + cy * sin_t;
                let ry = -cx * sin_t + cy * cos_t;

                // Map to source pixel coordinates
                let sx = rx * pix_per_out + src_cx;
                let sy = ry * pix_per_out + src_cy;

                let pixel = if sx >= 0.0 && sy >= 0.0 && sx < w as f32 && sy < h as f32 {
                    // Bilinear interpolation
                    let x0 = sx.floor() as u32;
                    let y0 = sy.floor() as u32;
                    let x1 = (x0 + 1).min(w - 1);
                    let y1 = (y0 + 1).min(h - 1);
                    let fx = sx - sx.floor();
                    let fy = sy - sy.floor();
                    let p00 = img.get_pixel(x0, y0);
                    let p10 = img.get_pixel(x1, y0);
                    let p01 = img.get_pixel(x0, y1);
                    let p11 = img.get_pixel(x1, y1);
                    let lerp = |a: u8, b: u8, c: u8, d: u8| {
                        (a as f32 * (1.0 - fx) * (1.0 - fy)
                            + b as f32 * fx * (1.0 - fy)
                            + c as f32 * (1.0 - fx) * fy
                            + d as f32 * fx * fy)
                            .round() as u8
                    };
                    image::Rgb([
                        lerp(p00[0], p10[0], p01[0], p11[0]),
                        lerp(p00[1], p10[1], p01[1], p11[1]),
                        lerp(p00[2], p10[2], p01[2], p11[2]),
                    ])
                } else {
                    avg
                };

                out.put_pixel(ox, oy, pixel);
            }
        }

        out
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum Algorithm {
    Optimal,
    Genetic,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct GenerationSettings {
    pub id: Uuid,
    pub name: String,

    pub proximity_importance: i64,
    pub algorithm: Algorithm,

    pub sidelen: u32,
    custom_target: Option<(u32, u32, Vec<u8>)>,
    pub target_crop_scale: CropScale,
    pub source_crop_scale: CropScale,
}

pub type SourceImg = image::RgbImage;

impl GenerationSettings {
    pub fn default(id: Uuid, name: String) -> Self {
        Self {
            name,
            proximity_importance: 13, // 20
            algorithm: Algorithm::Genetic,
            id,
            sidelen: 128,
            custom_target: None,
            target_crop_scale: CropScale::identity(),
            source_crop_scale: CropScale::identity(),
        }
    }

    pub fn get_target(&self) -> Result<(SourceImg, Vec<i64>), Box<dyn std::error::Error>> {
        let target = self.get_raw_target();
        let target = self.target_crop_scale.apply(&target, self.sidelen);
        let weights = if self.custom_target.is_some() {
            vec![255; (self.sidelen * self.sidelen) as usize] // uniform weights
        } else {
            let target_weights =
                image::load_from_memory(include_bytes!("data/weights.png"))?.to_rgb8();
            let target_weights = self.target_crop_scale.apply(&target_weights, self.sidelen);
            load_weights(target_weights)
        };

        Ok((target, weights))
    }

    pub(crate) fn get_raw_target(&self) -> SourceImg {
        if let Some((w, h, data)) = &self.custom_target {
            image::ImageBuffer::from_vec(*w, *h, data.clone()).unwrap()
        } else {
            image::load_from_memory(include_bytes!("data/target.png"))
                .unwrap()
                .to_rgb8()
        }
    }

    pub(crate) fn set_raw_target(&mut self, img: SourceImg) {
        let (w, h) = img.dimensions();
        let data = img.into_raw();
        self.custom_target = Some((w, h, data));
    }

    pub fn clone_with_new_id(&self) -> Self {
        let mut new = self.clone();
        new.id = Uuid::new_v4();

        new.name = if let Some(v_pos) = self.name.rfind(" v") {
            let potential_version = &self.name[v_pos + 2..];
            if let Ok(version) = potential_version.parse::<u32>() {
                let base_name = &self.name[..v_pos];
                format!("{} v{}", base_name, version + 1)
            } else {
                format!("{} v2", self.name)
            }
        } else {
            format!("{} v2", self.name)
        };

        new
    }
}

pub fn load_weights(source: SourceImg) -> Vec<i64> {
    let (width, height) = source.dimensions();
    let mut weights = vec![0; (width * height) as usize];
    for (x, y, pixel) in source.enumerate_pixels() {
        let weight = pixel[0] as i64;
        weights[(y * width + x) as usize] = weight;
    }
    weights
}
