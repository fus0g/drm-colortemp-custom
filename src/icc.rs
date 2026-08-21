//! Pure-Rust ICC v2.4 profile generator with VCGT (Video Card Gamma Table) support.
//!
//! Generates valid, standards-compliant display ICC color profiles containing
//! calibration lookup tables in the standard `vcgt` tag, readable by colord,
//! Mutter, and other color management systems.

/// Build an ICC v2.4 display profile binary with a `vcgt` gamma table.
///
/// # Arguments
/// * `description` - Human-readable profile description/title (e.g. "DRM ColorFix 8200K")
/// * `red_lut` - Red channel lookup table entries (u16, typically 256 entries)
/// * `green_lut` - Green channel lookup table entries (u16, typically 256 entries)
/// * `blue_lut` - Blue channel lookup table entries (u16, typically 256 entries)
///
/// # Returns
/// Binary ICC profile as `Vec<u8>`.
pub fn create_icc_profile_with_vcgt(
    description: &str,
    red_lut: &[u16],
    green_lut: &[u16],
    blue_lut: &[u16],
) -> Vec<u8> {
    assert_eq!(red_lut.len(), green_lut.len());
    assert_eq!(red_lut.len(), blue_lut.len());
    let entry_count = red_lut.len();

    // Prepare tags:
    // 1. desc (ProfileDescriptionTag)
    let desc_data = build_desc_tag(description);
    // 2. cprt (ProfileCopyrightTag)
    let cprt_data = build_text_tag("DRM Custom ColorFix - Public Domain");
    // 3. wtpt (MediaWhitePointTag) - D50
    let wtpt_data = build_xyz_tag(0x0000f6d6, 0x00010000, 0x0000d32d);
    // 4. rXYZ, gXYZ, bXYZ (Primary matrix columns) - sRGB / D65
    let r_xyz_data = build_xyz_tag(0x00006fa3, 0x000038f6, 0x00000391);
    let g_xyz_data = build_xyz_tag(0x00006294, 0x0000b785, 0x000018dc);
    let b_xyz_data = build_xyz_tag(0x000024a1, 0x00000f85, 0x0000b6d4);
    // 5. rTRC, gTRC, bTRC (Linear TRC curve: count=0)
    let trc_data = build_linear_trc_tag();
    // 6. vcgt (Video Card Gamma Table)
    let vcgt_data = build_vcgt_tag(entry_count, red_lut, green_lut, blue_lut);

    struct TagEntry<'a> {
        signature: [u8; 4],
        data: &'a [u8],
    }

    let tags = [
        TagEntry {
            signature: *b"desc",
            data: &desc_data,
        },
        TagEntry {
            signature: *b"cprt",
            data: &cprt_data,
        },
        TagEntry {
            signature: *b"wtpt",
            data: &wtpt_data,
        },
        TagEntry {
            signature: *b"rXYZ",
            data: &r_xyz_data,
        },
        TagEntry {
            signature: *b"gXYZ",
            data: &g_xyz_data,
        },
        TagEntry {
            signature: *b"bXYZ",
            data: &b_xyz_data,
        },
        TagEntry {
            signature: *b"rTRC",
            data: &trc_data,
        },
        TagEntry {
            signature: *b"gTRC",
            data: &trc_data,
        },
        TagEntry {
            signature: *b"bTRC",
            data: &trc_data,
        },
        TagEntry {
            signature: *b"vcgt",
            data: &vcgt_data,
        },
    ];

    let header_size = 128usize;
    let tag_count = tags.len();
    let tag_table_size = 4 + tag_count * 12; // 4 bytes for count + 12 bytes per tag entry

    // Calculate offsets with 4-byte padding
    let mut current_offset = header_size + tag_table_size;
    let mut tag_offsets: Vec<(u32, u32)> = Vec::with_capacity(tag_count);
    let mut padded_tag_data: Vec<Vec<u8>> = Vec::with_capacity(tag_count);

    for tag in &tags {
        let raw_len = tag.data.len();
        let pad_len = (4 - (raw_len % 4)) % 4;
        let mut padded = Vec::with_capacity(raw_len + pad_len);
        padded.extend_from_slice(tag.data);
        padded.resize(raw_len + pad_len, 0);

        tag_offsets.push((current_offset as u32, raw_len as u32));
        current_offset += padded.len();
        padded_tag_data.push(padded);
    }

    let total_size = current_offset;
    let mut out = Vec::with_capacity(total_size);

    // 1. ICC Header (128 bytes)
    out.extend_from_slice(&(total_size as u32).to_be_bytes()); // 0..4: Profile size
    out.extend_from_slice(b"none"); // 4..8: Preferred CMM
    out.extend_from_slice(&[0x02, 0x40, 0x00, 0x00]); // 8..12: Version 2.4.0
    out.extend_from_slice(b"mntr"); // 12..16: Device class: Monitor
    out.extend_from_slice(b"RGB "); // 16..20: Color space: RGB
    out.extend_from_slice(b"XYZ "); // 20..24: PCS: XYZ
                                    // 24..36: Date/time (year=2026, month=1, day=1, hour=0, min=0, sec=0)
    out.extend_from_slice(&2026u16.to_be_bytes());
    out.extend_from_slice(&1u16.to_be_bytes());
    out.extend_from_slice(&1u16.to_be_bytes());
    out.extend_from_slice(&0u16.to_be_bytes());
    out.extend_from_slice(&0u16.to_be_bytes());
    out.extend_from_slice(&0u16.to_be_bytes());
    out.extend_from_slice(b"acsp"); // 36..40: File signature
    out.extend_from_slice(b"APPL"); // 40..44: Primary platform
    out.extend_from_slice(&[0u8; 4]); // 44..48: Flags
    out.extend_from_slice(&[0u8; 4]); // 48..52: Device manufacturer
    out.extend_from_slice(&[0u8; 4]); // 52..56: Device model
    out.extend_from_slice(&[0u8; 8]); // 56..64: Device attributes
    out.extend_from_slice(&[0u8; 4]); // 64..68: Rendering intent (Perceptual)
                                      // 68..80: D50 illuminant in PCS (s15Fixed16Numbers: X=0.9642, Y=1.0, Z=0.8249)
    out.extend_from_slice(&0x0000f6d6u32.to_be_bytes());
    out.extend_from_slice(&0x00010000u32.to_be_bytes());
    out.extend_from_slice(&0x0000d32du32.to_be_bytes());
    out.extend_from_slice(b"DCCF"); // 80..84: Profile creator signature
    out.extend_from_slice(&[0u8; 16]); // 84..100: Profile ID
    out.extend_from_slice(&[0u8; 28]); // 100..128: Reserved

    assert_eq!(out.len(), 128);

    // 2. Tag Table
    out.extend_from_slice(&(tag_count as u32).to_be_bytes());
    for (i, tag) in tags.iter().enumerate() {
        let (offset, size) = tag_offsets[i];
        out.extend_from_slice(&tag.signature);
        out.extend_from_slice(&offset.to_be_bytes());
        out.extend_from_slice(&size.to_be_bytes());
    }

    // 3. Tag Data Elements
    for padded in padded_tag_data {
        out.extend_from_slice(&padded);
    }

    assert_eq!(out.len(), total_size);
    out
}

fn build_desc_tag(desc: &str) -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(b"desc"); // Type signature
    data.extend_from_slice(&[0u8; 4]); // Reserved

    let ascii_bytes = desc.as_bytes();
    let ascii_len = (ascii_bytes.len() + 1) as u32; // Include null terminator
    data.extend_from_slice(&ascii_len.to_be_bytes());
    data.extend_from_slice(ascii_bytes);
    data.push(0); // Null terminator

    // Unicode language code and count
    data.extend_from_slice(&0u32.to_be_bytes()); // Lang code
    data.extend_from_slice(&0u32.to_be_bytes()); // Unicode count

    // ScriptCode code, count and string
    data.extend_from_slice(&0u16.to_be_bytes()); // ScriptCode code
    data.push(0u8); // ScriptCode count
    data.extend_from_slice(&[0u8; 67]); // Mac scriptcode string

    data
}

fn build_text_tag(text: &str) -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(b"text");
    data.extend_from_slice(&[0u8; 4]);
    data.extend_from_slice(text.as_bytes());
    data.push(0); // Null terminator
    data
}

fn build_xyz_tag(x: u32, y: u32, z: u32) -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(b"XYZ ");
    data.extend_from_slice(&[0u8; 4]);
    data.extend_from_slice(&x.to_be_bytes());
    data.extend_from_slice(&y.to_be_bytes());
    data.extend_from_slice(&z.to_be_bytes());
    data
}

fn build_linear_trc_tag() -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(b"curv");
    data.extend_from_slice(&[0u8; 4]);
    data.extend_from_slice(&0u32.to_be_bytes()); // Count = 0 => linear gamma 1.0
    data
}

fn build_vcgt_tag(
    entry_count: usize,
    red_lut: &[u16],
    green_lut: &[u16],
    blue_lut: &[u16],
) -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(b"vcgt");
    data.extend_from_slice(&[0u8; 4]); // Reserved

    // Gamma type 0 = Table type
    data.extend_from_slice(&0u32.to_be_bytes());
    // Channels = 3
    data.extend_from_slice(&3u16.to_be_bytes());
    // Entry count
    data.extend_from_slice(&(entry_count as u16).to_be_bytes());
    // Entry size in bytes = 2 (u16)
    data.extend_from_slice(&2u16.to_be_bytes());

    // Red channel
    for &val in red_lut {
        data.extend_from_slice(&val.to_be_bytes());
    }
    // Green channel
    for &val in green_lut {
        data.extend_from_slice(&val.to_be_bytes());
    }
    // Blue channel
    for &val in blue_lut {
        data.extend_from_slice(&val.to_be_bytes());
    }

    data
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::temperature;

    #[test]
    fn test_create_icc_profile_header() {
        let (r, g, b) = temperature::generate_gamma_luts(256, 6500, 1.0);
        let profile = create_icc_profile_with_vcgt("Test Profile", &r, &g, &b);

        assert!(profile.len() > 128);
        assert_eq!(&profile[12..16], b"mntr");
        assert_eq!(&profile[16..20], b"RGB ");
        assert_eq!(&profile[20..24], b"XYZ ");
        assert_eq!(&profile[36..40], b"acsp");

        // Profile size in header matches actual byte size
        let header_size = u32::from_be_bytes(profile[0..4].try_into().unwrap()) as usize;
        assert_eq!(header_size, profile.len());
    }

    #[test]
    fn test_create_icc_profile_vcgt_tag_presence() {
        let (r, g, b) = temperature::generate_gamma_luts(256, 8000, 0.9);
        let profile = create_icc_profile_with_vcgt("DRM ColorFix 8000K", &r, &g, &b);

        // Find vcgt tag in tag table
        let tag_count = u32::from_be_bytes(profile[128..132].try_into().unwrap()) as usize;
        assert_eq!(tag_count, 10);

        let mut found_vcgt = false;
        for i in 0..tag_count {
            let entry_offset = 132 + i * 12;
            let sig = &profile[entry_offset..entry_offset + 4];
            if sig == b"vcgt" {
                found_vcgt = true;
                let offset = u32::from_be_bytes(
                    profile[entry_offset + 4..entry_offset + 8]
                        .try_into()
                        .unwrap(),
                ) as usize;
                let size = u32::from_be_bytes(
                    profile[entry_offset + 8..entry_offset + 12]
                        .try_into()
                        .unwrap(),
                ) as usize;

                // vcgt tag size: 4 (sig) + 4 (res) + 4 (type) + 2 (ch) + 2 (entries) + 2 (entry_size) + 256*2*3 (table) = 1554
                assert_eq!(size, 1554);
                assert_eq!(&profile[offset..offset + 4], b"vcgt");
                break;
            }
        }
        assert!(found_vcgt, "vcgt tag not found in generated ICC profile");
    }
}
