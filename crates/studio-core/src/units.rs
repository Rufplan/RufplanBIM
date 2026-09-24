//! Conversion between internal millimetres and US architectural feet-inches.

/// mm per inch.
pub const MM_PER_IN: f64 = 25.4;
/// mm per foot.
pub const MM_PER_FT: f64 = 304.8;

/// Formats a length in mm as feet-inches rounded to 1/16", e.g. `12'-6 1/2"`.
pub fn format_ft_in(mm: f64) -> String {
    let sixteenths_total = (mm.abs() / MM_PER_IN * 16.0).round() as i64;
    let sign = if mm < 0.0 && sixteenths_total > 0 {
        "-"
    } else {
        ""
    };
    let feet = sixteenths_total / (12 * 16);
    let rem = sixteenths_total % (12 * 16);
    let inches = rem / 16;
    let mut num = rem % 16;
    let mut den = 16;
    while num > 0 && num % 2 == 0 {
        num /= 2;
        den /= 2;
    }
    let frac = if num > 0 {
        format!(" {num}/{den}")
    } else {
        String::new()
    };
    format!("{sign}{feet}'-{inches}{frac}\"")
}

/// Parses a length typed by a user and returns mm.
///
/// Accepts `12'-6 1/2"`, `12' 6"`, `12'6`, `12'`, `6"`, `6 1/2"`, `12.5'`, `3/4"`,
/// metric `3000mm`, `300cm`, `3m`, and a bare number, which is read as feet (Revit's
/// convention for ft-in fields).
pub fn parse_length(input: &str) -> Option<f64> {
    let s = input
        .trim()
        .to_ascii_lowercase()
        .replace(['\u{2019}', '\u{2032}'], "'")
        .replace(['\u{201d}', '\u{2033}'], "\"");
    if s.is_empty() {
        return None;
    }
    let (neg, s) = match s.strip_prefix('-') {
        Some(rest) if !rest.starts_with('\'') => (true, rest.trim().to_owned()),
        _ => (false, s.clone()),
    };
    let value = if let Some(v) = s.strip_suffix("mm") {
        v.trim().parse::<f64>().ok()?
    } else if let Some(v) = s.strip_suffix("cm") {
        v.trim().parse::<f64>().ok()? * 10.0
    } else if let Some(v) = s.strip_suffix('m') {
        v.trim().parse::<f64>().ok()? * 1000.0
    } else if let Some((ft, rest)) = s.split_once('\'') {
        let feet = if ft.trim().is_empty() {
            0.0
        } else {
            ft.trim().parse::<f64>().ok()?
        };
        let rest = rest
            .trim()
            .trim_start_matches('-')
            .trim()
            .trim_end_matches('"')
            .trim();
        let inches = if rest.is_empty() {
            0.0
        } else {
            parse_inches(rest)?
        };
        feet * MM_PER_FT + inches * MM_PER_IN
    } else if let Some(v) = s.strip_suffix('"') {
        parse_inches(v.trim())? * MM_PER_IN
    } else {
        s.parse::<f64>().ok()? * MM_PER_FT
    };
    if !value.is_finite() {
        return None;
    }
    Some(if neg { -value } else { value })
}

/// `6`, `6.5`, `6 1/2`, `1/2`, `6-1/2` → inches.
fn parse_inches(s: &str) -> Option<f64> {
    let s = s.replace('-', " ");
    let mut total = 0.0;
    for part in s.split_whitespace() {
        total += match part.split_once('/') {
            Some((n, d)) => {
                let d: f64 = d.parse().ok()?;
                if d == 0.0 {
                    return None;
                }
                n.parse::<f64>().ok()? / d
            }
            None => part.parse::<f64>().ok()?,
        };
    }
    Some(total)
}

/// Formats an area in mm² as square feet, e.g. `1,200 SF`.
pub fn format_area_sf(mm2: f64) -> String {
    let sf = (mm2 / (MM_PER_FT * MM_PER_FT)).round() as i64;
    let digits = sf.abs().to_string();
    let mut grouped = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            grouped.push(',');
        }
        grouped.push(c);
    }
    format!("{}{grouped} SF", if sf < 0 { "-" } else { "" })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-6
    }

    #[test]
    fn formats_common_lengths() {
        assert_eq!(format_ft_in(0.0), "0'-0\"");
        assert_eq!(format_ft_in(MM_PER_FT * 10.0), "10'-0\"");
        assert_eq!(
            format_ft_in(MM_PER_FT * 12.0 + MM_PER_IN * 6.5),
            "12'-6 1/2\""
        );
        assert_eq!(
            format_ft_in(MM_PER_IN * 5.5 + MM_PER_IN / 16.0 * 1.0),
            "0'-5 9/16\""
        );
        assert_eq!(format_ft_in(MM_PER_IN * 11.99), "1'-0\"");
        assert_eq!(format_ft_in(-MM_PER_FT), "-1'-0\"");
    }

    #[test]
    fn parses_feet_inches_forms() {
        let want = MM_PER_FT * 12.0 + MM_PER_IN * 6.5;
        for s in [
            "12'-6 1/2\"",
            "12' 6 1/2\"",
            "12'6.5",
            "12'-6-1/2\"",
            " 12' - 6 1/2 \"",
        ] {
            assert!(close(parse_length(s).unwrap(), want), "{s}");
        }
        assert!(close(parse_length("10'").unwrap(), MM_PER_FT * 10.0));
        assert!(close(parse_length("6\"").unwrap(), MM_PER_IN * 6.0));
        assert!(close(parse_length("3/4\"").unwrap(), MM_PER_IN * 0.75));
        assert!(close(parse_length("12.5'").unwrap(), MM_PER_FT * 12.5));
        assert!(close(parse_length("10").unwrap(), MM_PER_FT * 10.0));
        assert!(close(parse_length("-2'").unwrap(), -MM_PER_FT * 2.0));
    }

    #[test]
    fn parses_metric() {
        assert!(close(parse_length("3000mm").unwrap(), 3000.0));
        assert!(close(parse_length("30 cm").unwrap(), 300.0));
        assert!(close(parse_length("3m").unwrap(), 3000.0));
    }

    #[test]
    fn rejects_garbage() {
        for s in ["", "abc", "1/0\"", "12'x"] {
            assert_eq!(parse_length(s), None, "{s}");
        }
    }

    #[test]
    fn format_parse_round_trip() {
        for mm in [0.0, 101.6, 1234.5, 3048.0, 9999.0] {
            let back = parse_length(&format_ft_in(mm)).unwrap();
            assert!(
                (back - mm).abs() <= MM_PER_IN / 32.0 + 1e-9,
                "{mm} -> {back}"
            );
        }
    }

    #[test]
    fn area_in_square_feet() {
        assert_eq!(format_area_sf(MM_PER_FT * MM_PER_FT * 1200.0), "1,200 SF");
        assert_eq!(format_area_sf(MM_PER_FT * MM_PER_FT * 85.0), "85 SF");
    }
}
