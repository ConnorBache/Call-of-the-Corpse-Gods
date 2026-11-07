use std::{fs,path::Path};
mod render {pub struct Picture {pub w:usize,pub h:usize,pub rgba:Vec<u8>}}
use render::Picture;
pub fn creature_picture(bytes: &[u8]) -> crate::render::Picture {
    let mut decoder =
        image_webp::WebPDecoder::new(std::io::Cursor::new(bytes)).expect("authored WebP");
    decoder.set_memory_limit(32 * 1024 * 1024);
    let (w, h) = decoder.dimensions();
    let mut pixels = vec![
        0;
        decoder
            .output_buffer_size()
            .expect("bounded authored image")
    ];
    decoder
        .read_image(&mut pixels)
        .expect("authored WebP pixels");
    let rgba = if decoder.has_alpha() {
        pixels
    } else {
        let mut rgba = Vec::with_capacity(w as usize * h as usize * 4);
        for p in pixels.chunks_exact(3) {
            rgba.extend_from_slice(&[p[0], p[1], p[2], 255]);
        }
        rgba
    };
    crate::render::Picture {
        w: w as usize,
        h: h as usize,
        rgba,
    }
}
fn page_cutout(mut art: Picture) -> Picture {
    let count = art.w * art.h;
    if count == 0 {
        return art;
    }
    // An authored alpha channel is the mask. Re-keying it destroys dark outlines.
    if art.rgba.chunks_exact(4).any(|p| p[3] < 255) {
        return art;
    }
    // Learn the matte from the perimeter, not from the brightness of the subject.
    // A bounded histogram avoids sorting and rejects rare edge outliers.
    let mut hist = [[0usize; 256]; 3];
    let mut samples = 0usize;
    for y in 0..art.h {
        for x in 0..art.w {
            if x != 0 && y != 0 && x + 1 != art.w && y + 1 != art.h {
                continue;
            }
            samples += 1;
            for c in 0..3 {
                hist[c][art.rgba[(y * art.w + x) * 4 + c] as usize] += 1;
            }
        }
    }
    let mut low = [0u8; 3];
    let mut high = [0u8; 3];
    for c in 0..3 {
        let quantile = |rank: usize| {
            let mut sum = 0;
            for v in 0..256 {
                sum += hist[c][v];
                if sum > rank {
                    return v as u8;
                }
            }
            255u8
        };
        low[c] = quantile(samples / 1000).saturating_sub(3);
        high[c] = quantile(samples - 1 - samples / 1000).saturating_add(3);
    }
    let mut visited = vec![false; count];
    let mut pending = Vec::new();
    for x in 0..art.w {
        pending.push(x);
        pending.push((art.h - 1) * art.w + x);
    }
    for y in 0..art.h {
        pending.push(y * art.w);
        pending.push(y * art.w + art.w - 1);
    }
    while let Some(i) = pending.pop() {
        if visited[i] {
            continue;
        }
        visited[i] = true;
        let px = &mut art.rgba[i * 4..i * 4 + 4];
        if px[3] != 0 && (0..3).any(|c| px[c] < low[c] || px[c] > high[c]) {
            continue;
        }
        px[3] = 0;
        let (x, y) = (i % art.w, i / art.w);
        if x > 0 {
            pending.push(i - 1);
        }
        if x + 1 < art.w {
            pending.push(i + 1);
        }
        if y > 0 {
            pending.push(i - art.w);
        }
        if y + 1 < art.h {
            pending.push(i + art.w);
        }
    }
    art
}

fn main() {
 let args:Vec<_>=std::env::args().collect();let source=Path::new(&args[1]);let out=Path::new(&args[2]);fs::create_dir_all(out).unwrap();
 let mut files:Vec<_>=fs::read_dir(source).unwrap().map(|x|x.unwrap().path()).filter(|x|x.extension().unwrap()=="webp").collect();files.sort();
 for path in files {
  let bytes=fs::read(&path).unwrap();let rgba=page_cutout(creature_picture(&bytes));
  let dst=out.join(path.file_stem().unwrap()).with_extension("png");
  let file=fs::File::create(&dst).unwrap();let mut encoder=png::Encoder::new(file,rgba.w as u32,rgba.h as u32);encoder.set_color(png::ColorType::Rgba);encoder.set_depth(png::BitDepth::Eight);encoder.write_header().unwrap().write_image_data(&rgba.rgba).unwrap();
  println!("{} {}x{}",dst.display(),rgba.w,rgba.h);
 }
}
