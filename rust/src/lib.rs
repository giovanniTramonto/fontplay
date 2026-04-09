use serde::Serialize;
use skrifa::{
    instance::LocationRef,
    outline::{DrawSettings, OutlinePen},
    prelude::Size,
    FontRef, MetadataProvider,
};
use read_fonts::TableProvider;
use wasm_bindgen::prelude::*;

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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GlyphSvg {
    pub d: String,
    pub view_box: String,
    pub advance_width: f32,
    pub height: f32,
    pub found: bool,
}

// ─── SVG pen ────────────────────────────────────────────────────────────────

struct SvgPen {
    d: String,
    ascender: f32,
}

impl SvgPen {
    fn new(ascender: f32) -> Self {
        Self { d: String::new(), ascender }
    }

    // Fonts have y-up; SVG has y-down — flip around the ascender line.
    #[inline]
    fn flip_y(&self, y: f32) -> f32 {
        self.ascender - y
    }
}

impl OutlinePen for SvgPen {
    fn move_to(&mut self, x: f32, y: f32) {
        self.d.push_str(&format!("M{:.3},{:.3}", x, self.flip_y(y)));
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.d.push_str(&format!("L{:.3},{:.3}", x, self.flip_y(y)));
    }

    fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
        self.d.push_str(&format!(
            "Q{:.3},{:.3},{:.3},{:.3}",
            cx,
            self.flip_y(cy),
            x,
            self.flip_y(y),
        ));
    }

    fn curve_to(&mut self, cx1: f32, cy1: f32, cx2: f32, cy2: f32, x: f32, y: f32) {
        self.d.push_str(&format!(
            "C{:.3},{:.3},{:.3},{:.3},{:.3},{:.3}",
            cx1,
            self.flip_y(cy1),
            cx2,
            self.flip_y(cy2),
            x,
            self.flip_y(y),
        ));
    }

    fn close(&mut self) {
        self.d.push('Z');
    }
}

// ─── WASM exports ────────────────────────────────────────────────────────────

/// Returns basic font metrics as a JS object.
#[wasm_bindgen]
pub fn get_font_info(data: &[u8]) -> Result<JsValue, JsValue> {
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

/// Renders a single Unicode code point to an SVG path.
/// Returns a `GlyphSvg` object or throws if the font cannot be loaded.
#[wasm_bindgen]
pub fn char_to_svg(data: &[u8], char_code: u32) -> Result<JsValue, JsValue> {
    let font = FontRef::new(data).map_err(|e| JsValue::from_str(&e.to_string()))?;

    let head = font.head().map_err(|e| JsValue::from_str(&e.to_string()))?;
    let hhea = font.hhea().map_err(|e| JsValue::from_str(&e.to_string()))?;

    let upem = head.units_per_em() as f32;
    let ascender = hhea.ascender().to_i16() as f32;
    let descender = hhea.descender().to_i16() as f32; // negative
    let height = ascender - descender;

    let ch = char::from_u32(char_code).ok_or_else(|| JsValue::from_str("Invalid char code"))?;

    let charmap = font.charmap();
    let Some(glyph_id) = charmap.map(ch) else {
        // Return an empty placeholder so the caller can still reserve space.
        let result = GlyphSvg {
            d: String::new(),
            view_box: format!("0 0 {upem} {height}"),
            advance_width: upem * 0.5,
            height,
            found: false,
        };
        return serde_wasm_bindgen::to_value(&result)
            .map_err(|e| JsValue::from_str(&e.to_string()));
    };

    // Glyph metrics (advance width)
    let metrics = font.glyph_metrics(Size::new(upem), LocationRef::default());
    let advance_width = metrics.advance_width(glyph_id).unwrap_or(upem * 0.6);

    // Draw outline
    let outlines = font.outline_glyphs();
    let mut pen = SvgPen::new(ascender);

    if let Some(glyph) = outlines.get(glyph_id) {
        let settings = DrawSettings::unhinted(Size::new(upem), LocationRef::default());
        glyph
            .draw(settings, &mut pen)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
    }

    let result = GlyphSvg {
        d: pen.d,
        view_box: format!("0 0 {advance_width} {height}"),
        advance_width,
        height,
        found: true,
    };

    serde_wasm_bindgen::to_value(&result).map_err(|e| JsValue::from_str(&e.to_string()))
}
