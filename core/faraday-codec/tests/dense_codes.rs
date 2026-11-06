//! A dense code (version 23, the highest error correction: 109 modules a
//! side) in a high-resolution camera frame reads at the module sizes a
//! camera gives, however the frame is lit and blurred. The frames are
//! drawn here from made-up bytes: the code, its blur, a lighting gradient,
//! a slight roll and perspective, and sensor noise.

use faraday_codec::decode_luma;
use faraday_codec::qr::{self, Ecc, Payload, QrMatrix};
use faraday_crypto::work::Compute;

/// A small deterministic generator: the frames are the same on every run.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> f32 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 40) as f32) / (1u64 << 24) as f32
    }

    fn gauss(&mut self) -> f32 {
        let sum: f32 = (0..6).map(|_| self.next()).sum();
        (sum - 3.0) * 1.414
    }
}

/// How a frame is drawn.
struct Shot {
    width: usize,
    height: usize,
    /// Pixels a module covers at the code's centre.
    module: f32,
    /// Gaussian blur, in pixels.
    sigma: f32,
    /// Change of module size per pixel across the frame (a code seen
    /// at a slight angle).
    perspective: f32,
    /// Brightness change from one side of the frame to the other.
    gradient: f32,
    seed: u64,
}

/// The code on white, dark `25` and light `235`, blurred, lit and noisy.
fn frame(code: &QrMatrix, shot: &Shot) -> Vec<u8> {
    let (side, _, modules) = code.to_luma(1, 0);
    let side_f = side as f32;
    let (w, h) = (shot.width, shot.height);
    let mut rng = Rng(shot.seed ^ 0x9e37_79b9_7f4a_7c15);
    let (dx, dy) = ((rng.next() - 0.5) * 20.0, (rng.next() - 0.5) * 20.0);
    let roll = ((rng.next() - 0.5) * 6.0).to_radians();
    let (sin, cos) = (roll.sin(), roll.cos());
    let mut img = vec![0f32; w * h];
    for y in 0..h {
        for x in 0..w {
            let mut sum = 0.0;
            for sy in 0..2 {
                for sx in 0..2 {
                    let u0 = x as f32 + (sx as f32 + 0.5) / 2.0 - w as f32 / 2.0 - dx;
                    let v0 = y as f32 + (sy as f32 + 0.5) / 2.0 - h as f32 / 2.0 - dy;
                    let (u, v) = (u0 * cos + v0 * sin, -u0 * sin + v0 * cos);
                    let scale = shot.module * (1.0 + shot.perspective * u).max(0.2);
                    let (mx, my) = (u / scale + side_f / 2.0, v / scale + side_f / 2.0);
                    let inside = mx >= 0.0 && my >= 0.0 && mx < side_f && my < side_f;
                    sum += if inside && modules[my as usize * side + mx as usize] < 128 {
                        25.0
                    } else {
                        235.0
                    };
                }
            }
            let light = 1.0
                + shot.gradient * (x as f32 / w as f32 - 0.5)
                + shot.gradient * 0.5 * (y as f32 / h as f32 - 0.5);
            img[y * w + x] = sum / 4.0 * light;
        }
    }
    blur(&mut img, w, h, shot.sigma);
    let mut noise = Rng(shot.seed);
    img.iter()
        .map(|v| (v + 2.0 * noise.gauss()).clamp(0.0, 255.0) as u8)
        .collect()
}

/// A separable Gaussian blur, edges repeated.
fn blur(img: &mut [f32], w: usize, h: usize, sigma: f32) {
    let r = (sigma * 3.0).ceil() as i32;
    let kernel: Vec<f32> = (-r..=r)
        .map(|i| (-((i * i) as f32) / (2.0 * sigma * sigma)).exp())
        .collect();
    let total: f32 = kernel.iter().sum();
    let at = |i: i32, n: usize| i.clamp(0, n as i32 - 1) as usize;
    let mut across = img.to_vec();
    for y in 0..h {
        for x in 0..w {
            let sum: f32 = kernel
                .iter()
                .enumerate()
                .map(|(i, k)| k * img[y * w + at(x as i32 + i as i32 - r, w)])
                .sum();
            across[y * w + x] = sum / total;
        }
    }
    for y in 0..h {
        for x in 0..w {
            let sum: f32 = kernel
                .iter()
                .enumerate()
                .map(|(i, k)| k * across[at(y as i32 + i as i32 - r, h) * w + x])
                .sum();
            img[y * w + x] = sum / total;
        }
    }
}

#[test]
fn a_dense_code_reads_at_the_module_sizes_a_camera_gives() {
    let c = Compute::unchecked();
    let mut rng = Rng(99);
    let payload: Vec<u8> = (0..qr::byte_capacity(23, Ecc::High))
        .map(|_| (rng.next() * 256.0) as u8)
        .collect();
    let code = qr::encode(&c, Payload::Bytes(&payload), Ecc::High).unwrap();
    assert_eq!(code.size(), 109);

    // Module size, blur and slight perspective, at full HD.
    let shots = [
        (6.0, 0.7, 0.0),
        (6.0, 1.0, 0.0),
        (7.0, 0.7, 0.0),
        (8.0, 0.7, 0.0),
        (8.0, 1.0, 0.0),
        (9.0, 0.7, 0.00005),
        (9.0, 1.0, 0.0),
    ];

    let mut missed = Vec::new();
    for (i, (module, sigma, perspective)) in shots.into_iter().enumerate() {
        let shot = Shot {
            width: 1920,
            height: 1080,
            module,
            sigma,
            perspective,
            gradient: 0.4,
            seed: 11 + i as u64,
        };
        let pixels = frame(&code, &shot);
        let found = decode_luma(&c, shot.width, shot.height, &pixels);
        if !found.iter().any(|d| d.bytes.as_slice() == payload) {
            missed.push((module, sigma, perspective));
        }
    }
    assert!(missed.is_empty(), "not read: {missed:?}");
}
