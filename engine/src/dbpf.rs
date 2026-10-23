use crate::error::{EngineError, Result};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

pub const BUST_TYPES: [u32; 2] = [0x16CC_F748, 0x0000_0000];

const MAX_ENTRY: u32 = 4 * 1024 * 1024;
const MAX_ENTRIES: u32 = 200_000;

#[derive(Debug, Clone)]
pub struct Thumbnail {
    pub resource_type: u32,
    pub group: u32,
    pub instance: u64,
    pub data: Vec<u8>,
}

fn u32_at(buf: &[u8], at: usize) -> Result<u32> {
    buf.get(at..at + 4)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .ok_or_else(|| EngineError::Invalid("dbpf truncated".into()))
}

fn is_jpeg(data: &[u8]) -> bool {
    data.len() > 4 && data[0] == 0xFF && data[1] == 0xD8 && data[2] == 0xFF
}

pub fn read_thumbnails(path: &Path, types: &[u32]) -> Result<Vec<Thumbnail>> {
    let mut file = File::open(path)?;
    let total = file.metadata()?.len();
    let mut head = [0u8; 96];
    file.read_exact(&mut head)?;
    if &head[0..4] != b"DBPF" || u32_at(&head, 4)? != 2 {
        return Err(EngineError::Invalid("not a dbpf package".into()));
    }
    let count = u32_at(&head, 36)?;
    let size = u32_at(&head, 44)? as u64;
    let pos = u32_at(&head, 64)? as u64;
    if count == 0 || count > MAX_ENTRIES || pos == 0 || pos.saturating_add(size) > total {
        return Ok(Vec::new());
    }
    file.seek(SeekFrom::Start(pos))?;
    let mut index = vec![0u8; size as usize];
    file.read_exact(&mut index)?;
    let flags = u32_at(&index, 0)?;
    let mut cursor = 4usize;
    let mut constants = [0u32; 3];
    for (bit, slot) in constants.iter_mut().enumerate() {
        if flags & (1 << bit) != 0 {
            *slot = u32_at(&index, cursor)?;
            cursor += 4;
        }
    }
    let mut out = Vec::new();
    for _ in 0..count {
        let mut fields = [0u32; 3];
        for (bit, slot) in fields.iter_mut().enumerate() {
            if flags & (1 << bit) != 0 {
                *slot = constants[bit];
            } else {
                *slot = u32_at(&index, cursor)?;
                cursor += 4;
            }
        }
        let inst_lo = u32_at(&index, cursor)?;
        let offset = u32_at(&index, cursor + 4)? as u64;
        let length = u32_at(&index, cursor + 8)? & 0x7FFF_FFFF;
        let compression = u32_at(&index, cursor + 16)? & 0xFFFF;
        cursor += 20;
        if !types.contains(&fields[0]) || compression != 0 || length == 0 || length > MAX_ENTRY || offset + length as u64 > total {
            continue;
        }
        file.seek(SeekFrom::Start(offset))?;
        let mut data = vec![0u8; length as usize];
        file.read_exact(&mut data)?;
        if is_jpeg(&data) {
            out.push(Thumbnail { resource_type: fields[0], group: fields[1], instance: ((fields[2] as u64) << 32) | inst_lo as u64, data });
        }
    }
    Ok(out)
}

fn embedded_alpha(data: &[u8]) -> Option<&[u8]> {
    let mut at = 2usize;
    while at + 4 <= data.len() && data[at] == 0xFF {
        let marker = data[at + 1];
        if marker == 0xDA || marker == 0xD9 {
            return None;
        }
        let length = u16::from_be_bytes([data[at + 2], data[at + 3]]) as usize;
        let payload = data.get(at + 4..at + 2 + length)?;
        if marker == 0xE0 && payload.len() > 8 && &payload[0..4] == b"ALFA" {
            let size = u32::from_be_bytes([payload[4], payload[5], payload[6], payload[7]]) as usize;
            return payload.get(8..8 + size);
        }
        at += 2 + length;
    }
    None
}

pub fn thumbnail_to_png(data: &[u8]) -> Result<Vec<u8>> {
    use image::{imageops, DynamicImage, ImageFormat, RgbaImage};
    let invalid = |m: &str| EngineError::Invalid(m.into());
    let rgb = image::load_from_memory_with_format(data, ImageFormat::Jpeg)
        .map_err(|_| invalid("thumbnail not decodable"))?
        .to_rgb8();
    let (width, height) = rgb.dimensions();
    let alpha = embedded_alpha(data)
        .and_then(|png| image::load_from_memory_with_format(png, ImageFormat::Png).ok())
        .map(|img| {
            let luma = img.to_luma8();
            if luma.dimensions() == (width, height) {
                luma
            } else {
                imageops::resize(&luma, width, height, imageops::FilterType::Triangle)
            }
        });
    let mut out = RgbaImage::new(width, height);
    for (x, y, pixel) in rgb.enumerate_pixels() {
        let a = alpha.as_ref().map(|m| m.get_pixel(x, y)[0]).unwrap_or(255);
        out.put_pixel(x, y, image::Rgba([pixel[0], pixel[1], pixel[2], a]));
    }
    let mut buffer = Vec::new();
    DynamicImage::ImageRgba8(out)
        .write_to(&mut std::io::Cursor::new(&mut buffer), ImageFormat::Png)
        .map_err(|_| invalid("thumbnail not encodable"))?;
    Ok(buffer)
}
