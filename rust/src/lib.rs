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
#[serde(tag = "type", rename_all = "camelCase")]
enum Transform {
    ScaleX { factor: f32 },
    ScaleY { factor: f32 },
    Shear { angle: f32 },
    Jitter { amplitude: f32 },
    Wave { amplitude: f32, frequency: f32 },
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct ColrInput {
    #[serde(default)]
    effects: Vec<String>,
    fill_color: Option<String>,
    gradient_colors: Option<Vec<String>>,
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

fn transform_point(x: f64, y: f64, transforms: &[Transform], rand: &mut Lcg) -> (f64, f64) {
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
        }
    }
    (nx, ny)
}

// ─── Glyph collection pen ────────────────────────────────────────────────────

use kurbo::{BezPath, CubicBez, PathEl, Point};

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

fn apply_transforms_to_path(path: &BezPath, transforms: &[Transform], seed: u32) -> BezPath {
    if transforms.is_empty() {
        return path.clone();
    }
    let mut rand = Lcg::new(seed);
    let mut out = BezPath::new();
    for el in path.iter() {
        match el {
            PathEl::MoveTo(p) => {
                let (nx, ny) = transform_point(p.x, p.y, transforms, &mut rand);
                out.move_to(Point::new(nx, ny));
            }
            PathEl::LineTo(p) => {
                let (nx, ny) = transform_point(p.x, p.y, transforms, &mut rand);
                out.line_to(Point::new(nx, ny));
            }
            PathEl::QuadTo(p1, p2) => {
                let (x1, y1) = transform_point(p1.x, p1.y, transforms, &mut rand);
                let (x2, y2) = transform_point(p2.x, p2.y, transforms, &mut rand);
                out.quad_to(Point::new(x1, y1), Point::new(x2, y2));
            }
            PathEl::CurveTo(p1, p2, p3) => {
                let (x1, y1) = transform_point(p1.x, p1.y, transforms, &mut rand);
                let (x2, y2) = transform_point(p2.x, p2.y, transforms, &mut rand);
                let (x3, y3) = transform_point(p3.x, p3.y, transforms, &mut rand);
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
            let transformed = apply_transforms_to_path(&pen.path, &req.transforms, gid_u16 as u32);
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
    let (palette, fill_idx, grad0_idx, grad1_idx, block_idx, shadow_idx) =
        build_palette(colr_input);

    let has = |e: &str| colr_input.effects.iter().any(|s| s == e);
    let has_shadow = has("shadow");
    let has_3d = has("3d-blocks");
    let has_fill = has("fill");
    let has_gradient = has("gradient");

    let mut layer_list: Vec<Paint> = vec![];
    let mut base_glyph_paints: Vec<BaseGlyphPaint> = vec![];

    // Build paint tree for every glyph that has an outline (skip .notdef = GID 0
    // if desired, but including it is harmless).
    for gid_u16 in 0..glyph_count {
        let gid = GlyphId16::new(gid_u16);
        let first_layer = layer_list.len() as u32;

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
            let color_line = ColorLine::new(
                Extend::Pad,
                2,
                vec![
                    ColorStop::new(F2Dot14::from_f32(0.0), grad0_idx, F2Dot14::from_f32(1.0)),
                    ColorStop::new(F2Dot14::from_f32(1.0), grad1_idx, F2Dot14::from_f32(1.0)),
                ],
            );
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

fn build_palette(colr: &ColrInput) -> (Vec<ColorRecord>, u16, u16, u16, u16, u16) {
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

    (records, fill_idx, grad0_idx, grad1_idx, block_idx, shadow_idx)
}
