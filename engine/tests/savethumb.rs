use s4im_engine::*;
use std::fs;
use std::path::Path;

fn jpeg_with_alpha(width: u32, height: u32, with_mask: bool) -> Vec<u8> {
    let mut rgb = image::RgbImage::new(width, height);
    for p in rgb.pixels_mut() {
        *p = image::Rgb([200, 60, 60]);
    }
    let mut jpeg = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, 90).encode_image(&rgb).unwrap();
    if !with_mask {
        return jpeg;
    }
    let mut mask = image::GrayImage::new(width, height);
    for (x, _, p) in mask.enumerate_pixels_mut() {
        *p = image::Luma([if x < width / 2 { 255 } else { 0 }]);
    }
    let mut png = Vec::new();
    mask.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png).unwrap();

    let mut payload = Vec::new();
    payload.extend_from_slice(b"ALFA");
    payload.extend_from_slice(&(png.len() as u32).to_be_bytes());
    payload.extend_from_slice(&png);
    let mut out = vec![0xFF, 0xD8, 0xFF, 0xE0];
    out.extend_from_slice(&((payload.len() + 2) as u16).to_be_bytes());
    out.extend_from_slice(&payload);
    out.extend_from_slice(&jpeg[2..]);
    out
}

fn dbpf(resources: &[(u32, Vec<u8>)], compression: u16) -> Vec<u8> {
    let mut data = vec![0u8; 96];
    let mut entries = Vec::new();
    for (kind, bytes) in resources {
        entries.push((*kind, data.len() as u32, bytes.len() as u32));
        data.extend_from_slice(bytes);
    }
    let index_pos = data.len() as u64;
    data.extend_from_slice(&0u32.to_le_bytes());
    for (kind, pos, size) in entries {
        for v in [kind, 0, 0, 0, pos, size, size] {
            data.extend_from_slice(&v.to_le_bytes());
        }
        data.extend_from_slice(&compression.to_le_bytes());
        data.extend_from_slice(&1u16.to_le_bytes());
    }
    data[0..4].copy_from_slice(b"DBPF");
    data[4..8].copy_from_slice(&2u32.to_le_bytes());
    data[8..12].copy_from_slice(&1u32.to_le_bytes());
    data[0x24..0x28].copy_from_slice(&(resources.len() as u32).to_le_bytes());
    data[0x40..0x48].copy_from_slice(&index_pos.to_le_bytes());
    data
}

fn write(path: &Path, bytes: &[u8]) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
}

#[test]
fn extracts_the_thumbnail_and_applies_the_alpha_mask() {
    let tmp = tempfile::tempdir().unwrap();
    let save = tmp.path().join("Slot_00000001.save");
    write(&save, &dbpf(&[(0x0D, vec![1; 64]), (0x14, jpeg_with_alpha(32, 20, true))], 0));

    let jpeg = extract_thumbnail_jpeg(&save).unwrap();
    assert_eq!(&jpeg[0..2], &[0xFF, 0xD8]);
    let png = jpeg_to_transparent_png(&jpeg).unwrap();
    let img = image::load_from_memory_with_format(&png, image::ImageFormat::Png).unwrap().to_rgba8();

    assert_eq!(img.dimensions(), (32, 20));
    assert_eq!(img.get_pixel(2, 5).0[3], 255);
    assert_eq!(img.get_pixel(29, 5).0[3], 0);
    assert!(img.get_pixel(2, 5).0[0] > 150);
}

#[test]
fn a_thumbnail_without_mask_stays_opaque() {
    let tmp = tempfile::tempdir().unwrap();
    let save = tmp.path().join("Slot_00000001.save");
    write(&save, &dbpf(&[(0x14, jpeg_with_alpha(16, 16, false))], 0));
    let png = jpeg_to_transparent_png(&extract_thumbnail_jpeg(&save).unwrap()).unwrap();
    let img = image::load_from_memory_with_format(&png, image::ImageFormat::Png).unwrap().to_rgba8();
    assert!(img.pixels().all(|p| p.0[3] == 255));
}

#[test]
fn unusable_files_give_none_and_never_panic() {
    let tmp = tempfile::tempdir().unwrap();
    let t = |name: &str, bytes: &[u8]| {
        let p = tmp.path().join(name);
        write(&p, bytes);
        extract_thumbnail_jpeg(&p)
    };
    assert!(t("vacio.save", b"").is_none());
    assert!(t("texto.save", b"this is not a save").is_none());
    let good = dbpf(&[(0x14, jpeg_with_alpha(16, 16, true))], 0);
    assert!(t("truncado.save", &good[..good.len() - 10]).is_none());
    assert!(t("cabecera.save", &good[..60]).is_none());
    assert!(t("sin_miniatura.save", &dbpf(&[(0x0D, vec![7; 32])], 0)).is_none());
    assert!(t("comprimido.save", &dbpf(&[(0x14, jpeg_with_alpha(16, 16, true))], 0x5A42)).is_none());
    let mut broken = good.clone();
    broken[0x40..0x48].copy_from_slice(&u64::MAX.to_le_bytes());
    assert!(t("indice_roto.save", &broken).is_none());
    let mut huge = good.clone();
    huge[0x24..0x28].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(t("cuenta_enorme.save", &huge).is_none());
    assert!(jpeg_to_transparent_png(b"not a jpeg").is_none());
}

#[test]
fn list_saves_attaches_cached_thumbnails() {
    let tmp = tempfile::tempdir().unwrap();
    let inst = tmp.path().join("Los Sims 4_a");
    let saves = inst.join("saves");
    write(&saves.join("Slot_00000001.save"), &dbpf(&[(0x14, jpeg_with_alpha(24, 16, true))], 0));
    write(&saves.join("Slot_00000002.save"), b"no es dbpf");
    write(&saves.join("Slot_00000001.save.ver0"), b"copy");
    let cache = tmp.path().join("cache");

    let list = list_saves(&inst, Some(&cache));
    let with = list.iter().find(|s| s.slot == 1).unwrap();
    let without = list.iter().find(|s| s.slot == 2).unwrap();
    assert!(without.thumb.is_none());
    let thumb = with.thumb.clone().unwrap();
    assert!(thumb.is_file());
    assert_eq!(thumb.extension().and_then(|e| e.to_str()), Some("png"));

    let stamp = fs::metadata(&thumb).unwrap().modified().unwrap();
    let again = list_saves(&inst, Some(&cache));
    assert_eq!(again.iter().find(|s| s.slot == 1).unwrap().thumb.as_ref(), Some(&thumb));
    assert_eq!(fs::metadata(&thumb).unwrap().modified().unwrap(), stamp);
    assert_eq!(fs::read_dir(&cache).unwrap().count(), 1);
    assert!(list_saves(&inst, None).iter().all(|s| s.thumb.is_none()));
}
