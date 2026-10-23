use s4im_engine::{read_thumbnails, BUST_TYPES};

fn package(entries: &[(u32, u32, u64, Vec<u8>)]) -> Vec<u8> {
    let mut data = vec![0u8; 96];
    let mut placed = Vec::new();
    for e in entries {
        placed.push(data.len() as u32);
        data.extend_from_slice(&e.3);
    }
    let index_pos = data.len() as u32;
    let mut index = vec![0u8; 4];
    for (i, e) in entries.iter().enumerate() {
        index.extend_from_slice(&e.0.to_le_bytes());
        index.extend_from_slice(&e.1.to_le_bytes());
        index.extend_from_slice(&((e.2 >> 32) as u32).to_le_bytes());
        index.extend_from_slice(&(e.2 as u32).to_le_bytes());
        index.extend_from_slice(&placed[i].to_le_bytes());
        index.extend_from_slice(&((e.3.len() as u32) | 0x8000_0000).to_le_bytes());
        index.extend_from_slice(&(e.3.len() as u32).to_le_bytes());
        index.extend_from_slice(&0u16.to_le_bytes());
        index.extend_from_slice(&1u16.to_le_bytes());
    }
    data[0..4].copy_from_slice(b"DBPF");
    data[4..8].copy_from_slice(&2u32.to_le_bytes());
    data[8..12].copy_from_slice(&1u32.to_le_bytes());
    data[36..40].copy_from_slice(&(entries.len() as u32).to_le_bytes());
    data[44..48].copy_from_slice(&(index.len() as u32).to_le_bytes());
    data[64..68].copy_from_slice(&index_pos.to_le_bytes());
    data.extend_from_slice(&index);
    data
}

fn jpeg(tag: u8) -> Vec<u8> {
    vec![0xFF, 0xD8, 0xFF, 0xE0, 0, 16, tag, tag, tag, tag]
}

#[test]
fn reads_only_bust_jpegs_and_leaves_the_package_untouched() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("localthumbcache.package");
    let bytes = package(&[
        (0x16CC_F748, 1, 0x1111_2222_3333_4444, jpeg(1)),
        (0x3C2A_8647, 2, 7, jpeg(2)),
        (0x16CC_F748, 3, 9, vec![1, 2, 3, 4, 5, 6]),
        (0, 4, 10, jpeg(3)),
    ]);
    std::fs::write(&file, &bytes).unwrap();
    let found = read_thumbnails(&file, &BUST_TYPES).unwrap();
    assert_eq!(found.len(), 2);
    assert_eq!(found[0].instance, 0x1111_2222_3333_4444);
    assert_eq!(found[0].data, jpeg(1));
    assert_eq!(found[1].data, jpeg(3));
    assert_eq!(std::fs::read(&file).unwrap(), bytes);
}

#[test]
fn rejects_files_that_are_not_packages() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("x.package");
    std::fs::write(&file, vec![0u8; 200]).unwrap();
    assert!(read_thumbnails(&file, &BUST_TYPES).is_err());
}

#[test]
fn composes_png_with_the_embedded_alpha() {
    use image::{ImageFormat, Rgb, RgbImage};
    let mut jpeg = Vec::new();
    image::DynamicImage::ImageRgb8(RgbImage::from_pixel(4, 4, Rgb([200, 100, 50])))
        .write_to(&mut std::io::Cursor::new(&mut jpeg), ImageFormat::Jpeg)
        .unwrap();
    let mut mask = image::GrayImage::from_pixel(4, 4, image::Luma([255]));
    mask.put_pixel(0, 0, image::Luma([0]));
    let mut alpha = Vec::new();
    image::DynamicImage::ImageLuma8(mask)
        .write_to(&mut std::io::Cursor::new(&mut alpha), ImageFormat::Png)
        .unwrap();
    let mut segment = b"ALFA".to_vec();
    segment.extend_from_slice(&(alpha.len() as u32).to_be_bytes());
    segment.extend_from_slice(&alpha);
    let mut with_alpha = jpeg[0..2].to_vec();
    with_alpha.extend_from_slice(&[0xFF, 0xE0]);
    with_alpha.extend_from_slice(&((segment.len() + 2) as u16).to_be_bytes());
    with_alpha.extend_from_slice(&segment);
    with_alpha.extend_from_slice(&jpeg[2..]);
    let png = s4im_engine::thumbnail_to_png(&with_alpha).unwrap();
    let decoded = image::load_from_memory(&png).unwrap().to_rgba8();
    assert_eq!(decoded.get_pixel(0, 0)[3], 0);
    assert_eq!(decoded.get_pixel(3, 3)[3], 255);
    let plain = s4im_engine::thumbnail_to_png(&jpeg).unwrap();
    assert_eq!(image::load_from_memory(&plain).unwrap().to_rgba8().get_pixel(0, 0)[3], 255);
}
