use crate::shots::fnv;
use std::fs::{self, File};
use std::io::{Cursor, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

const THUMB_TYPE: u32 = 0x14;
const HEADER_LEN: usize = 96;
const MAX_INDEX_BYTES: u64 = 8 * 1024 * 1024;
const MAX_THUMB_BYTES: u32 = 4 * 1024 * 1024;

fn u32le(b: &[u8], o: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(o..o.checked_add(4)?)?.try_into().ok()?))
}

fn u16le(b: &[u8], o: usize) -> Option<u16> {
    Some(u16::from_le_bytes(b.get(o..o.checked_add(2)?)?.try_into().ok()?))
}

fn u64le(b: &[u8], o: usize) -> Option<u64> {
    Some(u64::from_le_bytes(b.get(o..o.checked_add(8)?)?.try_into().ok()?))
}

pub fn extract_thumbnail_jpeg(path: &Path) -> Option<Vec<u8>> {
    let mut file = File::open(path).ok()?;
    let len = file.metadata().ok()?.len();
    let mut head = [0u8; HEADER_LEN];
    file.read_exact(&mut head).ok()?;
    if &head[0..4] != b"DBPF" || u32le(&head, 4)? != 2 {
        return None;
    }
    let count = u32le(&head, 0x24)? as usize;
    let index_pos = u64le(&head, 0x40)?;
    if count == 0 || count > 100_000 || index_pos.checked_add(4)? > len {
        return None;
    }
    let want = ((count as u64) * 40 + 32).min(len - index_pos).min(MAX_INDEX_BYTES);
    file.seek(SeekFrom::Start(index_pos)).ok()?;
    let mut index = vec![0u8; want as usize];
    file.read_exact(&mut index).ok()?;

    let flags = u32le(&index, 0)?;
    let mut offset = 4;
    let mut constants = [0u32; 3];
    for (i, slot) in constants.iter_mut().enumerate() {
        if flags & (1 << i) != 0 {
            *slot = u32le(&index, offset)?;
            offset += 4;
        }
    }

    for _ in 0..count {
        let mut field = |constant: Option<u32>| -> Option<u32> {
            match constant {
                Some(v) => Some(v),
                None => {
                    let v = u32le(&index, offset)?;
                    offset += 4;
                    Some(v)
                }
            }
        };
        let kind = field((flags & 1 != 0).then_some(constants[0]))?;
        let _group = field((flags & 2 != 0).then_some(constants[1]))?;
        let _instance_hi = field((flags & 4 != 0).then_some(constants[2]))?;
        let _instance_lo = field(None)?;
        let position = field(None)?;
        let size = field(None)? & 0x7FFF_FFFF;
        let _memory = field(None)?;
        let compression = u16le(&index, offset)?;
        offset += 4;

        if kind == THUMB_TYPE && compression == 0 && size > 4 && size <= MAX_THUMB_BYTES {
            if u64::from(position) + u64::from(size) > len {
                return None;
            }
            file.seek(SeekFrom::Start(u64::from(position))).ok()?;
            let mut data = vec![0u8; size as usize];
            file.read_exact(&mut data).ok()?;
            return (data.starts_with(&[0xFF, 0xD8])).then_some(data);
        }
    }
    None
}

fn alpha_mask(jpeg: &[u8]) -> Option<&[u8]> {
    let mut pos = 2;
    while pos + 4 <= jpeg.len() && jpeg[pos] == 0xFF {
        let marker = jpeg[pos + 1];
        if marker == 0xDA || marker == 0xD9 {
            return None;
        }
        let seg_len = u16::from_be_bytes([jpeg[pos + 2], jpeg[pos + 3]]) as usize;
        let payload = jpeg.get(pos + 4..pos + 2 + seg_len)?;
        if marker == 0xE0 && payload.starts_with(b"ALFA") && payload.len() >= 8 {
            let png_len = u32::from_be_bytes(payload[4..8].try_into().ok()?) as usize;
            return payload.get(8..8usize.checked_add(png_len)?);
        }
        pos += 2 + seg_len;
    }
    None
}

pub fn jpeg_to_transparent_png(jpeg: &[u8]) -> Option<Vec<u8>> {
    let mut rgba = image::load_from_memory_with_format(jpeg, image::ImageFormat::Jpeg).ok()?.to_rgba8();
    if let Some(mask) = alpha_mask(jpeg) {
        if let Ok(decoded) = image::load_from_memory_with_format(mask, image::ImageFormat::Png) {
            let mut luma = decoded.to_luma8();
            if luma.dimensions() != rgba.dimensions() {
                luma = image::imageops::resize(&luma, rgba.width(), rgba.height(), image::imageops::FilterType::Triangle);
            }
            for (pixel, alpha) in rgba.pixels_mut().zip(luma.pixels()) {
                pixel.0[3] = alpha.0[0];
            }
        }
    }
    let mut out = Vec::new();
    rgba.write_to(&mut Cursor::new(&mut out), image::ImageFormat::Png).ok()?;
    Some(out)
}

pub fn save_thumbnail(save: &Path, size: u64, modified: i64, cache_dir: &Path) -> Option<PathBuf> {
    let id = fnv(&format!("save|{}|{size}|{modified}", save.display()));
    let target = cache_dir.join(format!("{id}.png"));
    if target.is_file() {
        return Some(target);
    }
    let png = jpeg_to_transparent_png(&extract_thumbnail_jpeg(save)?)?;
    fs::create_dir_all(cache_dir).ok()?;
    let tmp = cache_dir.join(format!("{id}.tmp"));
    fs::write(&tmp, png).ok()?;
    fs::rename(&tmp, &target).ok()?;
    Some(target)
}
