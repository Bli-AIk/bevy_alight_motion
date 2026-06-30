//! # parsing.rs
//!
//! # 解析函数模块
//!
//! Parsing functions for vector and color string formats.
//! 向量和颜色字符串格式的解析函数。

/// Parse a comma-separated Vec3 string.
pub fn parse_vec3(s: &str) -> Result<[f32; 3], String> {
    let parts: Vec<&str> = s.split(',').collect();
    if parts.len() >= 3 {
        Ok([
            parts[0].trim().parse().map_err(|e| format!("{}", e))?,
            parts[1].trim().parse().map_err(|e| format!("{}", e))?,
            parts[2].trim().parse().map_err(|e| format!("{}", e))?,
        ])
    } else if parts.len() == 2 {
        Ok([
            parts[0].trim().parse().map_err(|e| format!("{}", e))?,
            parts[1].trim().parse().map_err(|e| format!("{}", e))?,
            0.0,
        ])
    } else if parts.len() == 1 && !s.is_empty() {
        let v: f32 = parts[0].trim().parse().map_err(|e| format!("{}", e))?;
        Ok([v, v, v])
    } else {
        Err(format!("Invalid vec3 format: {}", s))
    }
}

/// Parse a comma-separated Vec2 string.
pub fn parse_vec2(s: &str) -> Result<[f32; 2], String> {
    let parts: Vec<&str> = s.split(',').collect();
    if parts.len() >= 2 {
        Ok([
            parts[0].trim().parse().map_err(|e| format!("{}", e))?,
            parts[1].trim().parse().map_err(|e| format!("{}", e))?,
        ])
    } else if parts.len() == 1 && !s.is_empty() {
        let v: f32 = parts[0].trim().parse().map_err(|e| format!("{}", e))?;
        Ok([v, v])
    } else {
        Err(format!("Invalid vec2 format: {}", s))
    }
}

/// Parse color from #AARRGGBB format to [r, g, b, a] in 0.0-1.0 range.
pub fn parse_color(s: &str) -> Result<[f32; 4], String> {
    let s = s.trim_start_matches('#');
    if s.len() != 8 {
        return Err(format!("Invalid color format: #{}", s));
    }

    let a = u8::from_str_radix(&s[0..2], 16).map_err(|e| format!("{}", e))?;
    let r = u8::from_str_radix(&s[2..4], 16).map_err(|e| format!("{}", e))?;
    let g = u8::from_str_radix(&s[4..6], 16).map_err(|e| format!("{}", e))?;
    let b = u8::from_str_radix(&s[6..8], 16).map_err(|e| format!("{}", e))?;

    Ok([
        r as f32 / 255.0,
        g as f32 / 255.0,
        b as f32 / 255.0,
        a as f32 / 255.0,
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_vec3() {
        assert_eq!(parse_vec3("1.0,2.0,3.0").unwrap(), [1.0, 2.0, 3.0]);
        assert_eq!(
            parse_vec3("640.0, 480.0, 0.0").unwrap(),
            [640.0, 480.0, 0.0]
        );
        assert_eq!(parse_vec3("-1.5,2.5,0").unwrap(), [-1.5, 2.5, 0.0]);
    }

    #[test]
    fn test_parse_vec2() {
        assert_eq!(parse_vec2("100.0,200.0").unwrap(), [100.0, 200.0]);
        assert_eq!(parse_vec2("1.5, 2.5").unwrap(), [1.5, 2.5]);
    }

    #[test]
    fn test_parse_color() {
        let color = parse_color("#ff000000").unwrap();
        assert_eq!(color, [0.0, 0.0, 0.0, 1.0]);

        let color = parse_color("#ffffffff").unwrap();
        assert_eq!(color, [1.0, 1.0, 1.0, 1.0]);

        let color = parse_color("#80ff0000").unwrap();
        assert!((color[0] - 1.0).abs() < 0.01);
        assert!((color[1] - 0.0).abs() < 0.01);
        assert!((color[2] - 0.0).abs() < 0.01);
        assert!((color[3] - 0.5).abs() < 0.01);
    }
}
