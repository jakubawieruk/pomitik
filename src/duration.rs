use regex::Regex;
use std::fmt;

#[derive(Debug, PartialEq)]
pub struct Duration {
    pub total_secs: u64,
}

impl Duration {
    pub fn parse(input: &str) -> Result<Self, String> {
        let re = Regex::new(r"^(?:(\d+)h)?(?:(\d+)m)?(?:(\d+)s)?$").unwrap();
        let caps = re.captures(input).ok_or_else(|| {
            format!("Invalid duration format: '{input}'")
        })?;

        let hours: u64 = caps.get(1).map_or(0, |m| m.as_str().parse().unwrap());
        let minutes: u64 = caps.get(2).map_or(0, |m| m.as_str().parse().unwrap());
        let seconds: u64 = caps.get(3).map_or(0, |m| m.as_str().parse().unwrap());

        let total_secs = hours * 3600 + minutes * 60 + seconds;

        if total_secs == 0 {
            return Err("Duration must be greater than zero".to_string());
        }

        Ok(Duration { total_secs })
    }

    pub fn format_hms(&self) -> String {
        let h = self.total_secs / 3600;
        let m = (self.total_secs % 3600) / 60;
        let s = self.total_secs % 60;

        if h > 0 {
            format!("{h}:{m:02}:{s:02}")
        } else {
            format!("{m}:{s:02}")
        }
    }
}

impl fmt::Display for Duration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.format_hms())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minutes_only() {
        assert_eq!(Duration::parse("25m").unwrap().total_secs, 1500);
    }

    #[test]
    fn parse_seconds_only() {
        assert_eq!(Duration::parse("90s").unwrap().total_secs, 90);
    }

    #[test]
    fn parse_hours_only() {
        assert_eq!(Duration::parse("1h").unwrap().total_secs, 3600);
    }

    #[test]
    fn parse_hours_and_minutes() {
        assert_eq!(Duration::parse("1h30m").unwrap().total_secs, 5400);
    }

    #[test]
    fn parse_all_components() {
        assert_eq!(Duration::parse("1h30m15s").unwrap().total_secs, 5415);
    }

    #[test]
    fn parse_invalid_returns_error() {
        assert!(Duration::parse("abc").is_err());
    }

    #[test]
    fn parse_zero_returns_error() {
        assert!(Duration::parse("0m").is_err());
    }

    #[test]
    fn format_minutes_and_seconds() {
        assert_eq!(Duration { total_secs: 1500 }.format_hms(), "25:00");
    }

    #[test]
    fn format_with_hours() {
        assert_eq!(Duration { total_secs: 5415 }.format_hms(), "1:30:15");
    }

    #[test]
    fn format_seconds_only() {
        assert_eq!(Duration { total_secs: 45 }.format_hms(), "0:45");
    }

    #[test]
    fn format_human_readable() {
        assert_eq!(Duration { total_secs: 1500 }.to_string(), "25:00");
    }
}
