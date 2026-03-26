use crate::config::{FitMode, UrlPattern};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedImagePath {
    pub variant: PathVariant,
    pub hash: String,
    pub ext: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathVariant {
    Preset(String),
    Dynamic(DynamicParams),
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DynamicParams {
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub fit: Option<FitMode>,
}

impl DynamicParams {
    pub fn is_empty(&self) -> bool {
        self.width.is_none() && self.height.is_none() && self.fit.is_none()
    }
}

/// Parse a /v/* or /d/* wildcard path into its components.
///
/// The `path` argument is the wildcard portion after /v/ or /d/.
/// `known_presets` is the list of preset keys defined in config.
pub fn parse_v_path(
    pattern: UrlPattern,
    path: &str,
    known_presets: &[&str],
) -> Option<ParsedImagePath> {
    println!("Parsing path: {path} with pattern {pattern:?}");

    let date_segs = match pattern {
        UrlPattern::Ymd => 3,
        UrlPattern::Ym => 2,
        UrlPattern::Y => 1,
        UrlPattern::Flat => 0,
    };

    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    let first = segments.first()?;

    let (variant, rest) = if known_presets.contains(first) {
        (PathVariant::Preset((*first).to_string()), &segments[1..])
    } else if let Some(dynamic) = parse_dynamic_spec(first) {
        (PathVariant::Dynamic(dynamic), &segments[1..])
    } else {
        (PathVariant::None, segments.as_slice())
    };

    let expected_len = date_segs + 1;
    if rest.len() != expected_len {
        return None;
    }

    let filename = rest.last()?;
    let (hash, ext) = filename.rsplit_once('.')?;

    if !is_valid_blake3_hex(hash) || ext.is_empty() {
        return None;
    }

    Some(ParsedImagePath {
        variant,
        hash: hash.to_string(),
        ext: ext.to_string(),
    })
}

fn parse_dynamic_spec(spec: &str) -> Option<DynamicParams> {
    let mut width = None;
    let mut height = None;
    let mut fit = None;

    for part in spec.split(',') {
        if let Some(value) = part.strip_prefix("w_") {
            let parsed = value.parse::<u32>().ok()?;
            if parsed == 0 || width.replace(parsed).is_some() {
                return None;
            }
            continue;
        }

        if let Some(value) = part.strip_prefix("h_") {
            let parsed = value.parse::<u32>().ok()?;
            if parsed == 0 || height.replace(parsed).is_some() {
                return None;
            }
            continue;
        }

        if let Some(value) = part.strip_prefix("fit_") {
            let parsed = parse_fit_mode(value)?;
            if fit.replace(parsed).is_some() {
                return None;
            }
            continue;
        }

        return None;
    }

    let params = DynamicParams { width, height, fit };
    if params.is_empty() {
        return None;
    }

    if params.fit.is_some() && (params.width.is_none() || params.height.is_none()) {
        return None;
    }

    Some(params)
}

fn parse_fit_mode(value: &str) -> Option<FitMode> {
    match value {
        "cover" => Some(FitMode::Cover),
        "contain" => Some(FitMode::Contain),
        "fill" => Some(FitMode::Fill),
        "inside" => Some(FitMode::Inside),
        "outside" => Some(FitMode::Outside),
        _ => None,
    }
}

fn is_valid_blake3_hex(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::{DynamicParams, ParsedImagePath, PathVariant, parse_v_path};
    use crate::config::{FitMode, UrlPattern};

    const HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    #[test]
    fn parses_preset_path() {
        let parsed = parse_v_path(UrlPattern::Ymd, &format!("thumb/2026/03/23/{HASH}.jpg"), &["thumb"])
            .unwrap();

        assert_eq!(
            parsed,
            ParsedImagePath {
                variant: PathVariant::Preset("thumb".to_string()),
                hash: HASH.to_string(),
                ext: "jpg".to_string(),
            }
        );
    }

    #[test]
    fn parses_dynamic_width_path() {
        let parsed = parse_v_path(UrlPattern::Ymd, &format!("w_800/2026/03/23/{HASH}.jpg"), &[])
            .unwrap();

        assert_eq!(
            parsed.variant,
            PathVariant::Dynamic(DynamicParams {
                width: Some(800),
                height: None,
                fit: None,
            })
        );
    }

    #[test]
    fn parses_dynamic_width_height_fit_path() {
        let parsed = parse_v_path(
            UrlPattern::Flat,
            &format!("w_800,h_600,fit_cover/{HASH}.jpg"),
            &[],
        )
        .unwrap();

        assert_eq!(
            parsed.variant,
            PathVariant::Dynamic(DynamicParams {
                width: Some(800),
                height: Some(600),
                fit: Some(FitMode::Cover),
            })
        );
    }

    #[test]
    fn rejects_invalid_dynamic_spec() {
        assert!(parse_v_path(
            UrlPattern::Flat,
            &format!("w_800,fit_cover/{HASH}.jpg"),
            &[]
        )
        .is_none());

        assert!(parse_v_path(
            UrlPattern::Flat,
            &format!("w_0/{HASH}.jpg"),
            &[]
        )
        .is_none());

        assert!(parse_v_path(
            UrlPattern::Flat,
            &format!("fit_cover/{HASH}.jpg"),
            &[]
        )
        .is_none());
    }

    #[test]
    fn falls_back_to_default_path_when_first_segment_is_not_variant() {
        let parsed = parse_v_path(UrlPattern::Y, &format!("2026/{HASH}.jpg"), &["thumb"]).unwrap();
        assert_eq!(parsed.variant, PathVariant::None);
    }
}
