//! Cross-platform CJK font discovery and pure-Rust clean-room OpenType fallback synthesizer.
//!
//! Ensures `krilla` PDF authoring (`generate_sample_korean_contract_pdf`, `QuickSignText`) and
//! `hayro-interpret` fallback resolution succeed across macOS, Windows, Linux, and WASM without
//! bundling proprietary fonts or introducing C/C++ OS bindings.

use std::collections::BTreeSet;
use std::sync::{Arc, OnceLock};

use craft_pdf::interpret::InterpreterSettings;
use craft_pdf::interpret::font::{FontData, FontQuery};
use craft_pdf::write;
use krilla::text::Font;

use crate::PdfError;

#[cfg(not(target_arch = "wasm32"))]
const SYSTEM_CJK_FONT_CANDIDATES: &[(&str, u32)] = &[
    // macOS
    ("/System/Library/Fonts/AppleSDGothicNeo.ttc", 0),
    ("/System/Library/Fonts/Supplemental/AppleGothic.ttf", 0),
    // Windows
    (r"C:\Windows\Fonts\malgun.ttf", 0),
    (r"C:\Windows\Fonts\gulim.ttc", 0),
    // Linux (Noto Sans CJK / NanumGothic)
    ("/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc", 0),
    ("/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc", 0),
    ("/usr/share/fonts/truetype/nanum/NanumGothic.ttf", 0),
    ("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf", 0),
];

/// Cached raw bytes and TTC face index of the host system's CJK font, if available.
pub(crate) fn cached_system_cjk_font_bytes() -> Option<(Arc<Vec<u8>>, u32)> {
    static CACHED: OnceLock<Option<(Arc<Vec<u8>>, u32)>> = OnceLock::new();
    CACHED
        .get_or_init(|| {
            #[cfg(not(target_arch = "wasm32"))]
            {
                for &(path, index) in SYSTEM_CJK_FONT_CANDIDATES {
                    if let Ok(bytes) = std::fs::read(path)
                        && write::font(&bytes, index, None).is_ok()
                    {
                        return Some((Arc::new(bytes), index));
                    }
                }
                None
            }
            #[cfg(target_arch = "wasm32")]
            {
                None
            }
        })
        .clone()
}

/// Resolve a `krilla::text::Font` capable of shaping and subsetting `required_text`.
///
/// Prefers the platform's CJK font on native targets and falls back to a deterministic,
/// clean-room minimal TrueType font synthesized in memory (used on `wasm32-unknown-unknown`
/// and minimal headless containers with no system CJK fonts installed).
pub(crate) fn resolve_krilla_font(required_text: &str) -> Result<Font, PdfError> {
    if let Some((bytes, index)) = cached_system_cjk_font_bytes()
        && let Ok(font) = write::font(&bytes, index, None)
    {
        return Ok(font);
    }

    let mut codepoints: BTreeSet<char> = (' '..='~').collect();
    for ch in required_text.chars() {
        if !ch.is_control() {
            codepoints.insert(ch);
        }
    }
    let chars: Vec<char> = codepoints.into_iter().collect();
    let ttf_bytes = synthesize_minimal_unicode_ttf(&chars);
    write::font(&ttf_bytes, 0, None).map_err(PdfError::from)
}

/// Build `InterpreterSettings` with embedded standard fonts/CMaps AND system CJK fallback
/// for PDFs that omit embedded font programs for CJK CIDFonts.
pub(crate) fn build_interpreter_settings() -> InterpreterSettings {
    let base = InterpreterSettings::default();
    let default_resolver = base.font_resolver.clone();
    let font_resolver = Arc::new(move |query: &FontQuery| -> Option<(FontData, u32)> {
        if let Some(found) = default_resolver(query) {
            return Some(found);
        }
        if let FontQuery::Fallback(fb) = query {
            if let Some((bytes, index)) = cached_system_cjk_font_bytes() {
                let data: FontData = bytes;
                return Some((data, index));
            }
            let std_query = FontQuery::Standard(fb.pick_standard_font());
            return default_resolver(&std_query);
        }
        None
    });

    InterpreterSettings {
        font_resolver,
        ..base
    }
}

/// Synthesizes a valid, self-contained OpenType/TrueType font (`sfnt` v1.0) in memory
/// mapping every character in `chars` via `cmap` format 12 to clean geometric glyph contours.
///
/// 100% pure Rust, zero external assets, `MIT OR Apache-2.0` clean-room construction.
pub(crate) fn synthesize_minimal_unicode_ttf(chars: &[char]) -> Vec<u8> {
    let mut sorted: Vec<u32> = chars.iter().map(|&c| u32::from(c)).collect();
    sorted.sort_unstable();
    sorted.dedup();

    // Glyph 0 is `.notdef`; glyphs 1..=N correspond to `sorted`.
    let num_glyphs_usize = sorted.len().saturating_add(1).min(65534);
    let sorted = &sorted[..num_glyphs_usize.saturating_sub(1)];
    let num_glyphs = num_glyphs_usize as u16;

    // Build `glyf`, `loca` (format 1: u32 offsets), and `hmtx`.
    let mut glyf = Vec::new();
    let mut loca = Vec::with_capacity((num_glyphs_usize + 1) * 4);
    let mut hmtx = Vec::with_capacity(num_glyphs_usize * 4);

    // Helper to append a simple rectangular contour glyph (or empty space glyph).
    let append_rect_glyph =
        |glyf_buf: &mut Vec<u8>, x_min: i16, y_min: i16, x_max: i16, y_max: i16, empty: bool| {
            if empty {
                return 0i16;
            }
            // numberOfContours = 1, xMin, yMin, xMax, yMax
            push_i16(glyf_buf, 1);
            push_i16(glyf_buf, x_min);
            push_i16(glyf_buf, y_min);
            push_i16(glyf_buf, x_max);
            push_i16(glyf_buf, y_max);
            // endPtsOfContours[0] = 3 (4 points)
            push_u16(glyf_buf, 3);
            // instructionLength = 0
            push_u16(glyf_buf, 0);
            // 4 on-curve flags (0x01)
            glyf_buf.extend_from_slice(&[0x01, 0x01, 0x01, 0x01]);
            // X coordinates (relative): x_min, 0, +(x_max - x_min), 0
            let dx = x_max.saturating_sub(x_min);
            let dy = y_max.saturating_sub(y_min);
            push_i16_coord_deltas(glyf_buf, &[x_min, 0, dx, 0]);
            // Y coordinates (relative): y_min, +dy, 0, -dy
            push_i16_coord_deltas(glyf_buf, &[y_min, dy, 0, -dy]);
            while !glyf_buf.len().is_multiple_of(4) {
                glyf_buf.push(0);
            }
            x_min
        };

    // Glyph 0 (.notdef)
    loca.extend_from_slice(&(glyf.len() as u32).to_be_bytes());
    let lsb0 = append_rect_glyph(&mut glyf, 60, 0, 540, 700, false);
    push_u16(&mut hmtx, 600);
    push_i16(&mut hmtx, lsb0);

    for &cp in sorted {
        loca.extend_from_slice(&(glyf.len() as u32).to_be_bytes());
        let is_wide = (0x1100..=0x11FF).contains(&cp)
            || (0x2E80..=0x9FFF).contains(&cp)
            || (0xAC00..=0xD7AF).contains(&cp)
            || (0xF900..=0xFAFF).contains(&cp)
            || (0xFF01..=0xFF60).contains(&cp);
        let is_space = cp == 0x20 || cp == 0xA0 || cp == 0x3000;
        let adv: u16 = if is_wide { 1000 } else { 560 };
        let (x0, y0, x1, y1) = if is_wide {
            (70i16, 40i16, 930i16, 780i16)
        } else {
            (50i16, 0i16, 510i16, 720i16)
        };
        let lsb = append_rect_glyph(&mut glyf, x0, y0, x1, y1, is_space);
        push_u16(&mut hmtx, adv);
        push_i16(&mut hmtx, lsb);
    }
    loca.extend_from_slice(&(glyf.len() as u32).to_be_bytes());

    // `head` table (54 bytes)
    let mut head = Vec::with_capacity(54);
    push_u32(&mut head, 0x0001_0000); // version 1.0
    push_u32(&mut head, 0x0001_0000); // fontRevision
    push_u32(&mut head, 0); // checkSumAdjustment
    push_u32(&mut head, 0x5F0F_3CF5); // magicNumber
    push_u16(&mut head, 0x000B); // flags
    push_u16(&mut head, 1000); // unitsPerEm
    head.extend_from_slice(&0i64.to_be_bytes()); // created
    head.extend_from_slice(&0i64.to_be_bytes()); // modified
    push_i16(&mut head, 0); // xMin
    push_i16(&mut head, -200); // yMin
    push_i16(&mut head, 1000); // xMax
    push_i16(&mut head, 850); // yMax
    push_u16(&mut head, 0); // macStyle
    push_u16(&mut head, 8); // lowestRecPPEM
    push_i16(&mut head, 2); // fontDirectionHint
    push_i16(&mut head, 1); // indexToLocFormat = 1 (u32 loca)
    push_i16(&mut head, 0); // glyphDataFormat

    // `hhea` table (36 bytes)
    let mut hhea = Vec::with_capacity(36);
    push_u32(&mut hhea, 0x0001_0000);
    push_i16(&mut hhea, 850); // ascent
    push_i16(&mut hhea, -150); // descent
    push_i16(&mut hhea, 0); // lineGap
    push_u16(&mut hhea, 1000); // advanceWidthMax
    push_i16(&mut hhea, 0); // minLeftSideBearing
    push_i16(&mut hhea, 0); // minRightSideBearing
    push_i16(&mut hhea, 950); // xMaxExtent
    push_i16(&mut hhea, 1); // caretSlopeRise
    push_i16(&mut hhea, 0); // caretSlopeRun
    push_i16(&mut hhea, 0); // caretOffset
    for _ in 0..4 {
        push_i16(&mut hhea, 0);
    }
    push_i16(&mut hhea, 0); // metricDataFormat
    push_u16(&mut hhea, num_glyphs); // numberOfHMetrics

    // `maxp` table v1.0 (32 bytes)
    let mut maxp = Vec::with_capacity(32);
    push_u32(&mut maxp, 0x0001_0000);
    push_u16(&mut maxp, num_glyphs);
    push_u16(&mut maxp, 4); // maxPoints
    push_u16(&mut maxp, 1); // maxContours
    for _ in 0..11 {
        push_u16(&mut maxp, 0);
    }

    // `OS/2` table v4 (96 bytes)
    let mut os2 = Vec::with_capacity(96);
    push_u16(&mut os2, 4); // version
    push_i16(&mut os2, 600); // xAvgCharWidth
    push_u16(&mut os2, 400); // usWeightClass (Regular)
    push_u16(&mut os2, 5); // usWidthClass (Medium)
    push_u16(&mut os2, 0); // fsType (Installable embedding allowed)
    for &v in &[650i16, 600, 0, 75, 650, 600, 0, 480, 50, 300] {
        push_i16(&mut os2, v);
    }
    push_i16(&mut os2, 0); // sFamilyClass
    os2.extend_from_slice(&[0u8; 10]); // panose
    for _ in 0..4 {
        push_u32(&mut os2, 0); // ulUnicodeRange1..4
    }
    os2.extend_from_slice(b"OOFF"); // achVendID
    push_u16(&mut os2, 0x0040); // fsSelection (REGULAR)
    let first_cp = sorted.first().copied().unwrap_or(0x20).min(0xFFFF) as u16;
    let last_cp = sorted.last().copied().unwrap_or(0x7E).min(0xFFFF) as u16;
    push_u16(&mut os2, first_cp);
    push_u16(&mut os2, last_cp);
    push_i16(&mut os2, 850); // sTypoAscender
    push_i16(&mut os2, -150); // sTypoDescender
    push_i16(&mut os2, 0); // sTypoLineGap
    push_u16(&mut os2, 850); // usWinAscent
    push_u16(&mut os2, 150); // usWinDescent
    push_u32(&mut os2, 1); // ulCodePageRange1
    push_u32(&mut os2, 0); // ulCodePageRange2
    push_i16(&mut os2, 500); // sxHeight
    push_i16(&mut os2, 720); // sCapHeight
    push_u16(&mut os2, 0); // usDefaultChar
    push_u16(&mut os2, 0x20); // usBreakChar
    push_u16(&mut os2, 1); // usMaxContext

    // `name` table (minimal PostScript & family name)
    let family_utf16 = encode_utf16be("OneOfficeCleanSans");
    let mut name = Vec::new();
    push_u16(&mut name, 0); // format
    push_u16(&mut name, 2); // count (nameID 1 Family, nameID 6 PostScript)
    push_u16(&mut name, 6 + 2 * 12); // stringOffset
    for &name_id in &[1u16, 6u16] {
        push_u16(&mut name, 3); // platformID = Windows
        push_u16(&mut name, 1); // encodingID = Unicode BMP
        push_u16(&mut name, 0x0409); // languageID = en-US
        push_u16(&mut name, name_id);
        push_u16(&mut name, family_utf16.len() as u16);
        push_u16(&mut name, 0); // shared offset 0
    }
    name.extend_from_slice(&family_utf16);

    // `post` table v3.0 (32 bytes, no glyph names needed)
    let mut post = Vec::with_capacity(32);
    push_u32(&mut post, 0x0003_0000);
    push_u32(&mut post, 0); // italicAngle
    push_i16(&mut post, -100); // underlinePosition
    push_i16(&mut post, 50); // underlineThickness
    push_u32(&mut post, 0); // isFixedPitch
    for _ in 0..4 {
        push_u32(&mut post, 0);
    }

    // `cmap` table with Format 12 (Segmented Coverage, platform 3 encoding 10 + platform 0 encoding 4)
    let mut groups: Vec<(u32, u32, u32)> = Vec::new();
    for (idx, &cp) in sorted.iter().enumerate() {
        let gid = (idx + 1) as u32;
        if let Some(last) = groups.last_mut()
            && cp == last.1 + 1
            && gid == last.2 + (cp - last.0)
        {
            last.1 = cp;
        } else {
            groups.push((cp, cp, gid));
        }
    }
    let mut sub12 = Vec::with_capacity(16 + groups.len() * 12);
    push_u16(&mut sub12, 12); // format 12
    push_u16(&mut sub12, 0); // reserved
    push_u32(&mut sub12, (16 + groups.len() * 12) as u32); // length
    push_u32(&mut sub12, 0); // language
    push_u32(&mut sub12, groups.len() as u32);
    for &(start_cp, end_cp, start_gid) in &groups {
        push_u32(&mut sub12, start_cp);
        push_u32(&mut sub12, end_cp);
        push_u32(&mut sub12, start_gid);
    }

    let mut cmap = Vec::new();
    push_u16(&mut cmap, 0); // version
    push_u16(&mut cmap, 2); // numTables
    let sub_offset = 4 + 2 * 8;
    // Record 0: Unicode full repertoire (0, 4)
    push_u16(&mut cmap, 0);
    push_u16(&mut cmap, 4);
    push_u32(&mut cmap, sub_offset);
    // Record 1: Windows UCS-4 (3, 10)
    push_u16(&mut cmap, 3);
    push_u16(&mut cmap, 10);
    push_u32(&mut cmap, sub_offset);
    cmap.extend_from_slice(&sub12);

    // Assemble sfnt directory sorted by tag!
    let mut tables: Vec<([u8; 4], Vec<u8>)> = vec![
        (*b"OS/2", os2),
        (*b"cmap", cmap),
        (*b"glyf", glyf),
        (*b"head", head),
        (*b"hhea", hhea),
        (*b"hmtx", hmtx),
        (*b"loca", loca),
        (*b"maxp", maxp),
        (*b"name", name),
        (*b"post", post),
    ];
    tables.sort_by_key(|(tag, _)| *tag);

    let num_tables = tables.len() as u16;
    let mut entry_selector = 0u16;
    let mut max_pow2 = 1u16;
    while max_pow2 * 2 <= num_tables {
        max_pow2 *= 2;
        entry_selector += 1;
    }
    let search_range = max_pow2 * 16;
    let range_shift = num_tables * 16 - search_range;

    let mut out = Vec::new();
    push_u32(&mut out, 0x0001_0000); // sfntVersion TrueType
    push_u16(&mut out, num_tables);
    push_u16(&mut out, search_range);
    push_u16(&mut out, entry_selector);
    push_u16(&mut out, range_shift);

    let dir_end = 12 + tables.len() * 16;
    let mut offset = dir_end;
    let mut body = Vec::new();
    for (tag, data) in &tables {
        let checksum = sfnt_checksum(data);
        let len = data.len() as u32;
        out.extend_from_slice(tag);
        push_u32(&mut out, checksum);
        push_u32(&mut out, offset as u32);
        push_u32(&mut out, len);
        body.extend_from_slice(data);
        while !body.len().is_multiple_of(4) {
            body.push(0);
        }
        offset = dir_end + body.len();
    }
    out.extend_from_slice(&body);
    out
}

fn push_u16(buf: &mut Vec<u8>, v: u16) {
    buf.extend_from_slice(&v.to_be_bytes());
}

fn push_i16(buf: &mut Vec<u8>, v: i16) {
    buf.extend_from_slice(&v.to_be_bytes());
}

fn push_u32(buf: &mut Vec<u8>, v: u32) {
    buf.extend_from_slice(&v.to_be_bytes());
}

fn push_i16_coord_deltas(buf: &mut Vec<u8>, deltas: &[i16]) {
    for &d in deltas {
        push_i16(buf, d);
    }
}

fn encode_utf16be(s: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(s.len() * 2);
    for unit in s.encode_utf16() {
        out.extend_from_slice(&unit.to_be_bytes());
    }
    out
}

fn sfnt_checksum(data: &[u8]) -> u32 {
    let mut sum = 0u32;
    let mut chunks = data.chunks_exact(4);
    for c in &mut chunks {
        let word = u32::from_be_bytes([c[0], c[1], c[2], c[3]]);
        sum = sum.wrapping_add(word);
    }
    let rem = chunks.remainder();
    if !rem.is_empty() {
        let mut padded = [0u8; 4];
        padded[..rem.len()].copy_from_slice(rem);
        sum = sum.wrapping_add(u32::from_be_bytes(padded));
    }
    sum
}
