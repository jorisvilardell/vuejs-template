use anyhow::Result;
use image::{ImageBuffer, ImageFormat, Rgba};
use noise::{NoiseFn, Perlin, Seedable};
use serde::Serialize;
use std::io::Cursor;

#[derive(Clone, Copy)]
pub struct GenerateOptions {
    pub seed: u32,
    pub size: u32,
    pub scale: u32,
}

#[derive(Serialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum Biome {
    DeepWater,
    ShallowWater,
    Sand,
    Plains,
    Forest,
    Mountain,
    Snow,
}

impl Biome {
    fn rgba(self) -> [u8; 4] {
        match self {
            Biome::DeepWater    => [0x12, 0x3E, 0x7B, 0xFF],
            Biome::ShallowWater => [0x3E, 0x82, 0xC4, 0xFF],
            Biome::Sand         => [0xE8, 0xD4, 0x88, 0xFF],
            Biome::Plains       => [0x6F, 0xB3, 0x4E, 0xFF],
            Biome::Forest       => [0x2F, 0x6B, 0x33, 0xFF],
            Biome::Mountain     => [0x7A, 0x6E, 0x65, 0xFF],
            Biome::Snow         => [0xF7, 0xF7, 0xF7, 0xFF],
        }
    }

    fn key(self) -> &'static str {
        match self {
            Biome::DeepWater    => "deep_water",
            Biome::ShallowWater => "shallow_water",
            Biome::Sand         => "sand",
            Biome::Plains       => "plains",
            Biome::Forest       => "forest",
            Biome::Mountain     => "mountain",
            Biome::Snow         => "snow",
        }
    }
}

fn classify(h: f64) -> Biome {
    match h {
        v if v < 0.40 => Biome::DeepWater,
        v if v < 0.50 => Biome::ShallowWater,
        v if v < 0.55 => Biome::Sand,
        v if v < 0.70 => Biome::Plains,
        v if v < 0.78 => Biome::Forest,
        v if v < 0.90 => Biome::Mountain,
        _             => Biome::Snow,
    }
}

pub struct GenerateResult {
    pub png: Vec<u8>,
    pub json: serde_json::Value,
}

pub fn generate(opts: GenerateOptions) -> Result<GenerateResult> {
    let perlin = Perlin::new(0).set_seed(opts.seed);
    let n = opts.size as usize;

    let octaves = 4_u32;
    let lacunarity = 2.0_f64;
    let persistence = 0.5_f64;
    let base_freq = 4.0_f64 / opts.size as f64;

    let mut tiles: Vec<Vec<&'static str>> = vec![vec![""; n]; n];
    let mut img = ImageBuffer::<Rgba<u8>, _>::new(opts.size, opts.size);

    let cx = (n as f64) / 2.0;
    let cy = (n as f64) / 2.0;
    let max_dist = ((cx * cx) + (cy * cy)).sqrt();

    for (y, row) in tiles.iter_mut().enumerate() {
        for (x, cell) in row.iter_mut().enumerate() {
            let mut amp = 1.0;
            let mut freq = base_freq;
            let mut sum = 0.0;
            let mut norm = 0.0;
            for _ in 0..octaves {
                let v = perlin.get([x as f64 * freq, y as f64 * freq]);
                sum += v * amp;
                norm += amp;
                amp *= persistence;
                freq *= lacunarity;
            }
            let mut h = sum / norm;
            h = (h + 1.0) * 0.5;

            let dx = x as f64 - cx;
            let dy = y as f64 - cy;
            let d = ((dx * dx) + (dy * dy)).sqrt() / max_dist;
            let falloff = (1.0 - d.powf(2.2)).clamp(0.0, 1.0);
            h = (h * 0.85) + (falloff * 0.25);
            h = h.clamp(0.0, 1.0);

            let biome = classify(h);
            *cell = biome.key();
            img.put_pixel(x as u32, y as u32, Rgba(biome.rgba()));
        }
    }

    let target = opts.size * opts.scale;
    let upscaled = image::imageops::resize(&img, target, target, image::imageops::FilterType::Nearest);

    let mut buf = Cursor::new(Vec::new());
    upscaled.write_to(&mut buf, ImageFormat::Png)?;
    let png = buf.into_inner();

    let json = serde_json::json!({
        "size": opts.size,
        "scale": opts.scale,
        "seed": opts.seed,
        "biomes": ["deep_water","shallow_water","sand","plains","forest","mountain","snow"],
        "tiles": tiles,
    });

    Ok(GenerateResult { png, json })
}
