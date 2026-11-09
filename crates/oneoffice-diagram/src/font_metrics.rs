//! Korean + Latin HarfRust/Skrifa shaping, Kinsoku line-breaking, and vector glyph outlines.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use harfrust::{Direction, ShapeOptions, ShaperData, UnicodeBuffer};
use skrifa::instance::{LocationRef, Size};
use skrifa::outline::{DrawSettings, OutlinePen};
use skrifa::{GlyphId, MetadataProvider};
use unicode_segmentation::UnicodeSegmentation;
use vello_cpu::kurbo::BezPath;
use wordcraft_fonts::{FontDb, FontFace, SystemUiFont};

/// Maximum cached CJK glyph outlines in memory.
const MAX_CJK_OUTLINE_CACHE: usize = 16_384;

/// Horizontal safety padding (in points) reserved when wrapping multi-line text inside cards.
pub const DEFAULT_WRAP_SAFETY_PAD_PT: f32 = 4.0;

/// Rich inline span parsed from `**bold**` markdown markers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RichSpan {
    pub text: String,
    pub bold: bool,
}

/// Parses inline `**bold**` markers into ordered [`RichSpan`] runs without panicking.
#[must_use]
pub fn parse_rich_spans(input: &str) -> Vec<RichSpan> {
    if input.is_empty() {
        return Vec::new();
    }
    let mut spans = Vec::new();
    let mut rest = input;
    let mut bold = false;

    while let Some(pos) = rest.find("**") {
        let before = &rest[..pos];
        let after_marker = &rest[pos + 2..];
        // If opening `**` has no matching closing `**`, keep the rest literal so no text is lost.
        if !bold && !after_marker.contains("**") {
            let mut combined = String::with_capacity(rest.len());
            combined.push_str(before);
            combined.push_str("**");
            combined.push_str(after_marker);
            if !combined.is_empty() {
                spans.push(RichSpan { text: combined, bold: false });
            }
            return spans;
        }
        if !before.is_empty() {
            spans.push(RichSpan {
                text: before.to_owned(),
                bold,
            });
        }
        bold = !bold;
        rest = after_marker;
    }

    if !rest.is_empty() {
        spans.push(RichSpan {
            text: rest.to_owned(),
            bold,
        });
    }
    spans
}

/// Positioned run within a wrapped line.
#[derive(Debug, Clone, PartialEq)]
pub struct MeasuredRun {
    pub text: String,
    pub bold: bool,
    pub width_pt: f32,
}

/// One wrapped line of styled runs with exact measured advance width.
#[derive(Debug, Clone, PartialEq)]
pub struct WrappedLine {
    pub runs: Vec<MeasuredRun>,
    pub width_pt: f32,
}

/// Shaped glyph ready for vector outline rasterization in `compile_png`.
#[derive(Debug, Clone)]
pub enum DrawableGlyph {
    /// Glyph outline from bundled `wordcraft_fonts::FontFace` (`Inter` / `Source Sans 3`).
    Bundled {
        face: Arc<FontFace>,
        gid: u32,
        x_offset_pt: f32,
        y_offset_pt: f32,
        advance_pt: f32,
    },
    /// Glyph outline from system Korean/CJK OpenType font via `skrifa`.
    SystemCjk {
        gid: u32,
        upem: f32,
        x_offset_pt: f32,
        y_offset_pt: f32,
        advance_pt: f32,
    },
    /// Fontless deterministic vector fallback (e.g., on `wasm32` without system CJK fonts).
    FallbackChar {
        ch: char,
        advance_pt: f32,
    },
}

struct LoadedCjkFont {
    sys: SystemUiFont,
    upem: f32,
    shaper: Option<ShaperData>,
    outline_cache: Mutex<HashMap<u32, Arc<BezPath>>>,
}

/// Process-wide font shaping and outline context for `oneoffice-diagram`.
pub struct DiagramFontContext {
    latin_regular: Arc<FontFace>,
    latin_bold: Arc<FontFace>,
    cjk: Option<LoadedCjkFont>,
}

impl DiagramFontContext {
    /// Global singleton initialized once per process.
    pub fn global() -> &'static Self {
        static CTX: OnceLock<DiagramFontContext> = OnceLock::new();
        CTX.get_or_init(Self::init)
    }

    fn init() -> Self {
        let db = FontDb::global();
        let latin_regular = db.face("Inter", "Regular");
        let latin_bold = db.face("Inter", "SemiBold");

        let probe_chars = ['한', '글', '가', '제', '안', '①', '②', '③', '④', '·', '→', '↓'];
        let cjk = wordcraft_fonts::system_cjk_ui_font_for(false, true, &probe_chars).and_then(|sys| {
            let f = skrifa::FontRef::from_index(&sys.bytes, sys.index).ok()?;
            let upem = f
                .metrics(Size::unscaled(), LocationRef::default())
                .units_per_em as f32;
            let upem = if upem.is_finite() && upem > 0.0 {
                upem
            } else {
                1000.0
            };
            let shaper = harfrust::FontRef::from_index(&sys.bytes, sys.index)
                .ok()
                .map(|hb| ShaperData::new(&hb));
            Some(LoadedCjkFont {
                sys,
                upem,
                shaper,
                outline_cache: Mutex::new(HashMap::new()),
            })
        });

        Self {
            latin_regular,
            latin_bold,
            cjk,
        }
    }

    /// Returns whether a real Korean/CJK OpenType font was loaded on this platform.
    #[must_use]
    pub fn has_system_cjk(&self) -> bool {
        self.cjk.is_some()
    }

    /// Shapes `text` into drawable glyphs with exact advances in points.
    #[must_use]
    pub fn shape_text(&self, text: &str, font_size_pt: f32, bold: bool) -> Vec<DrawableGlyph> {
        let size = if font_size_pt.is_finite() && font_size_pt > 0.0 {
            font_size_pt
        } else {
            10.0
        };
        if text.is_empty() {
            return Vec::new();
        }
        let latin_face = if bold {
            &self.latin_bold
        } else {
            &self.latin_regular
        };

        let mut out = Vec::with_capacity(text.chars().count());
        let mut run_start = 0usize;
        let mut current_cjk: Option<bool> = None;

        for (idx, ch) in text.char_indices() {
            let prefer_cjk = prefers_cjk_face(ch, latin_face);
            match current_cjk {
                None => {
                    current_cjk = Some(prefer_cjk);
                    run_start = idx;
                }
                Some(prev) if prev != prefer_cjk => {
                    if let Some(sub) = text.get(run_start..idx) {
                        self.shape_subrun(sub, size, bold, prev, latin_face, &mut out);
                    }
                    current_cjk = Some(prefer_cjk);
                    run_start = idx;
                }
                _ => {}
            }
        }
        if let Some(use_cjk) = current_cjk
            && let Some(sub) = text.get(run_start..)
        {
            self.shape_subrun(sub, size, bold, use_cjk, latin_face, &mut out);
        }
        out
    }

    fn shape_subrun(
        &self,
        sub: &str,
        size_pt: f32,
        bold: bool,
        use_cjk: bool,
        latin_face: &Arc<FontFace>,
        out: &mut Vec<DrawableGlyph>,
    ) {
        if sub.is_empty() {
            return;
        }
        if !use_cjk {
            let upem = (latin_face.upem as f32).max(1.0);
            let k = size_pt / upem;
            let shaped = wordcraft_fonts::shape(latin_face, sub, &[], |c| c);
            if !shaped.is_empty() && shaped.iter().all(|g| g.gid != 0) {
                for g in shaped {
                    out.push(DrawableGlyph::Bundled {
                        face: Arc::clone(latin_face),
                        gid: g.gid,
                        x_offset_pt: g.x_offset as f32 * k,
                        y_offset_pt: g.y_offset as f32 * k,
                        advance_pt: (g.x_advance as f32 * k).max(0.0),
                    });
                }
                return;
            }
        }

        if let Some(cjk) = &self.cjk
            && let Ok(sk_font) = skrifa::FontRef::from_index(&cjk.sys.bytes, cjk.sys.index)
        {
            let cmap = sk_font.charmap();
            let all_mapped = sub.chars().all(|c| c == ' ' || cmap.map(c).is_some());
            let bold_track = if bold { size_pt * 0.025 } else { 0.0 };

            if all_mapped
                && let Some(shaper_data) = &cjk.shaper
                && let Ok(hb_font) = harfrust::FontRef::from_index(&cjk.sys.bytes, cjk.sys.index)
            {
                let shaper = shaper_data.shaper(&hb_font).build();
                let mut buf = UnicodeBuffer::new();
                for (i, ch) in sub.char_indices() {
                    buf.add(ch, i as u32);
                }
                buf.guess_segment_properties();
                buf.set_direction(Direction::LeftToRight);
                let gb = shaper.shape(buf, ShapeOptions::default());
                let k = size_pt / cjk.upem;
                let infos = gb.glyph_infos();
                let positions = gb.glyph_positions();
                if !infos.is_empty() && infos.iter().all(|inf| inf.glyph_id != 0) {
                    for (info, pos) in infos.iter().zip(positions.iter()) {
                        out.push(DrawableGlyph::SystemCjk {
                            gid: info.glyph_id,
                            upem: cjk.upem,
                            x_offset_pt: pos.x_offset as f32 * k,
                            y_offset_pt: pos.y_offset as f32 * k,
                            advance_pt: (pos.x_advance as f32 * k + bold_track).max(0.0),
                        });
                    }
                    return;
                }
            }

            // Per-character Skrifa cmap + fallback for mixed rare symbols
            let gm = sk_font.glyph_metrics(Size::unscaled(), LocationRef::default());
            let k = size_pt / cjk.upem;
            for ch in sub.chars() {
                if let Some(gid) = cmap.map(ch)
                    && gid.to_u32() != 0
                {
                    let adv = gm.advance_width(gid).unwrap_or(cjk.upem) * k + bold_track;
                    out.push(DrawableGlyph::SystemCjk {
                        gid: gid.to_u32(),
                        upem: cjk.upem,
                        x_offset_pt: 0.0,
                        y_offset_pt: 0.0,
                        advance_pt: adv.max(0.0),
                    });
                } else if latin_face.covers(ch) {
                    let gid = latin_face.glyph_for(ch);
                    let lk = size_pt / (latin_face.upem as f32).max(1.0);
                    let adv = latin_face.advance(gid) as f32 * lk;
                    out.push(DrawableGlyph::Bundled {
                        face: Arc::clone(latin_face),
                        gid,
                        x_offset_pt: 0.0,
                        y_offset_pt: 0.0,
                        advance_pt: adv.max(0.0),
                    });
                } else {
                    out.push(DrawableGlyph::FallbackChar {
                        ch,
                        advance_pt: fallback_char_advance(ch, size_pt, bold),
                    });
                }
            }
            return;
        }

        // Fontless environment (e.g. wasm32 without CJK font): bundled Latin where covered, deterministic fallback for Hangul/CJK
        for ch in sub.chars() {
            if latin_face.covers(ch) && !wordcraft_fonts::is_cjk(ch) {
                let gid = latin_face.glyph_for(ch);
                let lk = size_pt / (latin_face.upem as f32).max(1.0);
                let adv = latin_face.advance(gid) as f32 * lk;
                out.push(DrawableGlyph::Bundled {
                    face: Arc::clone(latin_face),
                    gid,
                    x_offset_pt: 0.0,
                    y_offset_pt: 0.0,
                    advance_pt: adv.max(0.0),
                });
            } else {
                out.push(DrawableGlyph::FallbackChar {
                    ch,
                    advance_pt: fallback_char_advance(ch, size_pt, bold),
                });
            }
        }
    }

    /// Measures exact advance width of plain `text` in points.
    #[must_use]
    pub fn measure_plain(&self, text: &str, font_size_pt: f32, bold: bool) -> f32 {
        self.shape_text(text, font_size_pt, bold)
            .iter()
            .map(|g| match g {
                DrawableGlyph::Bundled { advance_pt, .. }
                | DrawableGlyph::SystemCjk { advance_pt, .. }
                | DrawableGlyph::FallbackChar { advance_pt, .. } => *advance_pt,
            })
            .sum()
    }

    /// Measures single-line rich text (with `**bold**` markers stripped/applied).
    #[must_use]
    pub fn measure_rich_single_line(&self, raw: &str, font_size_pt: f32, base_bold: bool) -> f32 {
        parse_rich_spans(raw)
            .iter()
            .map(|s| self.measure_plain(&s.text, font_size_pt, s.bold || base_bold))
            .sum()
    }

    /// Returns cached Y-down flipped outline for `gid` in the system CJK font.
    #[must_use]
    pub fn cjk_outline(&self, gid: u32) -> Option<Arc<BezPath>> {
        let cjk = self.cjk.as_ref()?;
        if let Some(found) = cjk
            .outline_cache
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(&gid)
        {
            return Some(Arc::clone(found));
        }
        let sk_font = skrifa::FontRef::from_index(&cjk.sys.bytes, cjk.sys.index).ok()?;
        let glyph = sk_font.outline_glyphs().get(GlyphId::new(gid))?;
        let mut pen = YFlipOutlinePen {
            path: BezPath::new(),
        };
        glyph
            .draw(
                DrawSettings::unhinted(Size::unscaled(), LocationRef::default()),
                &mut pen,
            )
            .ok()?;
        let arc_path = Arc::new(pen.path);
        let mut cache = cjk
            .outline_cache
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if cache.len() >= MAX_CJK_OUTLINE_CACHE {
            cache.clear();
        }
        cache.insert(gid, Arc::clone(&arc_path));
        Some(arc_path)
    }
}

/// Measures single-line rich text using the process-wide font context.
#[must_use]
pub fn measure_rich_single_line(raw: &str, font_size_pt: f32, base_bold: bool) -> f32 {
    DiagramFontContext::global().measure_rich_single_line(raw, font_size_pt, base_bold)
}

struct YFlipOutlinePen {
    path: BezPath,
}

impl OutlinePen for YFlipOutlinePen {
    fn move_to(&mut self, x: f32, y: f32) {
        self.path.move_to((f64::from(x), f64::from(-y)));
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.path.line_to((f64::from(x), f64::from(-y)));
    }
    fn quad_to(&mut self, cx0: f32, cy0: f32, x: f32, y: f32) {
        self.path.quad_to(
            (f64::from(cx0), f64::from(-cy0)),
            (f64::from(x), f64::from(-y)),
        );
    }
    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        self.path.curve_to(
            (f64::from(cx0), f64::from(-cy0)),
            (f64::from(cx1), f64::from(-cy1)),
            (f64::from(x), f64::from(-y)),
        );
    }
    fn close(&mut self) {
        self.path.close_path();
    }
}

fn prefers_cjk_face(ch: char, latin_face: &FontFace) -> bool {
    if wordcraft_fonts::is_cjk(ch) {
        return true;
    }
    if matches!(
        ch,
        '\u{2460}'..='\u{24FF}'
            | '↑'
            | '↓'
            | '←'
            | '→'
            | '↔'
            | '▸'
            | '▹'
            | '▶'
            | '▷'
            | '◆'
            | '◇'
            | '■'
            | '□'
            | '●'
            | '○'
    ) {
        return true;
    }
    !latin_face.covers(ch)
}

fn fallback_char_advance(ch: char, size_pt: f32, bold: bool) -> f32 {
    let bold_mul = if bold { 1.03 } else { 1.0 };
    if ch == ' ' {
        size_pt * 0.28
    } else if wordcraft_fonts::is_cjk(ch) || matches!(ch, '\u{2460}'..='\u{24FF}') {
        size_pt * bold_mul
    } else {
        size_pt * 0.60 * bold_mul
    }
}

#[derive(Clone, Debug)]
struct StyledGrapheme {
    text: String,
    bold: bool,
    width_pt: f32,
    is_space: bool,
}

#[derive(Clone, Debug)]
struct KinsokuAtom {
    graphemes: Vec<StyledGrapheme>,
    width_pt: f32,
    is_space: bool,
}

impl KinsokuAtom {
    fn from_single(g: StyledGrapheme) -> Self {
        let width_pt = g.width_pt;
        let is_space = g.is_space;
        Self {
            graphemes: vec![g],
            width_pt,
            is_space,
        }
    }

    fn push(&mut self, g: StyledGrapheme) {
        self.width_pt += g.width_pt;
        self.graphemes.push(g);
    }
}

fn is_no_line_start(g: &str) -> bool {
    matches!(
        g,
        ")"
            | "]"
            | "}"
            | "%"
            | "."
            | ","
            | ":"
            | ";"
            | "!"
            | "?"
            | "·"
            | "…"
            | "’"
            | "”"
            | "'"
            | "\""
            | "〉"
            | "》"
            | "」"
            | "』"
            | "~"
            | "℃"
    )
}

fn is_no_line_end(g: &str) -> bool {
    matches!(
        g,
        "(" | "[" | "{" | "‘" | "“" | "〈" | "《" | "「" | "『" | "₩" | "$" | "#"
    )
}

fn is_ascii_word_char(g: &str) -> bool {
    let mut chars = g.chars();
    let Some(c) = chars.next() else {
        return false;
    };
    chars.next().is_none()
        && (c.is_ascii_alphanumeric() || matches!(c, '.' | '+' | '-' | '~' | '/' | '_'))
}

/// Wraps rich text (`**bold**` + `\n`) into lines respecting Korean Kinsoku rules,
/// eojeol boundaries, anti-orphan balancing, and `safety_pad_pt` margin.
#[must_use]
pub fn wrap_rich_text(
    raw_text: &str,
    font_size_pt: f32,
    base_bold: bool,
    inner_width_pt: f32,
    safety_pad_pt: f32,
) -> Vec<WrappedLine> {
    let font_ctx = DiagramFontContext::global();
    let effective_pad = if inner_width_pt > font_size_pt + safety_pad_pt + 2.0 {
        safety_pad_pt.max(0.0)
    } else {
        0.0
    };
    let wrap_limit = (inner_width_pt - effective_pad).max(font_size_pt.max(4.0));

    let mut all_lines = Vec::new();
    for para in raw_text.split('\n') {
        let trimmed = para.trim_end();
        if trimmed.is_empty() {
            all_lines.push(WrappedLine {
                runs: Vec::new(),
                width_pt: 0.0,
            });
            continue;
        }
        let spans = parse_rich_spans(trimmed);
        let mut graphemes = Vec::new();
        for span in spans {
            let bold = span.bold || base_bold;
            for g in span.text.graphemes(true) {
                let is_space = g.chars().all(char::is_whitespace);
                let text = if is_space { " ".to_owned() } else { g.to_owned() };
                let width_pt = font_ctx.measure_plain(&text, font_size_pt, bold);
                graphemes.push(StyledGrapheme {
                    text,
                    bold,
                    width_pt,
                    is_space,
                });
            }
        }
        let atoms = build_kinsoku_atoms(graphemes, wrap_limit);
        let mut para_lines = wrap_atoms_eojeol_first(&atoms, wrap_limit);
        balance_single_char_orphan(&mut para_lines, wrap_limit);

        for atom_line in para_lines {
            all_lines.push(atoms_to_wrapped_line(&atom_line));
        }
    }

    if all_lines.is_empty() {
        all_lines.push(WrappedLine {
            runs: Vec::new(),
            width_pt: 0.0,
        });
    }
    all_lines
}

fn build_kinsoku_atoms(graphemes: Vec<StyledGrapheme>, wrap_limit: f32) -> Vec<KinsokuAtom> {
    let mut atoms: Vec<KinsokuAtom> = Vec::new();
    let mut pending_open: Vec<StyledGrapheme> = Vec::new();

    for g in graphemes {
        if g.is_space {
            for op in pending_open.drain(..) {
                atoms.push(KinsokuAtom::from_single(op));
            }
            if atoms.last().is_none_or(|last| !last.is_space) {
                atoms.push(KinsokuAtom::from_single(g));
            }
            continue;
        }

        if is_no_line_end(&g.text) {
            pending_open.push(g);
            continue;
        }

        if is_no_line_start(&g.text)
            && pending_open.is_empty()
            && let Some(prev) = atoms.last_mut()
            && !prev.is_space
            && prev.width_pt + g.width_pt <= wrap_limit
        {
            prev.push(g);
            continue;
        }

        // Keep contiguous ASCII numbers/acronyms (e.g. "3.4", "68%", "RFP", "53~54") together
        if pending_open.is_empty()
            && is_ascii_word_char(&g.text)
            && let Some(prev) = atoms.last_mut()
            && !prev.is_space
            && prev
                .graphemes
                .last()
                .is_some_and(|lg| is_ascii_word_char(&lg.text))
            && prev.width_pt + g.width_pt <= wrap_limit * 0.75
        {
            prev.push(g);
            continue;
        }

        let mut atom = if let Some(first) = pending_open.first().cloned() {
            let mut a = KinsokuAtom::from_single(first);
            for rest in pending_open.iter().skip(1).cloned() {
                a.push(rest);
            }
            pending_open.clear();
            a.push(g);
            a
        } else {
            KinsokuAtom::from_single(g)
        };

        // If an atom itself somehow exceeded wrap_limit, split it back into individual graphemes
        if atom.width_pt > wrap_limit && atom.graphemes.len() > 1 {
            for sub_g in atom.graphemes.drain(..) {
                atoms.push(KinsokuAtom::from_single(sub_g));
            }
        } else {
            atoms.push(atom);
        }
    }

    for op in pending_open {
        if let Some(prev) = atoms.last_mut()
            && !prev.is_space
            && prev.width_pt + op.width_pt <= wrap_limit
        {
            prev.push(op);
        } else {
            atoms.push(KinsokuAtom::from_single(op));
        }
    }

    atoms
}

fn wrap_atoms_eojeol_first(atoms: &[KinsokuAtom], wrap_limit: f32) -> Vec<Vec<KinsokuAtom>> {
    // Group non-space atoms between space atoms into Korean eojeols
    let mut eojeols: Vec<Vec<KinsokuAtom>> = Vec::new();
    let mut cur_word: Vec<KinsokuAtom> = Vec::new();
    let mut space_proto: Option<KinsokuAtom> = None;

    for a in atoms {
        if a.is_space {
            space_proto = Some(a.clone());
            if !cur_word.is_empty() {
                eojeols.push(std::mem::take(&mut cur_word));
            }
        } else {
            cur_word.push(a.clone());
        }
    }
    if !cur_word.is_empty() {
        eojeols.push(cur_word);
    }

    let mut lines: Vec<Vec<KinsokuAtom>> = Vec::new();
    let mut current: Vec<KinsokuAtom> = Vec::new();
    let mut current_w = 0.0_f32;

    for word in eojeols {
        let word_w: f32 = word.iter().map(|a| a.width_pt).sum();
        let sp_w = if current.is_empty() {
            0.0
        } else {
            space_proto.as_ref().map_or(3.0, |s| s.width_pt)
        };

        if !current.is_empty() && current_w + sp_w + word_w <= wrap_limit {
            if let Some(sp) = &space_proto {
                current.push(sp.clone());
                current_w += sp.width_pt;
            }
            current.extend(word);
            current_w += word_w;
        } else if word_w <= wrap_limit {
            if !current.is_empty() {
                trim_trailing_spaces(&mut current);
                lines.push(std::mem::take(&mut current));
            }
            current_w = word_w;
            current = word;
        } else {
            // Single eojeol exceeds wrap_limit: break cleanly at KinsokuAtom boundaries
            if !current.is_empty() {
                if let Some(first_atom) = word.first()
                    && current_w + sp_w + first_atom.width_pt <= wrap_limit
                    && let Some(sp) = &space_proto
                {
                    current.push(sp.clone());
                    current_w += sp.width_pt;
                } else {
                    trim_trailing_spaces(&mut current);
                    lines.push(std::mem::take(&mut current));
                    current_w = 0.0;
                }
            }
            for atom in word {
                if !current.is_empty() && current_w + atom.width_pt > wrap_limit {
                    trim_trailing_spaces(&mut current);
                    lines.push(std::mem::take(&mut current));
                    current_w = 0.0;
                }
                if atom.width_pt > wrap_limit && atom.graphemes.len() > 1 {
                    for g in atom.graphemes {
                        if !current.is_empty() && current_w + g.width_pt > wrap_limit {
                            trim_trailing_spaces(&mut current);
                            lines.push(std::mem::take(&mut current));
                            current_w = 0.0;
                        }
                        current_w += g.width_pt;
                        current.push(KinsokuAtom::from_single(g));
                    }
                } else {
                    current_w += atom.width_pt;
                    current.push(atom);
                }
            }
        }
    }

    if !current.is_empty() {
        trim_trailing_spaces(&mut current);
        lines.push(current);
    }
    lines
}

fn trim_trailing_spaces(line: &mut Vec<KinsokuAtom>) {
    while line.last().is_some_and(|a| a.is_space) {
        line.pop();
    }
}

fn balance_single_char_orphan(lines: &mut [Vec<KinsokuAtom>], wrap_limit: f32) {
    if lines.len() < 2 {
        return;
    }
    let last_idx = lines.len() - 1;
    let prev_idx = last_idx - 1;

    let core_chars: usize = lines[last_idx]
        .iter()
        .flat_map(|a| &a.graphemes)
        .filter(|g| !g.is_space && !is_no_line_start(&g.text) && !is_no_line_end(&g.text))
        .count();

    if core_chars > 1 {
        return;
    }

    let non_space_prev = lines[prev_idx].iter().filter(|a| !a.is_space).count();
    if non_space_prev < 3 {
        return;
    }

    let Some(candidate) = lines[prev_idx].last().cloned() else {
        return;
    };
    if candidate.is_space {
        return;
    }
    let last_w: f32 = lines[last_idx].iter().map(|a| a.width_pt).sum();
    let sep_w = lines[prev_idx]
        .iter()
        .rev()
        .nth(1)
        .filter(|a| a.is_space)
        .map_or(0.0, |s| s.width_pt);

    if candidate.width_pt + sep_w + last_w <= wrap_limit {
        lines[prev_idx].pop();
        if sep_w > 0.0
            && let Some(sp) = lines[prev_idx].pop()
        {
            lines[last_idx].insert(0, sp);
        }
        lines[last_idx].insert(0, candidate);
        trim_trailing_spaces(&mut lines[prev_idx]);
    }
}

fn atoms_to_wrapped_line(atoms: &[KinsokuAtom]) -> WrappedLine {
    let mut runs: Vec<MeasuredRun> = Vec::new();
    let mut total_w = 0.0_f32;

    for atom in atoms {
        for g in &atom.graphemes {
            total_w += g.width_pt;
            if let Some(last) = runs.last_mut()
                && last.bold == g.bold
            {
                last.text.push_str(&g.text);
                last.width_pt += g.width_pt;
            } else {
                runs.push(MeasuredRun {
                    text: g.text.clone(),
                    bold: g.bold,
                    width_pt: g.width_pt,
                });
            }
        }
    }

    WrappedLine {
        runs,
        width_pt: total_w,
    }
}

/// Builds a crisp geometric vector outline in `[0, adv] × [-0.82*size, 0.14*size]` (Y-down around baseline `0.0`)
/// for fontless environments (such as `wasm32` without system Korean fonts).
#[must_use]
pub fn build_fallback_glyph_path(ch: char, size_pt: f32, adv_pt: f32) -> (BezPath, bool) {
    let mut p = BezPath::new();
    if ch.is_whitespace() {
        return (p, false);
    }
    let s = f64::from(size_pt);
    let w = f64::from(adv_pt.max(size_pt * 0.4));
    let top = -0.78 * s;
    let bot = 0.06 * s;
    let h = bot - top;

    // Circled numbers ①..⑳
    if ('\u{2460}'..='\u{2473}').contains(&ch) {
        let cx = w * 0.5;
        let cy = top + h * 0.5;
        let r = (w.min(h) * 0.44).max(1.5);
        // Circle approximation via 4 cubic arcs
        let k = 0.552_284_75 * r;
        p.move_to((cx, cy - r));
        p.curve_to((cx + k, cy - r), (cx + r, cy - k), (cx + r, cy));
        p.curve_to((cx + r, cy + k), (cx + k, cy + r), (cx, cy + r));
        p.curve_to((cx - k, cy + r), (cx - r, cy + k), (cx - r, cy));
        p.curve_to((cx - r, cy - k), (cx - k, cy - r), (cx, cy - r));
        p.close_path();
        p.move_to((cx, cy - r * 0.42));
        p.line_to((cx, cy + r * 0.42));
        return (p, true);
    }

    // Arrows
    match ch {
        '↓' => {
            let cx = w * 0.5;
            p.move_to((cx, top + h * 0.12));
            p.line_to((cx, bot - h * 0.16));
            p.move_to((cx - w * 0.24, bot - h * 0.36));
            p.line_to((cx, bot - h * 0.12));
            p.line_to((cx + w * 0.24, bot - h * 0.36));
            return (p, true);
        }
        '→' => {
            let cy = top + h * 0.5;
            p.move_to((w * 0.12, cy));
            p.line_to((w * 0.86, cy));
            p.move_to((w * 0.62, cy - h * 0.22));
            p.line_to((w * 0.88, cy));
            p.line_to((w * 0.62, cy + h * 0.22));
            return (p, true);
        }
        '·' | '•' => {
            let cx = w * 0.5;
            let cy = top + h * 0.5;
            let r = (s * 0.10).max(0.8);
            p.move_to((cx - r, cy - r));
            p.line_to((cx + r, cy - r));
            p.line_to((cx + r, cy + r));
            p.line_to((cx - r, cy + r));
            p.close_path();
            return (p, false);
        }
        _ => {}
    }

    // Decompose Hangul syllables U+AC00..=U+D7A3 into clean geometric Jamo strokes
    let cp = ch as u32;
    if (0xAC00..=0xD7A3).contains(&cp) {
        let idx = cp - 0xAC00;
        let cho = idx / (21 * 28);
        let jung = (idx / 28) % 21;
        let jong = idx % 28;

        let x0 = w * 0.12;
        let x1 = w * 0.88;
        let y0 = top + h * 0.08;
        let y1 = bot - h * 0.08;
        let has_jong = jong != 0;
        let vert_vowel = matches!(jung, 0..=7 | 20);

        let (cho_box, jung_box) = if vert_vowel {
            let cy_end = if has_jong { y0 + (y1 - y0) * 0.60 } else { y1 };
            (
                (x0, y0, x0 + (x1 - x0) * 0.48, cy_end),
                (x0 + (x1 - x0) * 0.56, y0, x1, cy_end),
            )
        } else {
            let cho_bottom = if has_jong {
                y0 + (y1 - y0) * 0.36
            } else {
                y0 + (y1 - y0) * 0.50
            };
            let jung_bottom = if has_jong {
                y0 + (y1 - y0) * 0.64
            } else {
                y1
            };
            (
                (x0 + (x1 - x0) * 0.12, y0, x1 - (x1 - x0) * 0.12, cho_bottom),
                (x0, cho_bottom + (y1 - y0) * 0.08, x1, jung_bottom),
            )
        };

        draw_hangul_consonant(&mut p, cho, cho_box);
        draw_hangul_vowel(&mut p, jung, jung_box);
        if has_jong {
            let jong_box = (x0 + (x1 - x0) * 0.14, y0 + (y1 - y0) * 0.70, x1 - (x1 - x0) * 0.14, y1);
            draw_hangul_consonant(&mut p, jong % 19, jong_box);
        }
        return (p, true);
    }

    // Generic geometric character frame for rare uncovered codepoints
    let x0 = w * 0.16;
    let x1 = w * 0.84;
    let y0 = top + h * 0.16;
    let y1 = bot - h * 0.14;
    p.move_to((x0, y0));
    p.line_to((x1, y0));
    p.line_to((x1, y1));
    p.line_to((x0, y1));
    p.close_path();
    (p, true)
}

fn draw_hangul_consonant(p: &mut BezPath, idx: u32, (x0, y0, x1, y1): (f64, f64, f64, f64)) {
    let xm = (x0 + x1) * 0.5;
    let ym = (y0 + y1) * 0.5;
    match idx % 19 {
        0 | 1 | 15 => {
            // ㄱ / ㄲ / ㅋ
            p.move_to((x0, y0));
            p.line_to((x1, y0));
            p.line_to((x1, y1));
            if idx % 19 == 15 {
                p.move_to((x0, ym));
                p.line_to((x1, ym));
            }
        }
        2 => {
            // ㄴ
            p.move_to((x0, y0));
            p.line_to((x0, y1));
            p.line_to((x1, y1));
        }
        3 | 4 | 16 => {
            // ㄷ / ㄸ / ㅌ
            p.move_to((x1, y0));
            p.line_to((x0, y0));
            p.line_to((x0, y1));
            p.line_to((x1, y1));
            if idx % 19 == 16 {
                p.move_to((x0, ym));
                p.line_to((x1, ym));
            }
        }
        5 => {
            // ㄹ
            p.move_to((x0, y0));
            p.line_to((x1, y0));
            p.line_to((x1, ym));
            p.line_to((x0, ym));
            p.line_to((x0, y1));
            p.line_to((x1, y1));
        }
        6 | 17 => {
            // ㅁ / ㅍ
            p.move_to((x0, y0));
            p.line_to((x1, y0));
            p.line_to((x1, y1));
            p.line_to((x0, y1));
            p.close_path();
        }
        7 | 8 => {
            // ㅂ / ㅃ
            p.move_to((x0, y0));
            p.line_to((x0, y1));
            p.line_to((x1, y1));
            p.line_to((x1, y0));
            p.move_to((x0, ym));
            p.line_to((x1, ym));
        }
        9 | 10 => {
            // ㅅ / ㅆ
            p.move_to((xm, y0));
            p.line_to((x0, y1));
            p.move_to((xm, y0 + (y1 - y0) * 0.25));
            p.line_to((x1, y1));
        }
        11 | 18 => {
            // ㅇ / ㅎ
            let rx = ((x1 - x0) * 0.45).max(0.8);
            let ry = ((y1 - y0) * 0.42).max(0.8);
            let cy = if idx % 19 == 18 {
                y0 + (y1 - y0) * 0.62
            } else {
                ym
            };
            if idx % 19 == 18 {
                p.move_to((x0, y0 + (y1 - y0) * 0.16));
                p.line_to((x1, y0 + (y1 - y0) * 0.16));
            }
            let kx = 0.552_284_75 * rx;
            let ky = 0.552_284_75 * ry;
            p.move_to((xm, cy - ry));
            p.curve_to((xm + kx, cy - ry), (xm + rx, cy - ky), (xm + rx, cy));
            p.curve_to((xm + rx, cy + ky), (xm + kx, cy + ry), (xm, cy + ry));
            p.curve_to((xm - kx, cy + ry), (xm - rx, cy + ky), (xm - rx, cy));
            p.curve_to((xm - rx, cy - ky), (xm - kx, cy - ry), (xm, cy - ry));
            p.close_path();
        }
        _ => {
            // ㅈ / ㅉ / ㅊ
            p.move_to((x0, y0));
            p.line_to((x1, y0));
            p.move_to((xm, y0));
            p.line_to((x0, y1));
            p.move_to((xm, y0 + (y1 - y0) * 0.25));
            p.line_to((x1, y1));
        }
    }
}

fn draw_hangul_vowel(p: &mut BezPath, idx: u32, (x0, y0, x1, y1): (f64, f64, f64, f64)) {
    let xm = (x0 + x1) * 0.5;
    let ym = (y0 + y1) * 0.5;
    match idx % 21 {
        0..=3 => {
            // ㅏ ㅐ ㅑ ㅒ
            p.move_to((x0 + (x1 - x0) * 0.2, y0));
            p.line_to((x0 + (x1 - x0) * 0.2, y1));
            p.move_to((x0 + (x1 - x0) * 0.2, ym));
            p.line_to((x1, ym));
        }
        4..=7 => {
            // ㅓ ㅔ ㅕ ㅖ
            p.move_to((x1 - (x1 - x0) * 0.2, y0));
            p.line_to((x1 - (x1 - x0) * 0.2, y1));
            p.move_to((x0, ym));
            p.line_to((x1 - (x1 - x0) * 0.2, ym));
        }
        8..=12 => {
            // ㅗ ㅘ ㅙ ㅚ ㅛ
            p.move_to((x0, y1 - (y1 - y0) * 0.2));
            p.line_to((x1, y1 - (y1 - y0) * 0.2));
            p.move_to((xm, y0));
            p.line_to((xm, y1 - (y1 - y0) * 0.2));
        }
        13..=17 => {
            // ㅜ ㅝ ㅞ ㅟ ㅠ
            p.move_to((x0, y0 + (y1 - y0) * 0.2));
            p.line_to((x1, y0 + (y1 - y0) * 0.2));
            p.move_to((xm, y0 + (y1 - y0) * 0.2));
            p.line_to((xm, y1));
        }
        18 => {
            // ㅡ
            p.move_to((x0, ym));
            p.line_to((x1, ym));
        }
        19 => {
            // ㅣ
            p.move_to((xm, y0));
            p.line_to((xm, y1));
        }
        _ => {
            // ㅢ
            p.move_to((x0, y1 - (y1 - y0) * 0.2));
            p.line_to((x1 - (x1 - x0) * 0.25, y1 - (y1 - y0) * 0.2));
            p.move_to((x1, y0));
            p.line_to((x1, y1));
        }
    }
}
