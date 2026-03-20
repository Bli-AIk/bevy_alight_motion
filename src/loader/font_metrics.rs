use ttf_parser::Face;

/// Font metrics extracted from TTF/OTF files.
#[derive(Debug, Clone, Default)]
pub struct FontMetrics {
    /// Ascender height normalized to 1.0 = em height (from OS/2 usWinAscent).
    pub win_ascent: f32,
    /// Descender depth normalized to 1.0 = em height (from OS/2 usWinDescent).
    pub win_descent: f32,
    /// Units per em for normalization.
    pub units_per_em: u16,
    /// hhea ascender normalized to em height (positive).
    pub hhea_ascent: f32,
    /// hhea descender normalized to em height (positive).
    pub hhea_descent: f32,
}

impl FontMetrics {
    /// Calculate the vertical center offset relative to baseline.
    /// This is (win_ascent - win_descent) / 2.
    /// Note: win_descent is stored as positive value.
    pub fn win_center(&self) -> f32 {
        (self.win_ascent - self.win_descent) / 2.0
    }

    /// Compute the line height ratio matching Android's StaticLayout float-based metrics.
    /// AM uses (descent - ascent) * spacingMult, where ascent/descent are hhea float values.
    pub fn am_line_height_ratio(&self, _font_size: f32) -> f32 {
        self.hhea_ascent + self.hhea_descent
    }

    /// Compute line height ratio adjusted for CJK fallback fonts.
    /// When text contains CJK characters, Android uses the CJK fallback font's
    /// (Noto Sans CJK, hhea ratio ≈ 1.448) line metrics which are taller than
    /// most Latin fonts. We take the max of the primary font and CJK ratio.
    pub fn am_line_height_ratio_cjk_aware(&self, _font_size: f32, text: &str) -> f32 {
        let primary = self.hhea_ascent + self.hhea_descent;
        if contains_cjk(text) {
            const CJK_FALLBACK_LINE_HEIGHT_RATIO: f32 = 1.448;
            primary.max(CJK_FALLBACK_LINE_HEIGHT_RATIO)
        } else {
            primary
        }
    }

    /// Compute the Y offset to compensate for AM's StaticLayout `includePad(true)`.
    /// AM uses win metrics (usWinAscent/usWinDescent) for first/last line padding,
    /// then centers the padded box at the element position. Bevy centers based on
    /// hhea line height metrics. The height difference shifts the visual text center.
    pub fn include_pad_y_offset(&self, font_size: f32) -> f32 {
        let win_total = self.win_ascent + self.win_descent;
        let hhea_total = self.hhea_ascent + self.hhea_descent;
        -(win_total - hhea_total) * font_size
    }
}

/// Check if text contains CJK characters (U+4E00..U+9FFF, U+3400..U+4DBF, etc.)
/// Used to determine line height based on CJK fallback font metrics.
pub fn contains_cjk(text: &str) -> bool {
    text.chars().any(|c| {
        matches!(c,
            '\u{4E00}'..='\u{9FFF}'
            | '\u{3400}'..='\u{4DBF}'
            | '\u{3000}'..='\u{303F}'
            | '\u{3040}'..='\u{309F}'
            | '\u{30A0}'..='\u{30FF}'
            | '\u{AC00}'..='\u{D7AF}'
        )
    })
}

pub(super) fn extract_font_metrics(data: &[u8]) -> Option<FontMetrics> {
    let face = Face::parse(data, 0).ok()?;
    let upm = face.units_per_em();
    let (win_ascent, win_descent) = if let Some(os2) = face.tables().os2 {
        (
            os2.windows_ascender() as f32 / upm as f32,
            (-os2.windows_descender()) as f32 / upm as f32,
        )
    } else {
        (
            face.ascender() as f32 / upm as f32,
            (-face.descender()) as f32 / upm as f32,
        )
    };
    let hhea_ascent = face.ascender() as f32 / upm as f32;
    let hhea_descent = (-face.descender()) as f32 / upm as f32;

    Some(FontMetrics {
        win_ascent,
        win_descent,
        units_per_em: upm,
        hhea_ascent,
        hhea_descent,
    })
}
