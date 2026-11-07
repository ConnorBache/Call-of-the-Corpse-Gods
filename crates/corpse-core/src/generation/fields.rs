//! Geology's shared correlated field; orchestration lives in geology.rs.
/// Exact Float32 lattice/storage boundaries with f64 interpolation, as JS Float32Array.
/// O(WH + ceil(W/λ)ceil(H/λ)) time and memory. No random calls after lattice fill.
pub fn value_field(
    width: usize,
    height: usize,
    wavelength: usize,
    r: &mut impl FnMut() -> f64,
) -> Result<Vec<f32>, String> {
    if width == 0 || height == 0 || wavelength == 0 || width > 60 && width != 79 || height > 60 {
        return Err("Unsupported geological field dimensions".into());
    }
    let cols = width.div_ceil(wavelength) + 1;
    let rows = height.div_ceil(wavelength) + 1;
    let lattice: Vec<f32> = (0..cols * rows).map(|_| r() as f32).collect();
    let mut result = vec![0.0; width * height];
    let smooth = |n: f64| n * n * (3.0 - 2.0 * n);
    for y in 0..height {
        for x in 0..width {
            let gx = x / wavelength;
            let gy = y / wavelength;
            let dx = smooth(x as f64 / wavelength as f64 - gx as f64);
            let dy = smooth(y as f64 / wavelength as f64 - gy as f64);
            let at = gy * cols + gx;
            let a = lattice[at] as f64 * (1.0 - dx) + lattice[at + 1] as f64 * dx;
            let b = lattice[at + cols] as f64 * (1.0 - dx) + lattice[at + cols + 1] as f64 * dx;
            result[y * width + x] = (a * (1.0 - dy) + b * dy) as f32;
        }
    }
    Ok(result)
}
