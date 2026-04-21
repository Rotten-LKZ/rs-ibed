use crate::config::MetadataField;

// ── JPEG marker constants ────────────────────────────────────

const SOI: u8 = 0xD8;
const SOS: u8 = 0xDA;
const APP1: u8 = 0xE1;

// ── EXIF tag → group mapping ─────────────────────────────────

// IFD0 tags
const TAG_MAKE: u16 = 0x010F;
const TAG_MODEL: u16 = 0x0110;
const TAG_SOFTWARE: u16 = 0x0131;
const TAG_DATETIME: u16 = 0x0132;
const TAG_ARTIST: u16 = 0x013B;
const TAG_COPYRIGHT: u16 = 0x8298;
const TAG_EXIF_IFD_PTR: u16 = 0x8769;
const TAG_GPS_IFD_PTR: u16 = 0x8825;

// ExifIFD tags
const TAG_EXPOSURE_TIME: u16 = 0x829A;
const TAG_FNUMBER: u16 = 0x829D;
const TAG_EXPOSURE_PROGRAM: u16 = 0x8822;
const TAG_ISO: u16 = 0x8827;
const TAG_DATETIME_ORIGINAL: u16 = 0x9003;
const TAG_DATETIME_DIGITIZED: u16 = 0x9004;
const TAG_SHUTTER_SPEED: u16 = 0x9201;
const TAG_APERTURE: u16 = 0x9202;
const TAG_EXPOSURE_BIAS: u16 = 0x9204;
const TAG_MAX_APERTURE: u16 = 0x9205;
const TAG_METERING_MODE: u16 = 0x9207;
const TAG_FLASH: u16 = 0x9209;
const TAG_FOCAL_LENGTH: u16 = 0x920A;
const TAG_COLOR_SPACE: u16 = 0xA001;
const TAG_FOCAL_LEN_35MM: u16 = 0xA405;
const TAG_EXPOSURE_MODE: u16 = 0xA402;
const TAG_WHITE_BALANCE: u16 = 0xA403;
const TAG_SCENE_CAPTURE: u16 = 0xA406;
const TAG_LENS_MAKE: u16 = 0xA433;
const TAG_LENS_MODEL: u16 = 0xA434;

fn tag_group_ifd0(tag: u16) -> Option<MetadataField> {
    match tag {
        TAG_MAKE | TAG_MODEL | TAG_SOFTWARE => Some(MetadataField::Camera),
        TAG_DATETIME => Some(MetadataField::Time),
        TAG_ARTIST | TAG_COPYRIGHT => Some(MetadataField::Copyright),
        TAG_GPS_IFD_PTR => Some(MetadataField::Location),
        TAG_EXIF_IFD_PTR => None, // structural, always keep
        _ => Some(MetadataField::Others),
    }
}

fn tag_group_exif(tag: u16) -> Option<MetadataField> {
    match tag {
        TAG_EXPOSURE_TIME | TAG_FNUMBER | TAG_EXPOSURE_PROGRAM | TAG_ISO
        | TAG_SHUTTER_SPEED | TAG_APERTURE | TAG_EXPOSURE_BIAS | TAG_MAX_APERTURE
        | TAG_METERING_MODE | TAG_FLASH | TAG_FOCAL_LENGTH | TAG_COLOR_SPACE
        | TAG_FOCAL_LEN_35MM | TAG_EXPOSURE_MODE | TAG_WHITE_BALANCE
        | TAG_SCENE_CAPTURE | TAG_LENS_MAKE | TAG_LENS_MODEL => Some(MetadataField::Settings),
        TAG_DATETIME_ORIGINAL | TAG_DATETIME_DIGITIZED => Some(MetadataField::Time),
        _ => Some(MetadataField::Others),
    }
}

// ── Byte order helpers ───────────────────────────────────────

#[derive(Clone, Copy)]
enum ByteOrder {
    Little,
    Big,
}

fn read_u16(data: &[u8], offset: usize, bo: ByteOrder) -> u16 {
    match bo {
        ByteOrder::Little => u16::from_le_bytes([data[offset], data[offset + 1]]),
        ByteOrder::Big => u16::from_be_bytes([data[offset], data[offset + 1]]),
    }
}

fn read_u32(data: &[u8], offset: usize, bo: ByteOrder) -> u32 {
    match bo {
        ByteOrder::Little => u32::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]),
        ByteOrder::Big => u32::from_be_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]),
    }
}

fn write_zero(data: &mut [u8], offset: usize, len: usize) {
    for b in &mut data[offset..offset + len] {
        *b = 0;
    }
}

// ── Public API ───────────────────────────────────────────────

/// Strip metadata from image bytes according to the keep list.
///
/// For JPEG: surgically zeros out unwanted EXIF IFD entries in-place
/// without re-encoding, so image quality is perfectly preserved.
///
/// For other formats: returns the data unchanged. The re-encoding
/// in the processing pipeline strips all metadata from processed variants.
pub fn strip_metadata(data: &[u8], ext: &str, keep: &[MetadataField]) -> Vec<u8> {
    // If keeping everything, nothing to do
    if has_all_groups(keep) {
        return data.to_vec();
    }

    match ext {
        "jpg" | "jpeg" => strip_jpeg_metadata(data, keep),
        _ => data.to_vec(),
    }
}

fn strip_jpeg_metadata(data: &[u8], keep: &[MetadataField]) -> Vec<u8> {
    let mut buf = data.to_vec();

    // Find APP1 EXIF segment and process it
    if let Some(app1_range) = find_app1_exif(&buf) {
        let tiff_start = app1_range.start;
        strip_exif_ifd_entries(&mut buf, tiff_start, keep);
    }

    buf
}

#[allow(dead_code)]
/// Strip ALL metadata markers from JPEG bytes.
/// Removes APP1-APP15 and COM markers entirely. Keeps APP0 (JFIF).
pub fn strip_all_metadata_jpeg(data: &[u8]) -> Vec<u8> {
    if data.len() < 2 || data[0] != 0xFF || data[1] != SOI {
        return data.to_vec();
    }

    let mut result = Vec::with_capacity(data.len());
    result.extend_from_slice(&[0xFF, SOI]);

    let mut pos = 2;

    while pos + 1 < data.len() {
        if data[pos] != 0xFF {
            result.extend_from_slice(&data[pos..]);
            break;
        }

        let marker = data[pos + 1];

        match marker {
            // Padding
            0xFF => {
                pos += 1;
                continue;
            }
            // SOS: rest is image data
            SOS => {
                result.extend_from_slice(&data[pos..]);
                break;
            }
            // Standalone markers (RST0-7, SOI, EOI, TEM)
            0xD0..=0xD9 | 0x01 => {
                result.extend_from_slice(&data[pos..pos + 2]);
                pos += 2;
            }
            // APP1-APP15, COM: strip
            0xE1..=0xEF | 0xFE => {
                if pos + 3 >= data.len() {
                    break;
                }
                let len = u16::from_be_bytes([data[pos + 2], data[pos + 3]]) as usize;
                pos += 2 + len;
            }
            // Everything else (APP0, DQT, DHT, SOF, DRI, etc.): keep
            _ => {
                if pos + 3 >= data.len() {
                    result.extend_from_slice(&data[pos..]);
                    break;
                }
                let len = u16::from_be_bytes([data[pos + 2], data[pos + 3]]) as usize;
                let end = (pos + 2 + len).min(data.len());
                result.extend_from_slice(&data[pos..end]);
                pos = end;
            }
        }
    }

    result
}

// ── Internal EXIF parsing ────────────────────────────────────

struct App1Range {
    start: usize, // offset of TIFF header within the buffer
}

/// Find the APP1 segment containing EXIF data. Returns the offset
/// of the TIFF header (the "II" or "MM" byte order mark).
fn find_app1_exif(data: &[u8]) -> Option<App1Range> {
    if data.len() < 2 || data[0] != 0xFF || data[1] != SOI {
        return None;
    }

    let mut pos = 2;

    while pos + 1 < data.len() {
        if data[pos] != 0xFF {
            return None;
        }

        let marker = data[pos + 1];

        if marker == SOS {
            return None; // Reached image data, no EXIF found
        }

        // Standalone markers
        if matches!(marker, 0xD0..=0xD9 | 0x01 | 0xFF) {
            pos += if marker == 0xFF { 1 } else { 2 };
            continue;
        }

        if pos + 3 >= data.len() {
            return None;
        }

        let seg_len = u16::from_be_bytes([data[pos + 2], data[pos + 3]]) as usize;
        let seg_data_start = pos + 4; // after marker (2) + length (2)

        if marker == APP1 && seg_len >= 8 {
            // Check for "Exif\0\0" identifier
            if seg_data_start + 6 <= data.len()
                && &data[seg_data_start..seg_data_start + 4] == b"Exif"
                && data[seg_data_start + 4] == 0
                && data[seg_data_start + 5] == 0
            {
                let tiff_start = seg_data_start + 6;
                return Some(App1Range { start: tiff_start });
            }
        }

        pos += 2 + seg_len;
    }

    None
}

/// Walk the TIFF IFD structure and zero out entries whose group
/// is not in the keep list.
fn strip_exif_ifd_entries(data: &mut [u8], tiff_start: usize, keep: &[MetadataField]) {
    if tiff_start + 8 > data.len() {
        return;
    }

    let bo = match &data[tiff_start..tiff_start + 2] {
        b"II" => ByteOrder::Little,
        b"MM" => ByteOrder::Big,
        _ => return,
    };

    let magic = read_u16(data, tiff_start + 2, bo);
    if magic != 42 {
        return;
    }

    let ifd0_offset = read_u32(data, tiff_start + 4, bo) as usize;
    let ifd0_abs = tiff_start + ifd0_offset;

    // Process IFD0
    let exif_ifd_offset = process_ifd(data, ifd0_abs, tiff_start, bo, keep, true);

    // If "location" is not in keep list, zero out the entire GPS IFD data
    // block so that GPS coordinates can't be recovered from the raw bytes.
    if !keep.contains(&MetadataField::Location) {
        zero_gps_ifd(data, ifd0_abs, tiff_start, bo);
    }

    // Process ExifIFD if present
    if let Some(exif_off) = exif_ifd_offset {
        let exif_abs = tiff_start + exif_off;
        process_ifd(data, exif_abs, tiff_start, bo, keep, false);
    }
}

/// Process one IFD: zero out entries whose group is not in the keep list.
/// Returns the ExifIFD offset if found (only relevant for IFD0).
fn process_ifd(
    data: &mut [u8],
    ifd_abs: usize,
    _tiff_start: usize,
    bo: ByteOrder,
    keep: &[MetadataField],
    is_ifd0: bool,
) -> Option<usize> {
    if ifd_abs + 2 > data.len() {
        return None;
    }

    let entry_count = read_u16(data, ifd_abs, bo) as usize;
    let mut exif_ifd_offset: Option<usize> = None;

    for i in 0..entry_count {
        let entry_abs = ifd_abs + 2 + i * 12;
        if entry_abs + 12 > data.len() {
            break;
        }

        let tag = read_u16(data, entry_abs, bo);

        // Capture ExifIFD pointer before any stripping
        if is_ifd0 && tag == TAG_EXIF_IFD_PTR {
            exif_ifd_offset = Some(read_u32(data, entry_abs + 8, bo) as usize);
            continue; // Never strip the ExifIFD pointer itself
        }

        let group = if is_ifd0 {
            tag_group_ifd0(tag)
        } else {
            tag_group_exif(tag)
        };

        if let Some(group) = group {
            if !keep.contains(&group) {
                // Zero out this 12-byte IFD entry
                write_zero(data, entry_abs, 12);
            }
        }
    }

    exif_ifd_offset
}

/// If "location" is NOT in keep list, also zero out the entire GPS IFD
/// data block by following the GPS IFD pointer.
fn zero_gps_ifd(data: &mut [u8], ifd_abs: usize, tiff_start: usize, bo: ByteOrder) {
    if ifd_abs + 2 > data.len() {
        return;
    }

    let entry_count = read_u16(data, ifd_abs, bo) as usize;

    for i in 0..entry_count {
        let entry_abs = ifd_abs + 2 + i * 12;
        if entry_abs + 12 > data.len() {
            break;
        }

        let tag = read_u16(data, entry_abs, bo);

        if tag == TAG_GPS_IFD_PTR {
            let gps_offset = read_u32(data, entry_abs + 8, bo) as usize;
            let gps_abs = tiff_start + gps_offset;

            // Zero the IFD pointer entry itself
            write_zero(data, entry_abs, 12);

            // Zero the GPS IFD entries
            if gps_abs + 2 <= data.len() {
                let gps_count = read_u16(data, gps_abs, bo) as usize;
                let gps_block_size = 2 + gps_count * 12 + 4;
                if gps_abs + gps_block_size <= data.len() {
                    write_zero(data, gps_abs, gps_block_size);
                }
            }
            break;
        }
    }
}

fn has_all_groups(keep: &[MetadataField]) -> bool {
    use MetadataField::*;
    [Camera, Settings, Time, Copyright, Location, Others]
        .iter()
        .all(|g| keep.contains(g))
}

