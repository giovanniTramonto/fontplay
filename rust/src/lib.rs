use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use skrifa::{
    instance::LocationRef,
    outline::{DrawSettings, OutlinePen},
    prelude::Size,
    FontRef, MetadataProvider,
};
use read_fonts::{types::Tag as ReadTag, TableProvider};
use wasm_bindgen::prelude::*;
use woff2::convert_woff2_to_ttf;

#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
}

// ─── Data types ──────────────────────────────────────────────────────────────

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FontInfo {
    pub units_per_em: u16,
    pub ascender: i16,
    pub descender: i16,
    pub glyph_count: u16,
}

// ─── Font normalization ───────────────────────────────────────────────────────

/// If `data` is a WOFF2 file, decompress it to sfnt bytes. Otherwise return
/// a cheap clone (Cow-like) so callers always get an owned slice.
fn to_sfnt(data: &[u8]) -> Result<Vec<u8>, String> {
    // WOFF2 magic: 0x774F4632 ('wOF2')
    if data.len() >= 4 && &data[0..4] == b"wOF2" {
        let mut buf = data;
        convert_woff2_to_ttf(&mut buf).map_err(|e| format!("WOFF2 decode: {e:?}"))
    // WOFF1 magic: 0x774F4646 ('wOFF') — not supported
    } else if data.len() >= 4 && &data[0..4] == b"wOFF" {
        Err("WOFF1 is not supported. Please convert to TTF, OTF, or WOFF2 first.".to_string())
    } else {
        Ok(data.to_vec())
    }
}

// ─── WASM exports ────────────────────────────────────────────────────────────

/// Returns basic font metrics as a JS object.
#[wasm_bindgen]
pub fn get_font_info(data: &[u8]) -> Result<JsValue, JsValue> {
    let sfnt = to_sfnt(data).map_err(|e| JsValue::from_str(&e))?;
    let data = sfnt.as_slice();
    let font = FontRef::new(data).map_err(|e| JsValue::from_str(&e.to_string()))?;

    let head = font.head().map_err(|e| JsValue::from_str(&e.to_string()))?;
    let hhea = font.hhea().map_err(|e| JsValue::from_str(&e.to_string()))?;
    let maxp = font.maxp().map_err(|e| JsValue::from_str(&e.to_string()))?;

    let info = FontInfo {
        units_per_em: head.units_per_em(),
        ascender: hhea.ascender().to_i16(),
        descender: hhea.descender().to_i16(),
        glyph_count: maxp.num_glyphs(),
    };

    serde_wasm_bindgen::to_value(&info).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Apply geometric transforms and COLR effects to every glyph in the font.
/// Always outputs a TrueType (glyf-based) font, regardless of input format.
/// Accepts TTF, OTF, and WOFF2.
#[wasm_bindgen]
pub fn style_font(font_data: &[u8], request_json: &str) -> Result<Vec<u8>, JsValue> {
    let sfnt = to_sfnt(font_data).map_err(|e| JsValue::from_str(&e))?;
    let req: StyleFontRequest =
        serde_json::from_str(request_json).map_err(|e| JsValue::from_str(&e.to_string()))?;
    style_font_internal(&sfnt, req).map_err(|e| JsValue::from_str(&e))
}


// ─── Request types ────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct StyleFontRequest {
    transforms: Vec<Transform>,
    colr: ColrInput,
    #[serde(default)]
    mood: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BlendCanvasRequest {
    blend_factor: f32,
    char_codes: Vec<u32>,
    bitmap_size: u32,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum Transform {
    ScaleX { factor: f32 },
    ScaleY { factor: f32 },
    Shear { angle: f32 },
    Jitter { amplitude: f32 },
    Wave { amplitude: f32, frequency: f32 },
    WaveY { amplitude: f32, frequency: f32 },
    Rotate { angle: f32 },
    Perspective { depth: f32 },
    Arch { amplitude: f32 },
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct ColrInput {
    #[serde(default)]
    effects: Vec<String>,
    fill_color: Option<String>,
    gradient_colors: Option<Vec<String>>,
    outline_color: Option<String>,
    block_color: Option<String>,
}

// ─── LCG random (matches JS seeded()) ────────────────────────────────────────

struct Lcg(u32);

impl Lcg {
    fn new(seed: u32) -> Self {
        Self(seed)
    }
    fn next(&mut self) -> f32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (self.0 as f64 / 4_294_967_295.0 * 2.0 - 1.0) as f32
    }
}

// ─── Point transform ─────────────────────────────────────────────────────────

fn transform_point(
    x: f64, y: f64,
    transforms: &[Transform],
    rand: &mut Lcg,
    cx: f64, cy: f64,
    upem: f64,
) -> (f64, f64) {
    let mut nx = x;
    let mut ny = y;
    for t in transforms {
        match t {
            Transform::ScaleX { factor } => nx *= *factor as f64,
            Transform::ScaleY { factor } => ny *= *factor as f64,
            Transform::Shear { angle } => {
                nx += ny * (*angle as f64 * std::f64::consts::PI / 180.0).tan();
            }
            Transform::Jitter { amplitude } => {
                nx += rand.next() as f64 * *amplitude as f64;
                ny += rand.next() as f64 * *amplitude as f64;
            }
            Transform::Wave { amplitude, frequency } => {
                nx += (ny * *frequency as f64).sin() * *amplitude as f64;
            }
            Transform::WaveY { amplitude, frequency } => {
                ny += (nx * *frequency as f64).sin() * *amplitude as f64;
            }
            Transform::Rotate { angle } => {
                let theta = *angle as f64 * std::f64::consts::PI / 180.0;
                let dx = nx - cx;
                let dy = ny - cy;
                nx = cx + dx * theta.cos() - dy * theta.sin();
                ny = cy + dx * theta.sin() + dy * theta.cos();
            }
            Transform::Perspective { depth } => {
                // trapezoid: x scales with y-height; y=0 baseline unchanged, y=upem fully scaled
                let t = ny / upem.max(1.0);
                nx *= 1.0 + *depth as f64 * t;
            }
            Transform::Arch { amplitude } => {
                // parabolic vertical bow: peaks at x=cx, falls off toward edges
                let dx = (nx - cx) / (upem * 0.5).max(1.0);
                ny += *amplitude as f64 * (1.0 - dx * dx);
            }
        }
    }
    (nx, ny)
}

// ─── Glyph collection pen ────────────────────────────────────────────────────

use kurbo::{BezPath, CubicBez, PathEl, Point, Shape};

struct CollectPen {
    path: BezPath,
    cur: Point,
}

impl CollectPen {
    fn new() -> Self {
        Self { path: BezPath::new(), cur: Point::ZERO }
    }
}

impl OutlinePen for CollectPen {
    fn move_to(&mut self, x: f32, y: f32) {
        let p = Point::new(x as f64, y as f64);
        self.path.move_to(p);
        self.cur = p;
    }
    fn line_to(&mut self, x: f32, y: f32) {
        let p = Point::new(x as f64, y as f64);
        self.path.line_to(p);
        self.cur = p;
    }
    fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
        let p1 = Point::new(cx as f64, cy as f64);
        let p2 = Point::new(x as f64, y as f64);
        self.path.quad_to(p1, p2);
        self.cur = p2;
    }
    fn curve_to(&mut self, cx1: f32, cy1: f32, cx2: f32, cy2: f32, x: f32, y: f32) {
        let p1 = Point::new(cx1 as f64, cy1 as f64);
        let p2 = Point::new(cx2 as f64, cy2 as f64);
        let p3 = Point::new(x as f64, y as f64);
        self.path.curve_to(p1, p2, p3);
        self.cur = p3;
    }
    fn close(&mut self) {
        self.path.close_path();
    }
}

// ─── Path helpers ─────────────────────────────────────────────────────────────

fn apply_transforms_to_path(path: &BezPath, transforms: &[Transform], seed: u32, upem: f64) -> BezPath {
    if transforms.is_empty() {
        return path.clone();
    }
    let bbox = path.bounding_box();
    let cx = (bbox.x0 + bbox.x1) * 0.5;
    let cy = (bbox.y0 + bbox.y1) * 0.5;
    let mut rand = Lcg::new(seed);
    let mut out = BezPath::new();
    for el in path.iter() {
        match el {
            PathEl::MoveTo(p) => {
                let (nx, ny) = transform_point(p.x, p.y, transforms, &mut rand, cx, cy, upem);
                out.move_to(Point::new(nx, ny));
            }
            PathEl::LineTo(p) => {
                let (nx, ny) = transform_point(p.x, p.y, transforms, &mut rand, cx, cy, upem);
                out.line_to(Point::new(nx, ny));
            }
            PathEl::QuadTo(p1, p2) => {
                let (x1, y1) = transform_point(p1.x, p1.y, transforms, &mut rand, cx, cy, upem);
                let (x2, y2) = transform_point(p2.x, p2.y, transforms, &mut rand, cx, cy, upem);
                out.quad_to(Point::new(x1, y1), Point::new(x2, y2));
            }
            PathEl::CurveTo(p1, p2, p3) => {
                let (x1, y1) = transform_point(p1.x, p1.y, transforms, &mut rand, cx, cy, upem);
                let (x2, y2) = transform_point(p2.x, p2.y, transforms, &mut rand, cx, cy, upem);
                let (x3, y3) = transform_point(p3.x, p3.y, transforms, &mut rand, cx, cy, upem);
                out.curve_to(Point::new(x1, y1), Point::new(x2, y2), Point::new(x3, y3));
            }
            PathEl::ClosePath => {
                out.close_path();
            }
        }
    }
    out
}

/// Convert any cubic bezier segments to quadratics so the path fits in glyf.
fn cubics_to_quads(path: &BezPath) -> BezPath {
    let mut out = BezPath::new();
    let mut cur = Point::ZERO;
    for el in path.iter() {
        match el {
            PathEl::MoveTo(p) => {
                out.move_to(p);
                cur = p;
            }
            PathEl::LineTo(p) => {
                out.line_to(p);
                cur = p;
            }
            PathEl::QuadTo(p1, p2) => {
                out.quad_to(p1, p2);
                cur = p2;
            }
            PathEl::CurveTo(p1, p2, p3) => {
                let cubic = CubicBez::new(cur, p1, p2, p3);
                for (_t0, _t1, quad) in cubic.to_quads(0.5) {
                    out.quad_to(quad.p1, quad.p2);
                }
                cur = p3;
            }
            PathEl::ClosePath => {
                out.close_path();
            }
        }
    }
    out
}

// ─── Font table helpers ───────────────────────────────────────────────────────

use write_fonts::{
    dump_table, FontBuilder,
    tables::{
        colr::{
            BaseGlyph, BaseGlyphList, BaseGlyphPaint, ColorLine, ColorStop, Colr, Extend, Layer,
            LayerList, Paint, PaintColrLayers, PaintGlyph, PaintLinearGradient, PaintSolid,
            PaintTranslate,
        },
        cpal::{ColorRecord, Cpal},
        glyf::{Glyph as WriteGlyph, GlyfLocaBuilder, SimpleGlyph},
        hmtx::{Hmtx, LongMetric},
        maxp::Maxp,
    },
    types::{F2Dot14, FWord, GlyphId16, Tag},
};

/// Copy all tables from `font` into `builder`, except those whose 4-byte tag is in `skip`.
/// Uses `FontRef::table_data()` (the same validated accessor skrifa uses internally) instead
/// of manually slicing raw bytes, so WOFF2-decoded SFNTs with unusual table layouts are
/// handled correctly.
fn copy_tables_except(builder: &mut FontBuilder, font_data: &[u8], font: &FontRef, skip: &[[u8; 4]]) {
    if font_data.len() < 12 {
        return;
    }
    let num_tables = u16::from_be_bytes([font_data[4], font_data[5]]) as usize;
    for i in 0..num_tables {
        let rec = 12 + i * 16;
        if rec + 16 > font_data.len() {
            break;
        }
        let tag_bytes: [u8; 4] = font_data[rec..rec + 4].try_into().unwrap_or([0; 4]);
        if skip.contains(&tag_bytes) {
            continue;
        }
        if let Some(data) = font.table_data(ReadTag::new(&tag_bytes)) {
            builder.add_raw(Tag::new(&tag_bytes), data.as_bytes().to_vec());
        }
    }
}

/// Rebuild the `name` table with the mood injected into:
/// - nameID 4  (Full name)       → "Family Mood"
/// - nameID 6  (PostScript name) → "Family-Mood"
/// - nameID 17 (Typographic Subfamily) → "Mood"  (added / replaced)
///
/// nameID 2 (Subfamily) is intentionally left as-is ("Regular" / "Bold" etc.)
/// because the OpenType spec restricts it to those four values and Font Book
/// validates this strictly.
///
/// Strategy: rebuild the table from scratch so count and stringOffset are always
/// consistent. Records are sorted by (platformID, encodingID, languageID, nameID)
/// as required by the spec.
fn patch_name_table(name_bytes: &[u8], mood: &str) -> Vec<u8> {
    if name_bytes.len() < 6 {
        return name_bytes.to_vec();
    }
    let count = u16::from_be_bytes([name_bytes[2], name_bytes[3]]) as usize;
    let string_offset = u16::from_be_bytes([name_bytes[4], name_bytes[5]]) as usize;
    if 6 + count * 12 > name_bytes.len() || string_offset > name_bytes.len() {
        return name_bytes.to_vec();
    }

    // Capitalise first letter (moods arrive lowercase from JS)
    let mood_cap: String = {
        let mut chars = mood.chars();
        match chars.next() {
            None => String::new(),
            Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
        }
    };

    // UTF-16 BE encode for platform 3 (Windows)
    let utf16be = |s: &str| -> Vec<u8> {
        s.encode_utf16().flat_map(|c| c.to_be_bytes()).collect()
    };

    // Helper: read a Windows (platform 3, encoding 1) name string
    let read_win = |name_id: u16| -> Option<Vec<u8>> {
        for i in 0..count {
            let rec = 6 + i * 12;
            let pid = u16::from_be_bytes([name_bytes[rec], name_bytes[rec + 1]]);
            let eid = u16::from_be_bytes([name_bytes[rec + 2], name_bytes[rec + 3]]);
            let nid = u16::from_be_bytes([name_bytes[rec + 6], name_bytes[rec + 7]]);
            if pid == 3 && eid == 1 && nid == name_id {
                let len = u16::from_be_bytes([name_bytes[rec + 8], name_bytes[rec + 9]]) as usize;
                let off = u16::from_be_bytes([name_bytes[rec + 10], name_bytes[rec + 11]]) as usize;
                let start = string_offset + off;
                if start + len <= name_bytes.len() {
                    return Some(name_bytes[start..start + len].to_vec());
                }
            }
        }
        None
    };

    // Decode UTF-16 BE → String
    let decode_utf16be = |bytes: &[u8]| -> String {
        bytes
            .chunks(2)
            .filter(|c| c.len() == 2)
            .filter_map(|c| char::from_u32(u16::from_be_bytes([c[0], c[1]]) as u32))
            .collect()
    };

    // Read existing values to append to (not overwrite)
    // For nameID 4: existing full name e.g. "Inter Bold" → "Inter Bold Playful"
    // For nameID 6: existing PS name e.g. "Inter-Bold"  → "Inter-Bold-Playful"
    // For nameID 17: existing typographic subfamily e.g. "Bold" → "Bold Playful"
    //                (falls back to nameID 2 if 17 absent)
    let existing_full = read_win(4)
        .map(|b| decode_utf16be(&b))
        .filter(|s| !s.is_empty());
    let existing_ps = read_win(6)
        .map(|b| decode_utf16be(&b))
        .filter(|s| !s.is_empty());
    let existing_subfamily = read_win(17)
        .or_else(|| read_win(2))
        .map(|b| decode_utf16be(&b))
        .filter(|s| !s.is_empty());

    let full_name = match existing_full {
        Some(ref s) => format!("{} {}", s, mood_cap),
        None => mood_cap.clone(),
    };
    let ps_name = match existing_ps {
        Some(ref s) => format!("{}-{}", s, mood_cap.replace(' ', "")),
        None => mood_cap.clone(),
    };
    let typographic_subfamily = match existing_subfamily {
        Some(ref s) => format!("{} {}", s, mood_cap),
        None => mood_cap.clone(),
    };

    // nameIDs to replace/add (platform 3, encoding 1, lang 0x0409)
    // nameID 2 is intentionally excluded — kept as "Regular" / "Bold" etc.
    let overrides: &[(u16, Vec<u8>)] = &[
        (4, utf16be(&full_name)),              // Full name
        (6, utf16be(&ps_name)),                // PostScript name
        (17, utf16be(&typographic_subfamily)), // Typographic Subfamily
    ];
    let override_ids: &[u16] = &[4, 6, 17];

    // Helper: read a Mac (platform 1, encoding 0) name string as raw bytes
    let read_mac = |name_id: u16| -> Option<Vec<u8>> {
        for i in 0..count {
            let rec = 6 + i * 12;
            let pid = u16::from_be_bytes([name_bytes[rec], name_bytes[rec + 1]]);
            let eid = u16::from_be_bytes([name_bytes[rec + 2], name_bytes[rec + 3]]);
            let nid = u16::from_be_bytes([name_bytes[rec + 6], name_bytes[rec + 7]]);
            if pid == 1 && eid == 0 && nid == name_id {
                let len = u16::from_be_bytes([name_bytes[rec + 8], name_bytes[rec + 9]]) as usize;
                let off = u16::from_be_bytes([name_bytes[rec + 10], name_bytes[rec + 11]]) as usize;
                let start = string_offset + off;
                if start + len <= name_bytes.len() {
                    return Some(name_bytes[start..start + len].to_vec());
                }
            }
        }
        None
    };

    // Build Mac variants by appending mood (Mac Roman = ASCII for mood strings)
    // Mac strings are raw bytes; appending " Mood" is safe for ASCII moods
    let mac_full = {
        let base = read_mac(4)
            .map(|b| String::from_utf8_lossy(&b).into_owned())
            .filter(|s| !s.is_empty());
        match base {
            Some(ref s) => format!("{} {}", s, mood_cap),
            None => full_name.clone(),
        }
    };
    let mac_ps = {
        let base = read_mac(6)
            .map(|b| String::from_utf8_lossy(&b).into_owned())
            .filter(|s| !s.is_empty());
        match base {
            Some(ref s) => format!("{}-{}", s, mood_cap.replace(' ', "")),
            None => ps_name.clone(),
        }
    };
    let mac_subfamily = {
        let base = read_mac(17)
            .or_else(|| read_mac(2))
            .map(|b| String::from_utf8_lossy(&b).into_owned())
            .filter(|s| !s.is_empty());
        match base {
            Some(ref s) => format!("{} {}", s, mood_cap),
            None => typographic_subfamily.clone(),
        }
    };

    // Mac overrides (platform 1, encoding 0, lang 0 = English)
    let mac_overrides: &[(u16, Vec<u8>)] = &[
        (4, mac_full.into_bytes()),
        (6, mac_ps.into_bytes()),
        (17, mac_subfamily.into_bytes()),
    ];
    // Collect all existing records (raw header bytes + string bytes)
    // Each entry: (platformID, encodingID, languageID, nameID, string_bytes)
    let mut records: Vec<(u16, u16, u16, u16, Vec<u8>)> = Vec::new();

    for i in 0..count {
        let rec = 6 + i * 12;
        let pid = u16::from_be_bytes([name_bytes[rec], name_bytes[rec + 1]]);
        let eid = u16::from_be_bytes([name_bytes[rec + 2], name_bytes[rec + 3]]);
        let lid = u16::from_be_bytes([name_bytes[rec + 4], name_bytes[rec + 5]]);
        let nid = u16::from_be_bytes([name_bytes[rec + 6], name_bytes[rec + 7]]);
        let len = u16::from_be_bytes([name_bytes[rec + 8], name_bytes[rec + 9]]) as usize;
        let off = u16::from_be_bytes([name_bytes[rec + 10], name_bytes[rec + 11]]) as usize;

        // Skip records we are replacing
        if pid == 3 && eid == 1 && override_ids.contains(&nid) {
            continue;
        }
        if pid == 1 && eid == 0 && override_ids.contains(&nid) {
            continue;
        }

        let start = string_offset + off;
        let string_bytes = if start + len <= name_bytes.len() {
            name_bytes[start..start + len].to_vec()
        } else {
            vec![]
        };
        records.push((pid, eid, lid, nid, string_bytes));
    }

    // Add the override records
    for &(name_id, ref bytes) in overrides {
        records.push((3, 1, 0x0409, name_id, bytes.clone()));
    }
    for &(name_id, ref bytes) in mac_overrides {
        // Only add Mac record if the font had one originally
        if read_mac(name_id).is_some() || name_id == 4 || name_id == 6 {
            records.push((1, 0, 0, name_id, bytes.clone()));
        }
    }

    // Sort by (platformID, encodingID, languageID, nameID) — required by spec
    records.sort_by_key(|&(pid, eid, lid, nid, _)| (pid, eid, lid, nid));

    // Serialise: header + records + string storage
    let new_count = records.len() as u16;
    let new_string_offset = 6u16 + new_count * 12;
    let mut string_storage: Vec<u8> = Vec::new();
    let mut record_bytes: Vec<u8> = Vec::with_capacity(records.len() * 12);

    for (pid, eid, lid, nid, ref s) in &records {
        let off = string_storage.len() as u16;
        let len = s.len() as u16;
        string_storage.extend_from_slice(s);

        record_bytes.extend_from_slice(&pid.to_be_bytes());
        record_bytes.extend_from_slice(&eid.to_be_bytes());
        record_bytes.extend_from_slice(&lid.to_be_bytes());
        record_bytes.extend_from_slice(&nid.to_be_bytes());
        record_bytes.extend_from_slice(&len.to_be_bytes());
        record_bytes.extend_from_slice(&off.to_be_bytes());
    }

    let mut out: Vec<u8> = Vec::new();
    out.extend_from_slice(&0u16.to_be_bytes());          // format = 0
    out.extend_from_slice(&new_count.to_be_bytes());
    out.extend_from_slice(&new_string_offset.to_be_bytes());
    out.extend_from_slice(&record_bytes);
    out.extend_from_slice(&string_storage);
    out
}

fn patch_u16(data: &mut Vec<u8>, offset: usize, value: u16) {
    if offset + 2 <= data.len() {
        data[offset] = (value >> 8) as u8;
        data[offset + 1] = (value & 0xff) as u8;
    }
}



// ─── Main style_font implementation ──────────────────────────────────────────

fn style_font_internal(font_data: &[u8], req: StyleFontRequest) -> Result<Vec<u8>, String> {
    let original = FontRef::new(font_data).map_err(|e| e.to_string())?;
    let orig_head = original.head().map_err(|e| e.to_string())?;
    let orig_hhea = original.hhea().map_err(|e| e.to_string())?;
    let upem = orig_head.units_per_em();
    let ascender = orig_hhea.ascender().to_i16();
    let descender = orig_hhea.descender().to_i16();
    let glyph_count = original.maxp().map_err(|e| e.to_string())?.num_glyphs();

    // scaleX factor for advance width adjustment
    let scale_x: f32 = req.transforms.iter().fold(1.0f32, |acc, t| {
        if let Transform::ScaleX { factor } = t { acc * factor } else { acc }
    });

    let outlines = original.outline_glyphs();
    let metrics = original.glyph_metrics(Size::new(upem as f32), LocationRef::default());

    // ── Rebuild glyf + loca ────────────────────────────────────────────────────
    let mut glyf_builder = GlyfLocaBuilder::new();
    let mut h_metrics: Vec<LongMetric> = Vec::with_capacity(glyph_count as usize);

    for gid_u16 in 0..glyph_count {
        let gid = skrifa::GlyphId::new(gid_u16 as u32);

        // Original advance width (scaled if scaleX is applied)
        let orig_advance = metrics.advance_width(gid).unwrap_or(upem as f32 * 0.5);
        let new_advance = (orig_advance * scale_x).round() as u16;

        // Collect glyph outline
        let mut pen = CollectPen::new();
        let settings = DrawSettings::unhinted(Size::new(upem as f32), LocationRef::default());
        let has_outline = outlines
            .get(gid)
            .and_then(|g| g.draw(settings, &mut pen).ok())
            .is_some();

        let write_glyph = if !has_outline || pen.path.is_empty() {
            WriteGlyph::Empty
        } else {
            let transformed = apply_transforms_to_path(&pen.path, &req.transforms, gid_u16 as u32, upem as f64);
            let quad = cubics_to_quads(&transformed);
            SimpleGlyph::from_bezpath(&quad)
                .map(WriteGlyph::from)
                .unwrap_or(WriteGlyph::Empty)
        };

        let lsb = match &write_glyph {
            WriteGlyph::Simple(g) => g.bbox.x_min,
            _ => 0,
        };

        glyf_builder.add_glyph(&write_glyph).map_err(|e| e.to_string())?;
        h_metrics.push(LongMetric { advance: new_advance, side_bearing: lsb });
    }

    let (new_glyf, new_loca, loca_format) = glyf_builder.build();
    let new_glyf_bytes = dump_table(&new_glyf).map_err(|e| e.to_string())?;
    let new_loca_bytes = dump_table(&new_loca).map_err(|e| e.to_string())?;
    let hmtx = Hmtx::new(h_metrics, vec![]);

    // ── head: patch indexToLocFormat + clear checkSumAdjustment ───────────────
    let mut head_bytes = original
        .table_data(ReadTag::new(b"head"))
        .ok_or("no head table")?
        .as_bytes()
        .to_vec();
    let loca_fmt_val: u16 = match loca_format {
        write_fonts::tables::loca::LocaFormat::Long => 1,
        write_fonts::tables::loca::LocaFormat::Short => 0,
    };
    patch_u16(&mut head_bytes, 50, loca_fmt_val);
    patch_u16(&mut head_bytes, 8, 0);
    patch_u16(&mut head_bytes, 10, 0);

    // ── hhea: patch numberOfHMetrics (all glyphs have full metrics now) ────────
    let mut hhea_bytes = original
        .table_data(ReadTag::new(b"hhea"))
        .ok_or("no hhea table")?
        .as_bytes()
        .to_vec();
    patch_u16(&mut hhea_bytes, 34, glyph_count);

    // ── COLR + CPAL ───────────────────────────────────────────────────────────
    let (colr, cpal) = build_colr_cpal(glyph_count, &req.colr, upem, ascender, descender);

    // ── maxp: always build version 1.0 (required for TTF/glyf fonts) ─────────
    // Compute maxPoints and maxContours by scanning glyph_gids isn't possible
    // here since we've already serialised. Set to 0 — validators accept this.
    let maxp = Maxp {
        num_glyphs: glyph_count,
        max_points: Some(0),
        max_contours: Some(0),
        max_composite_points: Some(0),
        max_composite_contours: Some(0),
        max_zones: Some(2),
        max_twilight_points: Some(0),
        max_storage: Some(0),
        max_function_defs: Some(0),
        max_instruction_defs: Some(0),
        max_stack_elements: Some(0),
        max_size_of_instructions: Some(0),
        max_component_elements: Some(0),
        max_component_depth: Some(0),
    };

    // ── name: rebuild with mood injected into Subfamily / Full name / PS name ──
    let name_bytes = original
        .table_data(ReadTag::new(b"name"))
        .map(|d| {
            if let Some(ref mood) = req.mood {
                patch_name_table(d.as_bytes(), mood)
            } else {
                d.as_bytes().to_vec()
            }
        })
        .unwrap_or_default();

    // ── Assemble font ─────────────────────────────────────────────────────────
    let skip: &[[u8; 4]] = &[
        *b"glyf", *b"loca", *b"hmtx", *b"maxp", *b"head", *b"hhea",
        *b"name",
        *b"COLR", *b"CPAL",
        *b"CFF ", *b"CFF2",
        *b"HVAR", *b"VVAR", *b"MVAR", *b"STAT", *b"fvar", *b"gvar",
    ];

    let mut builder = FontBuilder::new();
    copy_tables_except(&mut builder, font_data, &original, skip);

    builder.add_raw(Tag::new(b"name"), name_bytes);
    builder.add_raw(Tag::new(b"glyf"), new_glyf_bytes);
    builder.add_raw(Tag::new(b"loca"), new_loca_bytes);
    builder.add_raw(Tag::new(b"head"), head_bytes);
    builder.add_raw(Tag::new(b"hhea"), hhea_bytes);

    let font_bytes = builder
        .add_table(&maxp).map_err(|e| e.to_string())?
        .add_table(&hmtx).map_err(|e| e.to_string())?
        .add_table(&cpal).map_err(|e| e.to_string())?
        .add_table(&colr).map_err(|e| e.to_string())?
        .build();

    Ok(font_bytes)
}

// ─── COLR + CPAL ─────────────────────────────────────────────────────────────

fn build_colr_cpal(
    glyph_count: u16,
    colr_input: &ColrInput,
    upem: u16,
    ascender: i16,
    descender: i16,
) -> (Colr, Cpal) {
    let (palette, fill_idx, grad0_idx, grad1_idx, block_idx, shadow_idx, white_idx, outline_idx, grad2_idx) =
        build_palette(colr_input);

    let has = |e: &str| colr_input.effects.iter().any(|s| s == e);
    let has_shadow = has("shadow");
    let has_3d = has("3d-blocks");
    let has_fill = has("fill");
    let has_gradient = has("gradient");
    let has_highlight = has("highlight");
    let has_outline = has("outline");
    let has_double_outline = has("double-outline");
    let has_3rd_gradient = colr_input.gradient_colors.as_ref().map(|v| v.len() >= 3).unwrap_or(false);

    let mut layer_list: Vec<Paint> = vec![];
    let mut base_glyph_paints: Vec<BaseGlyphPaint> = vec![];

    // Build paint tree for every glyph that has an outline (skip .notdef = GID 0
    // if desired, but including it is harmless).
    for gid_u16 in 0..glyph_count {
        let gid = GlyphId16::new(gid_u16);
        let first_layer = layer_list.len() as u32;

        // Outline rings: 16 evenly-spaced directions per ring for smooth coverage
        if has_double_outline || has_outline {
            let make_ring = |d: f32| -> Vec<(i16, i16)> {
                (0..16).map(|i| {
                    let a = 2.0 * std::f32::consts::PI * (i as f32) / 16.0;
                    ((d * a.cos()).round() as i16, (d * a.sin()).round() as i16)
                }).collect()
            };
            let d_inner = (upem as f32 / 18.0).max(20.0);
            if has_double_outline {
                let d_outer = (upem as f32 / 8.0).max(40.0);
                for (dx, dy) in make_ring(d_outer) {
                    layer_list.push(Paint::Translate(PaintTranslate::new(
                        Paint::Glyph(PaintGlyph::new(
                            Paint::Solid(PaintSolid::new(outline_idx, F2Dot14::from_f32(0.5))),
                            gid,
                        )),
                        FWord::new(dx), FWord::new(dy),
                    )));
                }
            }
            for (dx, dy) in make_ring(d_inner) {
                layer_list.push(Paint::Translate(PaintTranslate::new(
                    Paint::Glyph(PaintGlyph::new(
                        Paint::Solid(PaintSolid::new(outline_idx, F2Dot14::from_f32(1.0))),
                        gid,
                    )),
                    FWord::new(dx), FWord::new(dy),
                )));
            }
        }

        if has_shadow {
            layer_list.push(Paint::Translate(PaintTranslate::new(
                Paint::Glyph(PaintGlyph::new(
                    Paint::Solid(PaintSolid::new(shadow_idx, F2Dot14::from_f32(0.28))),
                    gid,
                )),
                FWord::new(50),
                FWord::new(-60),
            )));
        }

        if has_3d {
            for j in (1i16..=8).rev() {
                let offset = j * 5;
                layer_list.push(Paint::Translate(PaintTranslate::new(
                    Paint::Glyph(PaintGlyph::new(
                        Paint::Solid(PaintSolid::new(block_idx, F2Dot14::from_f32(1.0))),
                        gid,
                    )),
                    FWord::new(offset),
                    FWord::new(-offset),
                )));
            }
        }

        let main_paint = if has_gradient {
            let mut stops = vec![
                ColorStop::new(F2Dot14::from_f32(0.0), grad0_idx, F2Dot14::from_f32(1.0)),
                ColorStop::new(F2Dot14::from_f32(1.0), grad1_idx, F2Dot14::from_f32(1.0)),
            ];
            if has_3rd_gradient {
                stops.insert(1, ColorStop::new(F2Dot14::from_f32(0.5), grad2_idx, F2Dot14::from_f32(1.0)));
            }
            let color_line = ColorLine::new(Extend::Pad, stops.len() as u16, stops);
            Paint::Glyph(PaintGlyph::new(
                Paint::LinearGradient(PaintLinearGradient::new(
                    color_line,
                    FWord::new(0),
                    FWord::new(ascender),
                    FWord::new(upem as i16),
                    FWord::new(descender),
                    FWord::new(upem as i16),
                    FWord::new(ascender),
                )),
                gid,
            ))
        } else if has_fill || has_3d {
            Paint::Glyph(PaintGlyph::new(
                Paint::Solid(PaintSolid::new(fill_idx, F2Dot14::from_f32(1.0))),
                gid,
            ))
        } else {
            Paint::Glyph(PaintGlyph::new(
                Paint::Solid(PaintSolid::new(0, F2Dot14::from_f32(1.0))),
                gid,
            ))
        };
        layer_list.push(main_paint);

        if has_highlight {
            let mid_y = (ascender as i32 + descender as i32) / 2;
            let color_line = ColorLine::new(
                Extend::Pad,
                2,
                vec![
                    ColorStop::new(F2Dot14::from_f32(0.0), white_idx, F2Dot14::from_f32(0.55)),
                    ColorStop::new(F2Dot14::from_f32(1.0), white_idx, F2Dot14::from_f32(0.0)),
                ],
            );
            layer_list.push(Paint::Glyph(PaintGlyph::new(
                Paint::LinearGradient(PaintLinearGradient::new(
                    color_line,
                    FWord::new(0),
                    FWord::new(ascender),
                    FWord::new(0),
                    FWord::new(mid_y as i16),
                    FWord::new(upem as i16),
                    FWord::new(ascender),
                )),
                gid,
            )));
        }

        let num_layers = (layer_list.len() as u32 - first_layer) as u8;
        let root_paint = if num_layers == 1 {
            layer_list.pop().unwrap()
        } else {
            Paint::ColrLayers(PaintColrLayers::new(num_layers, first_layer))
        };

        base_glyph_paints.push(BaseGlyphPaint::new(gid, root_paint));
    }

    // ── COLRv0 fallback (macOS CoreText doesn't render COLRv1 for user-installed fonts) ──
    // One solid-color layer per glyph using the primary effect color.
    // COLRv1 (Chrome, etc.) uses the full paint tree above; CoreText falls back to v0.
    let v0_palette_idx: u16 = if has_gradient {
        grad0_idx
    } else if has_fill || has_3d {
        fill_idx
    } else {
        0 // black — same result as glyf, no visual change
    };
    let mut v0_base_glyphs: Vec<BaseGlyph> = Vec::with_capacity(glyph_count as usize);
    let mut v0_layers: Vec<Layer> = Vec::with_capacity(glyph_count as usize);
    for gid_u16 in 0..glyph_count {
        let first_layer = v0_layers.len() as u16;
        v0_layers.push(Layer::new(GlyphId16::new(gid_u16), v0_palette_idx));
        v0_base_glyphs.push(BaseGlyph::new(GlyphId16::new(gid_u16), first_layer, 1));
    }
    let num_v0_base = v0_base_glyphs.len() as u16;
    let num_v0_layers = v0_layers.len() as u16;

    let n_layers = layer_list.len() as u32;
    let n_base = base_glyph_paints.len() as u32;
    let mut colr = Colr::new(num_v0_base, Some(v0_base_glyphs), Some(v0_layers), num_v0_layers);
    colr.base_glyph_list = Some(BaseGlyphList::new(n_base, base_glyph_paints)).into();
    if n_layers > 0 {
        colr.layer_list = Some(LayerList::new(n_layers, layer_list)).into();
    }

    let n_colors = palette.len() as u16;
    let cpal = Cpal::new(n_colors, 1, n_colors, Some(palette), vec![0]);

    (colr, cpal)
}


/// 1-D Euclidean distance transform (Felzenszwalb-Huttenlocher).
/// Input: squared initial distances (0.0 for seeds, large value for non-seeds).
/// Returns squared distances to nearest seed.
fn edt_1d(f: &[f32]) -> Vec<f32> {
    let n = f.len();
    if n == 0 { return vec![]; }
    let mut d = vec![0.0f32; n];
    let mut v = vec![0usize; n];
    let mut z = vec![0.0f32; n + 1];
    v[0] = 0;
    z[0] = f32::NEG_INFINITY;
    z[1] = f32::INFINITY;
    let mut k = 0usize;

    for q in 1..n {
        loop {
            let r = v[k] as f32;
            let s = ((f[q] + (q * q) as f32) - (f[v[k]] + (v[k] * v[k]) as f32))
                / (2.0 * q as f32 - 2.0 * r);
            if s <= z[k] {
                if k == 0 { v[0] = q; z[0] = f32::NEG_INFINITY; z[1] = f32::INFINITY; break; }
                k -= 1;
            } else {
                k += 1;
                v[k] = q;
                z[k] = s;
                z[k + 1] = f32::INFINITY;
                break;
            }
        }
    }
    let mut k = 0usize;
    for q in 0..n {
        while z[k + 1] < q as f32 { k += 1; }
        let diff = q as f32 - v[k] as f32;
        d[q] = diff * diff + f[v[k]];
    }
    d
}

/// Separable 2-D Euclidean distance transform.
/// Input: 0.0 for seed pixels, large value otherwise.
/// Returns Euclidean distances (not squared).
fn edt_2d(grid: &[f32], w: usize, h: usize) -> Vec<f32> {
    let mut g = grid.to_vec();
    for y in 0..h {
        let row = g[y * w..(y + 1) * w].to_vec();
        let d = edt_1d(&row);
        g[y * w..(y + 1) * w].copy_from_slice(&d);
    }
    for x in 0..w {
        let col: Vec<f32> = (0..h).map(|y| g[y * w + x]).collect();
        let d = edt_1d(&col);
        for y in 0..h { g[y * w + x] = d[y].sqrt(); }
    }
    g
}

/// Compute signed distance field: positive inside, negative outside.
fn compute_sdf(bitmap: &[bool], w: usize, h: usize) -> Vec<f32> {
    let inf = (w * w + h * h + 1) as f32;
    let to_outside: Vec<f32> = bitmap.iter().map(|&b| if !b { 0.0 } else { inf }).collect();
    let to_inside:  Vec<f32> = bitmap.iter().map(|&b| if  b { 0.0 } else { inf }).collect();
    let dist_out = edt_2d(&to_outside, w, h);
    let dist_in  = edt_2d(&to_inside,  w, h);
    bitmap.iter().enumerate().map(|(i, &inside)| {
        if inside { dist_out[i] } else { -dist_in[i] }
    }).collect()
}

/// Directed-edge boundary tracing. Collects edges between inside/outside pixel pairs,
/// then follows the directed chain to form closed contours. One contour per connected
/// boundary component; no interior pixels are visited so contours are never fragmented.
fn trace_contours(bitmap: &[bool], w: usize, h: usize) -> Vec<Vec<(f32, f32)>> {
    use std::collections::{HashMap, HashSet};

    let inside = |x: i32, y: i32| -> bool {
        x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < h
            && bitmap[y as usize * w + x as usize]
    };

    // Build one directed edge per boundary transition.
    // Corner coordinates: pixel (x,y) has corners at integer (x,y)..(x+1,y+1).
    // Winding: inside region is to the left of each directed edge.
    let mut next_edge: HashMap<(i32, i32), (i32, i32)> = HashMap::new();
    for y in 0..=(h as i32) {
        for x in 0..=(w as i32) {
            let above = inside(x, y - 1);
            let below = inside(x, y);
            if above != below {
                if below { next_edge.insert((x, y), (x + 1, y)); }
                else     { next_edge.insert((x + 1, y), (x, y)); }
            }
            let left  = inside(x - 1, y);
            let right = inside(x, y);
            if left != right {
                if right { next_edge.insert((x, y + 1), (x, y)); }
                else     { next_edge.insert((x, y), (x, y + 1)); }
            }
        }
    }

    let mut visited: HashSet<(i32, i32)> = HashSet::new();
    let mut contours = Vec::new();
    let mut starts: Vec<(i32, i32)> = next_edge.keys().copied().collect();
    starts.sort_unstable();

    for start in starts {
        if visited.contains(&start) { continue; }
        let mut contour: Vec<(f32, f32)> = Vec::new();
        let mut cur = start;
        loop {
            if visited.contains(&cur) { break; }
            visited.insert(cur);
            contour.push((cur.0 as f32, cur.1 as f32));
            match next_edge.get(&cur) {
                Some(&nxt) => cur = nxt,
                None => break,
            }
        }
        if contour.len() >= 3 { contours.push(contour); }
    }
    contours
}

/// Douglas-Peucker polyline simplification.
// ── Bézier curve fitting (Schneider 1990) ────────────────────────────────────

#[inline] fn v2add(a:(f64,f64),b:(f64,f64))->(f64,f64){(a.0+b.0,a.1+b.1)}
#[inline] fn v2sub(a:(f64,f64),b:(f64,f64))->(f64,f64){(a.0-b.0,a.1-b.1)}
#[inline] fn v2scale(v:(f64,f64),s:f64)->(f64,f64){(v.0*s,v.1*s)}
#[inline] fn v2dot(a:(f64,f64),b:(f64,f64))->f64{a.0*b.0+a.1*b.1}
#[inline] fn v2len(v:(f64,f64))->f64{(v.0*v.0+v.1*v.1).sqrt()}
#[inline] fn v2dist(a:(f64,f64),b:(f64,f64))->f64{v2len(v2sub(a,b))}
#[inline] fn v2norm(v:(f64,f64))->(f64,f64){
    let l=v2len(v); if l<1e-10{(1.0,0.0)}else{(v.0/l,v.1/l)}
}

fn eval_bezier(p:&[(f64,f64);4],t:f64)->(f64,f64){
    let mt=1.0-t;
    v2add(v2add(v2scale(p[0],mt*mt*mt),v2scale(p[1],3.0*mt*mt*t)),
          v2add(v2scale(p[2],3.0*mt*t*t),v2scale(p[3],t*t*t)))
}

fn chord_params(pts:&[(f64,f64)])->Vec<f64>{
    let n=pts.len();
    let mut u=vec![0.0f64;n];
    for i in 1..n{u[i]=u[i-1]+v2dist(pts[i],pts[i-1]);}
    let tot=u[n-1];
    if tot>0.0{u.iter_mut().for_each(|v|*v/=tot);}
    u
}

fn fit_one_bezier(pts:&[(f64,f64)],u:&[f64],t1:(f64,f64),t2:(f64,f64))->[(f64,f64);4]{
    let n=pts.len();
    let p0=pts[0]; let p3=pts[n-1];
    let mut c=[[0.0f64;2];2];
    let mut x=[0.0f64;2];
    for i in 0..n{
        let t=u[i]; let mt=1.0-t;
        let b1=3.0*mt*mt*t; let b2=3.0*mt*t*t;
        let a1=v2scale(t1,b1); let a2=v2scale(t2,b2);
        c[0][0]+=v2dot(a1,a1); c[0][1]+=v2dot(a1,a2);
        c[1][0]+=v2dot(a2,a1); c[1][1]+=v2dot(a2,a2);
        let b0=mt*mt*mt; let b3=t*t*t;
        let rhs=v2sub(pts[i],v2add(v2scale(p0,b0+b1),v2scale(p3,b2+b3)));
        x[0]+=v2dot(a1,rhs); x[1]+=v2dot(a2,rhs);
    }
    let det=c[0][0]*c[1][1]-c[0][1]*c[1][0];
    let (al1,al2)=if det.abs()<1e-10{
        let d=v2dist(p0,p3)/3.0;(d,d)
    }else{
        ((x[0]*c[1][1]-x[1]*c[0][1])/det,(c[0][0]*x[1]-c[1][0]*x[0])/det)
    };
    [p0,v2add(p0,v2scale(t1,al1.max(0.0))),v2add(p3,v2scale(t2,al2.max(0.0))),p3]
}

fn max_err_idx(pts:&[(f64,f64)],u:&[f64],bez:&[(f64,f64);4])->(f64,usize){
    (0..pts.len())
        .map(|i|(v2dist(pts[i],eval_bezier(bez,u[i])),i))
        .fold((0.0,0),|(md,mi),(d,i)|if d>md{(d,i)}else{(md,mi)})
}

fn fit_cubic_seg(pts:&[(f64,f64)],t1:(f64,f64),t2:(f64,f64),max_e:f64,out:&mut Vec<[(f64,f64);4]>){
    let n=pts.len();
    if n<2{return;}
    if n==2{
        let d=v2dist(pts[0],pts[1])/3.0;
        out.push([pts[0],v2add(pts[0],v2scale(t1,d)),v2add(pts[1],v2scale(t2,d)),pts[1]]);
        return;
    }
    let u=chord_params(pts);
    let bez=fit_one_bezier(pts,&u,t1,t2);
    let(err,si)=max_err_idx(pts,&u,&bez);
    if err<=max_e{out.push(bez);return;}
    // Newton reparameterize once then retry
    let u2:Vec<f64>=(0..n).map(|i|{
        let t0=u[i]; let mt=1.0-t0;
        let q=eval_bezier(&bez,t0);
        let d=v2add(v2add(v2scale(v2sub(bez[1],bez[0]),3.0*mt*mt),
                          v2scale(v2sub(bez[2],bez[1]),6.0*mt*t0)),
                    v2scale(v2sub(bez[3],bez[2]),3.0*t0*t0));
        let den=v2dot(d,d);
        if den<1e-10{t0}else{(t0-v2dot(v2sub(q,pts[i]),d)/den).clamp(0.0,1.0)}
    }).collect();
    let bez2=fit_one_bezier(pts,&u2,t1,t2);
    let(err2,si2)=max_err_idx(pts,&u2,&bez2);
    if err2<=max_e{out.push(bez2);return;}
    // Split at point of max error
    let si=if err2<err{si2}else{si}.clamp(1,n-2);
    let ct=v2norm(v2sub(pts[si.saturating_sub(1)],pts[(si+1).min(n-1)]));
    fit_cubic_seg(&pts[..=si],t1,ct,max_e,out);
    fit_cubic_seg(&pts[si..],(-ct.0,-ct.1),t2,max_e,out);
}

/// Fit cubic Bézier curves to a closed pixel contour and append to BezPath.
/// Replaces Douglas-Peucker + Chaikin with Schneider's least-squares algorithm.
fn fit_bezier_contour(
    path: &mut BezPath,
    contour: &[(f32,f32)],
    left_pad_px: f64,
    baseline_px: f64,
    scale: f64,
) {
    let n=contour.len();
    if n<3{return;}
    let pts:Vec<(f64,f64)>=contour.iter().map(|&(x,y)|(x as f64,y as f64)).collect();
    let to_font=|p:(f64,f64)|->Point{
        Point::new((p.0-left_pad_px)/scale,(baseline_px-p.1)/scale)
    };
    // Detect corners: angle between consecutive directions > 60°
    let corners:Vec<usize>=(0..n).filter(|&i|{
        let d1=v2norm(v2sub(pts[i],pts[(i+n-1)%n]));
        let d2=v2norm(v2sub(pts[(i+1)%n],pts[i]));
        v2dot(d1,d2)<0.5
    }).collect();
    let starts=if corners.is_empty(){vec![0]}else{corners};
    let nc=starts.len();
    let mut first=true;
    for ci in 0..nc{
        let s=starts[ci];
        let e=starts[(ci+1)%nc];
        let mut seg:Vec<(f64,f64)>=Vec::new();
        let mut i=s;
        loop{
            seg.push(pts[i]);
            i=(i+1)%n;
            if i==e{seg.push(pts[e]);break;}
            if seg.len()>n+2{break;}
        }
        if seg.len()<2{continue;}
        let last=seg.len()-1;
        let t1=v2norm(v2sub(seg[1],seg[0]));
        let t2=v2norm(v2sub(seg[last-1],seg[last]));
        let mut beziers:Vec<[(f64,f64);4]>=Vec::new();
        fit_cubic_seg(&seg,t1,t2,1.5,&mut beziers);
        for bez in &beziers{
            if first{path.move_to(to_font(bez[0]));first=false;}
            path.curve_to(to_font(bez[1]),to_font(bez[2]),to_font(bez[3]));
        }
    }
    if !first{path.close_path();}
}

fn dilate(bitmap: &[bool], w: usize, h: usize, r: i32) -> Vec<bool> {
    let mut out = vec![false; w * h];
    for y in 0..h {
        for x in 0..w {
            'search: for dy in -r..=r {
                for dx in -r..=r {
                    let nx = x as i32 + dx;
                    let ny = y as i32 + dy;
                    if nx >= 0 && ny >= 0 && nx < w as i32 && ny < h as i32
                        && bitmap[ny as usize * w + nx as usize]
                    {
                        out[y * w + x] = true;
                        break 'search;
                    }
                }
            }
        }
    }
    out
}

fn erode(bitmap: &[bool], w: usize, h: usize, r: i32) -> Vec<bool> {
    let mut out = bitmap.to_vec();
    for y in 0..h {
        for x in 0..w {
            if !bitmap[y * w + x] { continue; }
            'search: for dy in -r..=r {
                for dx in -r..=r {
                    let nx = x as i32 + dx;
                    let ny = y as i32 + dy;
                    if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32
                        || !bitmap[ny as usize * w + nx as usize]
                    {
                        out[y * w + x] = false;
                        break 'search;
                    }
                }
            }
        }
    }
    out
}

/// Morphological opening (erode → dilate): removes small isolated blobs.
fn morphological_open(bitmap: &[bool], w: usize, h: usize, r: i32) -> Vec<bool> {
    let eroded = erode(bitmap, w, h, r);
    dilate(&eroded, w, h, r)
}

/// Remove connected components with fewer than `min_size` pixels (4-connected).
fn remove_small_components(bitmap: &mut [bool], w: usize, h: usize, min_size: usize) {
    let mut visited = vec![false; w * h];
    for sy in 0..h {
        for sx in 0..w {
            if !bitmap[sy * w + sx] || visited[sy * w + sx] { continue; }
            let mut component = Vec::new();
            let mut queue = std::collections::VecDeque::new();
            queue.push_back((sx, sy));
            visited[sy * w + sx] = true;
            while let Some((cx, cy)) = queue.pop_front() {
                component.push((cx, cy));
                for (nx, ny) in [
                    (cx.wrapping_sub(1), cy), (cx + 1, cy),
                    (cx, cy.wrapping_sub(1)), (cx, cy + 1),
                ] {
                    if nx < w && ny < h && bitmap[ny * w + nx] && !visited[ny * w + nx] {
                        visited[ny * w + nx] = true;
                        queue.push_back((nx, ny));
                    }
                }
            }
            if component.len() < min_size {
                for (px, py) in component {
                    bitmap[py * w + px] = false;
                }
            }
        }
    }
}
/// Debug: SDF-blend a single pair of grayscale bitmaps; returns grayscale result (0=inside, 255=outside).
#[wasm_bindgen]
pub fn blend_one_bitmap_debug(bmp1: &[u8], bmp2: &[u8], bitmap_size: u32, blend_factor: f32) -> Vec<u8> {
    let sz = bitmap_size as usize;
    let bin1: Vec<bool> = bmp1.iter().map(|&v| v < 128).collect();
    let bin2: Vec<bool> = bmp2.iter().map(|&v| v < 128).collect();
    let sdf1 = compute_sdf(&bin1, sz, sz);
    let sdf2 = compute_sdf(&bin2, sz, sz);
    let t = blend_factor.clamp(0.0, 1.0);
    let blended_raw: Vec<bool> = sdf1.iter().zip(&sdf2)
        .map(|(&a, &b)| a * (1.0 - t) + b * t > 0.0)
        .collect();
    let mut blended = morphological_open(&blended_raw, sz, sz, 2);
    remove_small_components(&mut blended, sz, sz, 100);
    blended.iter().map(|&b| if b { 0u8 } else { 255u8 }).collect()
}

/// Blend fonts using pre-rendered OffscreenCanvas bitmaps for the listed chars.
/// Other glyphs are copied from font1 unchanged.
#[wasm_bindgen]
pub fn blend_from_canvas_bitmaps(
    font1_data: &[u8],
    font2_data: &[u8],
    bitmaps1: &[u8],
    bitmaps2: &[u8],
    request_json: &str,
) -> Result<Vec<u8>, JsValue> {
    let sfnt1 = to_sfnt(font1_data).map_err(|e| JsValue::from_str(&e))?;
    let sfnt2 = to_sfnt(font2_data).map_err(|e| JsValue::from_str(&e))?;
    let req: BlendCanvasRequest = serde_json::from_str(request_json)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    blend_from_canvas_internal(&sfnt1, &sfnt2, bitmaps1, bitmaps2, req)
        .map_err(|e| JsValue::from_str(&e))
}

fn blend_from_canvas_internal(
    font1_data: &[u8],
    font2_data: &[u8],
    bitmaps1: &[u8],
    bitmaps2: &[u8],
    req: BlendCanvasRequest,
) -> Result<Vec<u8>, String> {
    let font1 = FontRef::new(font1_data).map_err(|e| e.to_string())?;
    let font2 = FontRef::new(font2_data).map_err(|e| e.to_string())?;

    let head1 = font1.head().map_err(|e| e.to_string())?;
    let hhea1 = font1.hhea().map_err(|e| e.to_string())?;
    let upem1 = head1.units_per_em() as f64;
    let ascender1 = hhea1.ascender().to_i16() as f64;
    let descender1 = hhea1.descender().to_i16() as f64;

    let head2 = font2.head().map_err(|e| e.to_string())?;
    let upem2 = head2.units_per_em() as f64;
    let upem_scale2 = upem1 / upem2.max(1.0);

    let glyph_count = font1.maxp().map_err(|e| e.to_string())?.num_glyphs();

    let t = req.blend_factor.clamp(0.0, 1.0) as f64;
    let bmp_sz = req.bitmap_size as usize;

    // Canvas → font coordinate transform (must match JS renderGlyphToCanvas)
    let em = (ascender1 - descender1).max(1.0);
    let scale = (bmp_sz as f64 * 0.8) / em;
    let baseline_px = bmp_sz as f64 * 0.5 + (ascender1 + descender1) * 0.5 * scale;

    // Build glyph_id → bitmap_index and glyph_id → font2 advance maps via charmap.
    // Both lookups go through character codes so Font2's glyph ordering doesn't matter.
    let charmap1 = font1.charmap();
    let charmap2 = font2.charmap();
    let metrics1 = font1.glyph_metrics(Size::new(upem1 as f32), LocationRef::default());
    let metrics2 = font2.glyph_metrics(Size::new(upem2 as f32), LocationRef::default());
    let mut gid_to_bmp: HashMap<u32, usize> = HashMap::new();
    let mut gid_to_adv2: HashMap<u32, f64> = HashMap::new();
    for (idx, &code) in req.char_codes.iter().enumerate() {
        if let Some(ch) = char::from_u32(code) {
            if let Some(gid1) = charmap1.map(ch) {
                let g1 = gid1.to_u32();
                gid_to_bmp.entry(g1).or_insert(idx);
                if let Some(gid2) = charmap2.map(ch) {
                    let adv2 = metrics2.advance_width(gid2).unwrap_or(upem2 as f32 * 0.5) as f64
                        * upem_scale2;
                    gid_to_adv2.entry(g1).or_insert(adv2);
                }
            }
        }
    }

    let outlines1 = font1.outline_glyphs();

    let mut glyf_builder = GlyfLocaBuilder::new();
    let mut h_metrics: Vec<LongMetric> = Vec::with_capacity(glyph_count as usize);

    for gid_u16 in 0..glyph_count {
        let gid = skrifa::GlyphId::new(gid_u16 as u32);
        let adv1 = metrics1.advance_width(gid).unwrap_or(upem1 as f32 * 0.5) as f64;
        let adv2 = gid_to_adv2.get(&(gid_u16 as u32)).copied().unwrap_or(adv1);
        let blended_advance = (adv1 + (adv2 - adv1) * t).round() as u16;

        // SDF blend from canvas bitmaps
        if let Some(&bmp_idx) = gid_to_bmp.get(&(gid_u16 as u32)) {
            let bstart = bmp_idx * bmp_sz * bmp_sz;
            let bend = bstart + bmp_sz * bmp_sz;
            if bend <= bitmaps1.len() && bend <= bitmaps2.len() {
                let left_pad_px = ((bmp_sz as f64 - adv1 * scale) / 2.0).max(0.0);
                let bin1: Vec<bool> = bitmaps1[bstart..bend].iter().map(|&v| v < 128).collect();
                let bin2: Vec<bool> = bitmaps2[bstart..bend].iter().map(|&v| v < 128).collect();
                let sdf1 = compute_sdf(&bin1, bmp_sz, bmp_sz);
                let sdf2 = compute_sdf(&bin2, bmp_sz, bmp_sz);
                let norm = |sdf: &[f32]| -> Vec<f32> {
                    let mx = sdf.iter().cloned().map(f32::abs).fold(0.0f32, f32::max).max(1.0);
                    sdf.iter().map(|&v| v / mx).collect()
                };
                let sdf1n = norm(&sdf1);
                let sdf2n = norm(&sdf2);
                let tf = t as f32;
                let blended_raw: Vec<bool> = sdf1n.iter().zip(&sdf2n)
                    .map(|(&a, &b)| a * (1.0 - tf) + b * tf > 0.0)
                    .collect();
                let mut blended = morphological_open(&blended_raw, bmp_sz, bmp_sz, 2);
                remove_small_components(&mut blended, bmp_sz, bmp_sz, 100);
                let raw_contours = trace_contours(&blended, bmp_sz, bmp_sz);
                let mut full_path = BezPath::new();
                for contour in &raw_contours {
                    fit_bezier_contour(&mut full_path, contour, left_pad_px, baseline_px, scale);
                }
                let g = if full_path.is_empty() {
                    WriteGlyph::Empty
                } else {
                    let quad = cubics_to_quads(&full_path);
                    SimpleGlyph::from_bezpath(&quad).map(WriteGlyph::from).unwrap_or(WriteGlyph::Empty)
                };
                let lsb = match &g { WriteGlyph::Simple(sg) => sg.bbox.x_min, _ => 0 };
                glyf_builder.add_glyph(&g).map_err(|e| e.to_string())?;
                h_metrics.push(LongMetric { advance: blended_advance, side_bearing: lsb });
                continue;
            }
        }

        // Copy font1 outline unchanged
        let mut pen1 = CollectPen::new();
        let ds = DrawSettings::unhinted(Size::new(upem1 as f32), LocationRef::default());
        let has1 = outlines1.get(gid).and_then(|g| g.draw(ds, &mut pen1).ok()).is_some() && !pen1.path.is_empty();
        if has1 {
            let quad = cubics_to_quads(&pen1.path);
            let g = SimpleGlyph::from_bezpath(&quad).map(WriteGlyph::from).unwrap_or(WriteGlyph::Empty);
            let lsb = match &g { WriteGlyph::Simple(sg) => sg.bbox.x_min, _ => 0 };
            glyf_builder.add_glyph(&g).map_err(|e| e.to_string())?;
            h_metrics.push(LongMetric { advance: blended_advance, side_bearing: lsb });
        } else {
            glyf_builder.add_glyph(&WriteGlyph::Empty).map_err(|e| e.to_string())?;
            h_metrics.push(LongMetric { advance: blended_advance, side_bearing: 0 });
        }
    }

    let (new_glyf, new_loca, loca_format) = glyf_builder.build();
    let new_glyf_bytes = dump_table(&new_glyf).map_err(|e| e.to_string())?;
    let new_loca_bytes = dump_table(&new_loca).map_err(|e| e.to_string())?;
    let hmtx = Hmtx::new(h_metrics, vec![]);

    let mut head_bytes = font1.table_data(ReadTag::new(b"head")).ok_or("no head table")?.as_bytes().to_vec();
    let loca_fmt_val: u16 = match loca_format {
        write_fonts::tables::loca::LocaFormat::Long => 1,
        write_fonts::tables::loca::LocaFormat::Short => 0,
    };
    patch_u16(&mut head_bytes, 50, loca_fmt_val);
    patch_u16(&mut head_bytes, 8, 0);
    patch_u16(&mut head_bytes, 10, 0);

    let mut hhea_bytes = font1.table_data(ReadTag::new(b"hhea")).ok_or("no hhea table")?.as_bytes().to_vec();
    patch_u16(&mut hhea_bytes, 34, glyph_count);

    let empty_colr = ColrInput::default();
    let (colr, cpal) = build_colr_cpal(
        glyph_count, &empty_colr,
        head1.units_per_em(), hhea1.ascender().to_i16(), hhea1.descender().to_i16(),
    );
    let maxp = Maxp {
        num_glyphs: glyph_count,
        max_points: Some(0), max_contours: Some(0),
        max_composite_points: Some(0), max_composite_contours: Some(0),
        max_zones: Some(2), max_twilight_points: Some(0), max_storage: Some(0),
        max_function_defs: Some(0), max_instruction_defs: Some(0),
        max_stack_elements: Some(0), max_size_of_instructions: Some(0),
        max_component_elements: Some(0), max_component_depth: Some(0),
    };
    let name_bytes = font1.table_data(ReadTag::new(b"name"))
        .map(|d| patch_name_table(d.as_bytes(), "Blend"))
        .unwrap_or_default();

    let skip: &[[u8; 4]] = &[
        *b"glyf", *b"loca", *b"hmtx", *b"maxp", *b"head", *b"hhea",
        *b"name", *b"COLR", *b"CPAL",
        *b"CFF ", *b"CFF2", *b"HVAR", *b"VVAR", *b"MVAR", *b"STAT", *b"fvar", *b"gvar",
    ];
    let mut builder = FontBuilder::new();
    copy_tables_except(&mut builder, font1_data, &font1, skip);
    builder.add_raw(Tag::new(b"name"), name_bytes);
    builder.add_raw(Tag::new(b"glyf"), new_glyf_bytes);
    builder.add_raw(Tag::new(b"loca"), new_loca_bytes);
    builder.add_raw(Tag::new(b"head"), head_bytes);
    builder.add_raw(Tag::new(b"hhea"), hhea_bytes);

    let font_bytes = builder
        .add_table(&maxp).map_err(|e| e.to_string())?
        .add_table(&hmtx).map_err(|e| e.to_string())?
        .add_table(&cpal).map_err(|e| e.to_string())?
        .add_table(&colr).map_err(|e| e.to_string())?
        .build();

    Ok(font_bytes)
}

fn build_palette(colr: &ColrInput) -> (Vec<ColorRecord>, u16, u16, u16, u16, u16, u16, u16, u16) {
    let mut records: Vec<ColorRecord> = vec![];

    let parse_hex = |hex: &str| -> (u8, u8, u8) {
        let h = hex.trim_start_matches('#');
        if h.len() < 6 { return (0, 0, 0); }
        let r = u8::from_str_radix(&h[0..2], 16).unwrap_or(0);
        let g = u8::from_str_radix(&h[2..4], 16).unwrap_or(0);
        let b = u8::from_str_radix(&h[4..6], 16).unwrap_or(0);
        (r, g, b)
    };

    let push = |records: &mut Vec<ColorRecord>, r: u8, g: u8, b: u8, a: u8| -> u16 {
        let idx = records.len() as u16;
        records.push(ColorRecord { blue: b, green: g, red: r, alpha: a });
        idx
    };

    let _ = push(&mut records, 0, 0, 0, 255);

    let (fr, fg, fb) = colr.fill_color.as_deref().map(parse_hex).unwrap_or((0xe6, 0x39, 0x46));
    let fill_idx = push(&mut records, fr, fg, fb, 255);

    let (g0r, g0g, g0b) = colr.gradient_colors.as_ref().and_then(|v| v.first()).map(|s| parse_hex(s)).unwrap_or((0xf9, 0x73, 0x16));
    let grad0_idx = push(&mut records, g0r, g0g, g0b, 255);

    let (g1r, g1g, g1b) = colr.gradient_colors.as_ref().and_then(|v| v.get(1)).map(|s| parse_hex(s)).unwrap_or((0x8b, 0x5c, 0xf6));
    let grad1_idx = push(&mut records, g1r, g1g, g1b, 255);

    let (br, bg, bb) = colr.block_color.as_deref().map(parse_hex).unwrap_or((0x11, 0x11, 0x11));
    let block_idx = push(&mut records, br, bg, bb, 255);

    let shadow_idx = push(&mut records, 0, 0, 0, 255);
    let white_idx = push(&mut records, 255, 255, 255, 255);

    let (or_, og, ob) = colr.outline_color.as_deref().map(parse_hex).unwrap_or((0xff, 0xff, 0xff));
    let outline_idx = push(&mut records, or_, og, ob, 255);

    let (g2r, g2g, g2b) = colr.gradient_colors.as_ref().and_then(|v| v.get(2)).map(|s| parse_hex(s)).unwrap_or((g1r, g1g, g1b));
    let grad2_idx = push(&mut records, g2r, g2g, g2b, 255);

    (records, fill_idx, grad0_idx, grad1_idx, block_idx, shadow_idx, white_idx, outline_idx, grad2_idx)
}
