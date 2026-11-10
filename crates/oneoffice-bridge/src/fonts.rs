//! Korean HarfRust font measurement and Kinsoku-aware eojeol line wrapping (`wordcraft-fonts`).
//!
//! Uses `wordcraft_fonts::FontDb::global()` and `system_cjk_ui_font_for(false, true, ...)` when
//! system Korean fonts (`Pretendard`, `Apple SD Gothic Neo`, `Malgun Gothic`, `Noto Sans CJK KR`)
//! are available on Native, and seamlessly falls back to bundled Latin HarfRust shaping plus
//! calibrated Hangul/CJK advance metrics on headless or `wasm32-unknown-unknown` targets.

use std::sync::{Arc, OnceLock};

use serde::{Deserialize, Serialize};
use wordcraft_fonts::{FALLBACK_FAMILY, FontDb, FontFace};

use crate::error::BridgeError;

/// Candidate Korean UI/document font families checked in priority order.
const KOREAN_FONT_CANDIDATES: &[&str] = &[
    "Pretendard",
    "Pretendard Variable",
    "Apple SD Gothic Neo",
    "Malgun Gothic",
    "맑은 고딕",
    "Noto Sans CJK KR",
    "Noto Sans KR",
    "Source Han Sans KR",
    "NanumGothic",
    "나눔고딕",
    "NanumBarunGothic",
    "AppleGothic",
];

/// Representative Hangul probe characters used to verify real Korean glyph coverage.
const KOREAN_PROBE_CHARS: &[char] = &['한', '글', '가', '제', '안', '서'];

/// Multi-line bounding-box measurement produced by [`TextMeasurer::measure_wrapped_box`].
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct TextBoxMetrics {
    /// Wrapped lines after applying Korean eojeol and Kinsoku line-break rules.
    pub lines: Vec<String>,
    /// Maximum measured line width in points (`pt`).
    pub width_pt: f32,
    /// Total height of all wrapped lines in points (`pt`).
    pub height_pt: f32,
    /// Single-line height in points (`pt`).
    pub line_height_pt: f32,
}

/// Shared text-measurement contract consumed by `oneoffice-diagram` and chart layout engines.
pub trait TextMeasurer {
    /// Measures the advance width of `text` in points (`pt`). For multi-line text containing
    /// `'\n'`, returns the maximum width across lines.
    fn measure_text_pt(&self, text: &str, font_size_pt: f32, bold: bool) -> f32;

    /// Computes recommended single-line height in points (`pt`) for Korean/mixed typography.
    fn line_height_pt(&self, font_size_pt: f32) -> f32;

    /// Wraps `text` to fit within `max_width_pt`, preserving Korean eojeol (space-delimited phrase)
    /// boundaries first and enforcing Kinsoku line-start / line-end punctuation rules.
    fn wrap_korean_lines(
        &self,
        text: &str,
        max_width_pt: f32,
        font_size_pt: f32,
        bold: bool,
    ) -> Vec<String>;

    /// Wraps `text` and computes its resulting bounding box (`width_pt`, `height_pt`).
    fn measure_wrapped_box(
        &self,
        text: &str,
        max_width_pt: f32,
        font_size_pt: f32,
        bold: bool,
    ) -> TextBoxMetrics {
        let lines = self.wrap_korean_lines(text, max_width_pt, font_size_pt, bold);
        let lh = self.line_height_pt(font_size_pt);
        let width_pt = lines
            .iter()
            .map(|line| self.measure_text_pt(line, font_size_pt, bold))
            .fold(0.0f32, f32::max);
        let height_pt = if lines.is_empty() {
            0.0
        } else {
            lh * (lines.len() as f32)
        };
        TextBoxMetrics {
            lines,
            width_pt,
            height_pt,
            line_height_pt: lh,
        }
    }
}

/// Korean-first text measurer backed by `wordcraft_fonts` (`harfrust` + `skrifa`).
#[derive(Clone, Debug, Default)]
pub struct KoreanTextMeasurer {
    preferred_family: Option<String>,
}

impl KoreanTextMeasurer {
    /// Creates a new [`KoreanTextMeasurer`] backed by [`FontDb::global()`] and system Korean UI
    /// font discovery where available.
    #[must_use]
    pub fn new() -> Self {
        let _ = default_korean_family();
        Self {
            preferred_family: None,
        }
    }

    /// Creates a [`KoreanTextMeasurer`] that tries `family` first before system Korean fallbacks.
    #[must_use]
    pub fn with_preferred_family(family: impl Into<String>) -> Self {
        let _ = default_korean_family();
        Self {
            preferred_family: Some(family.into()),
        }
    }

    /// Installs TTF/OTF/TTC/WOFF/WOFF2 font bytes into [`FontDb::global()`] for runtime WASM or
    /// custom-font environments and returns the newly added family names.
    pub fn install_font_bytes(name: &str, bytes: &[u8]) -> Result<Vec<String>, BridgeError> {
        match FontDb::global().install_font(name, bytes) {
            Ok(installed) => Ok(installed.font.families()),
            Err(wordcraft_fonts::UserFontError::AlreadyAvailable(existing)) => Ok(existing),
            Err(err) => Err(BridgeError::FontInstall(err.to_string())),
        }
    }

    /// Returns the resolved Korean font family name if a Hangul-capable font face is loaded.
    #[must_use]
    pub fn resolved_korean_family(&self) -> Option<String> {
        let db = FontDb::global();
        if let Some(pref) = &self.preferred_family
            && db.has_family(pref)
        {
            let face = db.face(pref, "Regular");
            if covers_korean(&face) {
                return Some(face.family.clone());
            }
        }
        if let Some(fam) = default_korean_family() {
            return Some(fam.to_string());
        }
        // Check any runtime user-installed fonts (e.g., on WASM after `install_font_bytes`).
        for uf in db.user_fonts() {
            for fam in uf.families() {
                let face = db.face(&fam, "Regular");
                if covers_korean(&face) {
                    return Some(face.family.clone());
                }
            }
        }
        None
    }

    /// Measures `text` width in points (`pt`). Multi-line strings return the widest line's width.
    #[must_use]
    pub fn measure_text_pt(&self, text: &str, font_size_pt: f32, bold: bool) -> f32 {
        if text.is_empty() || !font_size_pt.is_finite() || font_size_pt <= 0.0 {
            return 0.0;
        }
        let size = font_size_pt.min(10_000.0);
        text.split('\n')
            .map(|raw| self.measure_single_line_pt(raw.trim_end_matches('\r'), size, bold))
            .fold(0.0f32, f32::max)
    }

    /// Computes single-line height in points (`pt`) from resolved Korean font vertical metrics
    /// (clamped to readable Korean proposal line spacing `1.30..=1.50 em`).
    #[must_use]
    pub fn line_height_pt(&self, font_size_pt: f32) -> f32 {
        if !font_size_pt.is_finite() || font_size_pt <= 0.0 {
            return 0.0;
        }
        let size = font_size_pt.min(10_000.0);
        if let Some(fam) = self.resolved_korean_family() {
            let face = FontDb::global().face(&fam, "Regular");
            if face.upem > 0.0 {
                let raw_em = ((face.ascent + face.descent) / face.upem) as f32;
                if raw_em.is_finite() && raw_em > 0.0 {
                    return size * raw_em.clamp(1.30, 1.50);
                }
            }
        }
        size * 1.35
    }

    /// Wraps `text` into lines no wider than `max_width_pt` (except when a single Kinsoku atom is
    /// wider than `max_width_pt`), prioritizing Korean eojeol (whitespace) boundaries and
    /// preventing Kinsoku line-start / line-end forbidden punctuation violations.
    #[must_use]
    pub fn wrap_korean_lines(
        &self,
        text: &str,
        max_width_pt: f32,
        font_size_pt: f32,
        bold: bool,
    ) -> Vec<String> {
        if text.is_empty() {
            return Vec::new();
        }
        if !font_size_pt.is_finite() || font_size_pt <= 0.0 {
            return text
                .split('\n')
                .map(|line| line.trim_end_matches('\r').to_string())
                .collect();
        }
        let size = font_size_pt.min(10_000.0);
        let mut out = Vec::new();

        for raw_para in text.split('\n') {
            let para = raw_para.trim_end_matches('\r');
            if para.trim().is_empty() {
                out.push(String::new());
                continue;
            }

            if !max_width_pt.is_finite() || max_width_pt <= 0.0 {
                for word in normalize_eojeols(para) {
                    for atom in split_kinsoku_atoms(&word) {
                        let trimmed = atom.trim();
                        if !trimmed.is_empty() {
                            out.push(trimmed.to_string());
                        }
                    }
                }
                continue;
            }

            let eojeols = normalize_eojeols(para);
            let mut current_line = String::new();

            for eojeol in eojeols {
                if current_line.is_empty() {
                    self.place_eojeol_on_fresh_line(&eojeol, max_width_pt, size, bold, &mut out, &mut current_line);
                } else {
                    let candidate = format!("{current_line} {eojeol}");
                    if self.measure_single_line_pt(&candidate, size, bold) <= max_width_pt {
                        current_line = candidate;
                    } else {
                        out.push(std::mem::take(&mut current_line));
                        self.place_eojeol_on_fresh_line(
                            &eojeol,
                            max_width_pt,
                            size,
                            bold,
                            &mut out,
                            &mut current_line,
                        );
                    }
                }
            }

            if !current_line.is_empty() {
                out.push(current_line);
            }
        }

        out
    }

    /// Wraps `text` and returns full multi-line bounding-box metrics.
    #[must_use]
    pub fn measure_wrapped_box(
        &self,
        text: &str,
        max_width_pt: f32,
        font_size_pt: f32,
        bold: bool,
    ) -> TextBoxMetrics {
        <Self as TextMeasurer>::measure_wrapped_box(self, text, max_width_pt, font_size_pt, bold)
    }

    fn place_eojeol_on_fresh_line(
        &self,
        eojeol: &str,
        max_width_pt: f32,
        font_size_pt: f32,
        bold: bool,
        out: &mut Vec<String>,
        current_line: &mut String,
    ) {
        if self.measure_single_line_pt(eojeol, font_size_pt, bold) <= max_width_pt {
            *current_line = eojeol.to_string();
            return;
        }

        let atoms = split_kinsoku_atoms(eojeol);
        let mut sub_line = String::new();
        for atom in atoms {
            if sub_line.is_empty() {
                sub_line = atom.trim_start().to_string();
            } else {
                let candidate = format!("{sub_line}{atom}");
                if self.measure_single_line_pt(candidate.trim_end(), font_size_pt, bold)
                    <= max_width_pt
                {
                    sub_line = candidate;
                } else {
                    let flushed = sub_line.trim_end().to_string();
                    if !flushed.is_empty() {
                        out.push(flushed);
                    }
                    sub_line = atom.trim_start().to_string();
                }
            }
        }
        *current_line = sub_line.trim_end().to_string();
    }

    fn measure_single_line_pt(&self, line: &str, font_size_pt: f32, bold: bool) -> f32 {
        if line.is_empty() {
            return 0.0;
        }
        let db = FontDb::global();
        let style = if bold { "Bold" } else { "Regular" };
        let latin_face = db.face(FALLBACK_FAMILY, style);
        let ko_face = self
            .resolved_korean_family()
            .map(|fam| db.face(&fam, style));

        #[derive(Clone, Copy, PartialEq, Eq)]
        enum RunKind {
            Korean,
            Latin,
            Fallback,
        }

        let classify = |ch: char| -> RunKind {
            if ch.is_control() {
                return RunKind::Fallback;
            }
            if let Some(kf) = &ko_face
                && kf.covers(ch)
            {
                return RunKind::Korean;
            }
            if latin_face.covers(ch) {
                return RunKind::Latin;
            }
            RunKind::Fallback
        };

        let mut total_pt = 0.0f32;
        let mut run_start = 0usize;
        let mut current_kind: Option<RunKind> = None;

        for (idx, ch) in line.char_indices() {
            let kind = classify(ch);
            match current_kind {
                None => {
                    run_start = idx;
                    current_kind = Some(kind);
                }
                Some(prev) if prev != kind => {
                    if let Some(sub) = line.get(run_start..idx) {
                        total_pt += measure_subrun(
                            sub,
                            prev,
                            ko_face.as_deref(),
                            &latin_face,
                            font_size_pt,
                            bold,
                        );
                    }
                    run_start = idx;
                    current_kind = Some(kind);
                }
                Some(_) => {}
            }
        }

        if let (Some(kind), Some(sub)) = (current_kind, line.get(run_start..)) {
            total_pt += measure_subrun(
                sub,
                kind,
                ko_face.as_deref(),
                &latin_face,
                font_size_pt,
                bold,
            );
        }

        if total_pt.is_finite() {
            total_pt.max(0.0)
        } else {
            0.0
        }
    }
}

impl gridcraft_chart::Measure for KoreanTextMeasurer {
    fn text_width(&self, text: &str, size: f32, bold: bool) -> f32 {
        self.measure_text_pt(text, size, bold)
    }
}

impl TextMeasurer for KoreanTextMeasurer {
    fn measure_text_pt(&self, text: &str, font_size_pt: f32, bold: bool) -> f32 {
        KoreanTextMeasurer::measure_text_pt(self, text, font_size_pt, bold)
    }

    fn line_height_pt(&self, font_size_pt: f32) -> f32 {
        KoreanTextMeasurer::line_height_pt(self, font_size_pt)
    }

    fn wrap_korean_lines(
        &self,
        text: &str,
        max_width_pt: f32,
        font_size_pt: f32,
        bold: bool,
    ) -> Vec<String> {
        KoreanTextMeasurer::wrap_korean_lines(self, text, max_width_pt, font_size_pt, bold)
    }
}

fn covers_korean(face: &FontFace) -> bool {
    KOREAN_PROBE_CHARS.iter().all(|&ch| face.covers(ch))
}

pub(crate) fn default_korean_family() -> Option<&'static str> {
    static RESOLVED: OnceLock<Option<String>> = OnceLock::new();
    RESOLVED
        .get_or_init(|| {
            let db = FontDb::global();
            for &cand in KOREAN_FONT_CANDIDATES {
                if db.has_family(cand) {
                    let face = db.face(cand, "Regular");
                    if covers_korean(&face) {
                        return Some(face.family.clone());
                    }
                }
            }
            if let Some(sys_font) =
                wordcraft_fonts::system_cjk_ui_font_for(false, true, KOREAN_PROBE_CHARS)
            {
                let _ = db.install_font(&sys_font.family, &sys_font.bytes);
                let face = db.face(&sys_font.family, "Regular");
                if covers_korean(&face) {
                    return Some(face.family.clone());
                }
            }
            for cf in wordcraft_fonts::korean_fonts() {
                let face = db.face(cf.family, "Regular");
                if covers_korean(&face) {
                    return Some(face.family.clone());
                }
            }
            None
        })
        .as_deref()
}

fn shape_face_advance_pt(face: &FontFace, sub: &str, font_size_pt: f32, bold: bool) -> Option<f32> {
    if face.upem <= 0.0 {
        return None;
    }
    let glyphs = wordcraft_fonts::shape(face, sub, &[], |c| c);
    if glyphs.is_empty() || glyphs.iter().any(|g| g.gid == 0) {
        return None;
    }
    let total_units: i64 = glyphs.iter().map(|g| i64::from(g.x_advance)).sum();
    let mut pt = ((total_units as f64 / face.upem) * f64::from(font_size_pt)) as f32;
    // Compensate synthetic emboldening when a CJK TTC only supplies a Regular weight face.
    if bold && face.weight < 550.0 {
        pt *= 1.04;
    }
    pt.is_finite().then_some(pt.max(0.0))
}

fn measure_subrun<K: Copy + Eq>(
    sub: &str,
    _kind: K,
    ko_face: Option<&FontFace>,
    latin_face: &Arc<FontFace>,
    font_size_pt: f32,
    bold: bool,
) -> f32 {
    if let Some(kf) = ko_face
        && sub.chars().all(|c| !c.is_control() && kf.covers(c))
        && let Some(pt) = shape_face_advance_pt(kf, sub, font_size_pt, bold)
    {
        return pt;
    }
    if sub.chars().all(|c| !c.is_control() && latin_face.covers(c))
        && let Some(pt) = shape_face_advance_pt(latin_face, sub, font_size_pt, bold)
    {
        return pt;
    }
    sub.chars()
        .map(|c| fallback_char_advance_em(c, bold) * font_size_pt)
        .sum()
}

/// Calibrated fallback advance width in `em` units for headless / WASM environments when a
/// character is not covered by loaded OpenType faces.
#[must_use]
pub fn fallback_char_advance_em(ch: char, bold: bool) -> f32 {
    if ch == '\t' {
        return 1.12;
    }
    if ch.is_control() || is_combining_mark(ch) {
        return 0.0;
    }
    let base_em = if ch == ' ' {
        0.28
    } else if is_hangul(ch) {
        0.95
    } else if wordcraft_fonts::is_cjk(ch) || wordcraft_fonts::is_emoji(ch) {
        1.00
    } else if matches!(
        ch,
        '.' | ',' | ':' | ';' | '!' | '|' | '\'' | 'i' | 'l' | 'I' | '(' | ')' | '[' | ']'
    ) {
        0.34
    } else if matches!(ch, 'W' | 'M' | '@' | '%' | 'm' | 'w') {
        0.78
    } else if ch.is_ascii() {
        0.56
    } else {
        0.65
    };
    base_em * if bold { 1.05 } else { 1.0 }
}

/// Returns `true` if `ch` is a Hangul syllable or Jamo character.
#[must_use]
pub fn is_hangul(ch: char) -> bool {
    matches!(
        ch,
        '\u{1100}'..='\u{11FF}'
            | '\u{3130}'..='\u{318F}'
            | '\u{A960}'..='\u{A97F}'
            | '\u{AC00}'..='\u{D7A3}'
            | '\u{D7B0}'..='\u{D7FF}'
    )
}

/// Returns `true` if `ch` is forbidden from starting a wrapped line under Korean/CJK Kinsoku
/// rules (KS X 1001 / KLREQ 행두 금칙문자: closing brackets, sentence/clause punctuation, unit
/// marks).
#[must_use]
pub fn is_kinsoku_line_start_forbidden(ch: char) -> bool {
    matches!(
        ch,
        ')'
            | ']'
            | '}'
            | '>'
            | '›'
            | '»'
            | '”'
            | '’'
            | '〉'
            | '》'
            | '」'
            | '』'
            | '】'
            | '〕'
            | '〗'
            | '〙'
            | '〛'
            | '）'
            | '］'
            | '｝'
            | ','
            | '.'
            | '!'
            | '?'
            | ':'
            | ';'
            | '·'
            | '…'
            | '‥'
            | '、'
            | '。'
            | '，'
            | '．'
            | '！'
            | '？'
            | '：'
            | '；'
            | '%'
            | '％'
            | '‰'
            | '℃'
            | '℉'
            | '°'
            | '′'
            | '″'
    )
}

/// Returns `true` if `ch` is forbidden from ending a wrapped line under Korean/CJK Kinsoku rules
/// (KS X 1001 / KLREQ 행말 금칙문자: opening brackets/quotes and currency prefixes).
#[must_use]
pub fn is_kinsoku_line_end_forbidden(ch: char) -> bool {
    matches!(
        ch,
        '('
            | '['
            | '{'
            | '<'
            | '‹'
            | '«'
            | '“'
            | '‘'
            | '〈'
            | '《'
            | '「'
            | '『'
            | '【'
            | '〔'
            | '〖'
            | '〘'
            | '〚'
            | '（'
            | '［'
            | '｛'
            | '$'
            | '₩'
            | '￥'
            | '¥'
            | '€'
            | '£'
            | '＄'
            | '￦'
    )
}

fn is_combining_mark(ch: char) -> bool {
    matches!(
        ch,
        '\u{0300}'..='\u{036F}'
            | '\u{1AB0}'..='\u{1AFF}'
            | '\u{1DC0}'..='\u{1DFF}'
            | '\u{20D0}'..='\u{20FF}'
            | '\u{FE00}'..='\u{FE0F}'
            | '\u{FE20}'..='\u{FE2F}'
    )
}

/// Splits a paragraph into whitespace-separated eojeols while fusing stray space-separated
/// Kinsoku punctuation onto its neighbor so whitespace wrapping never orphans forbidden marks.
fn normalize_eojeols(para: &str) -> Vec<String> {
    let mut eojeols: Vec<String> = Vec::new();
    let mut pending_open: Option<String> = None;

    for raw in para.split_whitespace() {
        let token = match pending_open.take() {
            Some(prefix) => format!("{prefix} {raw}"),
            None => raw.to_string(),
        };

        let starts_forbidden = token
            .chars()
            .next()
            .is_some_and(is_kinsoku_line_start_forbidden);
        if starts_forbidden && let Some(prev) = eojeols.last_mut() {
            prev.push(' ');
            prev.push_str(&token);
            continue;
        }

        let all_end_forbidden = !token.is_empty()
            && token.chars().all(is_kinsoku_line_end_forbidden);
        if all_end_forbidden {
            pending_open = Some(token);
            continue;
        }

        eojeols.push(token);
    }

    if let Some(trailing_open) = pending_open {
        if let Some(last) = eojeols.last_mut() {
            last.push(' ');
            last.push_str(&trailing_open);
        } else {
            eojeols.push(trailing_open);
        }
    }

    eojeols
}

/// Splits an overlong eojeol into minimal Kinsoku-compliant character atoms:
/// - `line_end_forbidden` characters (`(`, `[`, `₩`, `“`) fuse forward to the following atom.
/// - `line_start_forbidden` characters (`.`, `,`, `)`, `%`, `!`) fuse backward to the preceding atom.
fn split_kinsoku_atoms(token: &str) -> Vec<String> {
    let mut atoms: Vec<String> = Vec::new();
    let mut pending_prefix = String::new();

    for ch in token.chars() {
        if is_combining_mark(ch) {
            if !pending_prefix.is_empty() {
                pending_prefix.push(ch);
            } else if let Some(last) = atoms.last_mut() {
                last.push(ch);
            } else {
                atoms.push(ch.to_string());
            }
        } else if is_kinsoku_line_end_forbidden(ch) {
            pending_prefix.push(ch);
        } else if is_kinsoku_line_start_forbidden(ch) || ch.is_whitespace() {
            if !pending_prefix.is_empty() {
                pending_prefix.push(ch);
                if let Some(last) = atoms.last_mut() {
                    last.push_str(&pending_prefix);
                    pending_prefix.clear();
                }
            } else if let Some(last) = atoms.last_mut() {
                last.push(ch);
            } else {
                pending_prefix.push(ch);
            }
        } else {
            let mut atom = std::mem::take(&mut pending_prefix);
            atom.push(ch);
            atoms.push(atom);
        }
    }

    if !pending_prefix.is_empty() {
        if let Some(last) = atoms.last_mut() {
            last.push_str(&pending_prefix);
        } else {
            atoms.push(pending_prefix);
        }
    }

    atoms
}

#[cfg(test)]
mod tests {
    use super::*;
    use gridcraft_chart::Measure;

    #[test]
    fn korean_measurement_scales_monotonically_with_size_and_weight() {
        let m = KoreanTextMeasurer::new();
        let w10 = m.measure_text_pt("차세대 클라우드 전환 제안서 (2026)", 10.0, false);
        let w20 = m.measure_text_pt("차세대 클라우드 전환 제안서 (2026)", 20.0, false);
        let w10_bold = m.measure_text_pt("차세대 클라우드 전환 제안서 (2026)", 10.0, true);

        assert!(w10 > 50.0, "expected realistic advance width, got {w10}");
        assert!(w20 > w10 * 1.8 && w20 < w10 * 2.2, "w10={w10}, w20={w20}");
        assert!(w10_bold >= w10, "bold={w10_bold}, regular={w10}");
        assert!((m.text_width("KPI 달성률", 12.0, false) - m.measure_text_pt("KPI 달성률", 12.0, false)).abs() < 1e-5);
    }

    #[test]
    fn korean_eojeol_and_kinsoku_wrapping_prevents_forbidden_line_starts_and_ends() {
        let m = KoreanTextMeasurer::new();
        let sample = "공공 클라우드 전환을 통한 운영비용 절감 효과(연간 35.4%) 달성, 보안성 강화!";
        let lines = m.wrap_korean_lines(sample, 110.0, 11.0, false);
        assert!(lines.len() >= 2, "expected wrapping into multiple lines: {lines:?}");

        for (idx, line) in lines.iter().enumerate() {
            assert!(!line.is_empty(), "line {idx} should not be empty");
            if idx > 0 {
                let first = line.chars().next();
                assert!(
                    first.is_none_or(|c| !is_kinsoku_line_start_forbidden(c)),
                    "line {idx} started with forbidden Kinsoku punctuation: {line:?}"
                );
            }
            let last = line.chars().next_back();
            assert!(
                last.is_none_or(|c| !is_kinsoku_line_end_forbidden(c)),
                "line {idx} ended with forbidden Kinsoku opening mark: {line:?}"
            );
        }

        // Also test stray space before punctuation and overlong compound Korean token.
        let tricky = "초거대생성형인공지능기반클라우드인프라구축사업(AI)달성률35.4%! 결과 , 완료 .";
        let tricky_lines = m.wrap_korean_lines(tricky, 65.0, 12.0, false);
        assert!(tricky_lines.len() >= 3, "{tricky_lines:?}");
        for (idx, line) in tricky_lines.iter().enumerate() {
            if idx > 0 {
                assert!(
                    line.chars().next().is_none_or(|c| !is_kinsoku_line_start_forbidden(c)),
                    "tricky line {idx} started with forbidden char: {line:?}"
                );
            }
            assert!(
                line.chars().next_back().is_none_or(|c| !is_kinsoku_line_end_forbidden(c)),
                "tricky line {idx} ended with forbidden char: {line:?}"
            );
        }
    }

    #[test]
    fn hostile_font_inputs_never_panic() {
        let m = KoreanTextMeasurer::new();
        for bad_size in [0.0, -12.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 1e9] {
            let w = m.measure_text_pt("테스트", bad_size, true);
            assert!(w.is_finite() && w >= 0.0);
            let lh = m.line_height_pt(bad_size);
            assert!(lh.is_finite() && lh >= 0.0);
            let wrapped = m.wrap_korean_lines("가나다 (라) 마바사!", 80.0, bad_size, false);
            assert!(!wrapped.is_empty());
        }
        for bad_width in [0.0, -50.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 0.5] {
            let bbox = m.measure_wrapped_box("예산 절감률 (25%) 달성!", bad_width, 12.0, false);
            assert!(!bbox.lines.is_empty());
            assert!(bbox.width_pt.is_finite() && bbox.height_pt.is_finite());
        }
        assert!(m.wrap_korean_lines("", 100.0, 12.0, false).is_empty());
    }
}
