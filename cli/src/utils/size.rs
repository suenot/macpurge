use bytesize::ByteSize;

/// Format bytes to human-readable string
pub fn format_size(bytes: u64) -> String {
    ByteSize(bytes).to_string_as(true)
}

/// Parse human-readable size string to bytes
pub fn parse_size(s: &str) -> Option<u64> {
    let s = s.trim().to_uppercase();

    let (num_part, unit) = if s.ends_with("GB") {
        (&s[..s.len()-2], 1024 * 1024 * 1024)
    } else if s.ends_with("MB") {
        (&s[..s.len()-2], 1024 * 1024)
    } else if s.ends_with("KB") {
        (&s[..s.len()-2], 1024)
    } else if s.ends_with("B") {
        (&s[..s.len()-1], 1)
    } else {
        (s.as_str(), 1)
    };

    num_part.trim().parse::<f64>().ok().map(|n| (n * unit as f64) as u64)
}

/// Format duration in seconds to human-readable string
pub fn format_duration(seconds: u64) -> String {
    if seconds < 60 {
        format!("{}s", seconds)
    } else if seconds < 3600 {
        format!("{}m {}s", seconds / 60, seconds % 60)
    } else {
        format!("{}h {}m", seconds / 3600, (seconds % 3600) / 60)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_size() {
        assert!(!format_size(1024).is_empty());
        assert!(!format_size(1024 * 1024).is_empty());
    }

    #[test]
    fn test_parse_size() {
        assert_eq!(parse_size("1024"), Some(1024));
        assert_eq!(parse_size("1KB"), Some(1024));
        assert_eq!(parse_size("1MB"), Some(1024 * 1024));
    }
}
