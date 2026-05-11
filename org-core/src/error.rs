use std::fmt;

#[derive(Debug)]
pub enum OrgModeError {
    InvalidDirectory(String),
    InvalidHeadingPath(String),
    InvalidElementId(String),
    InvalidAgendaViewType(String),
    WalkError(ignore::Error),
    GlobError(globset::Error),
    IoError(std::io::Error),
    ShellExpansionError(String),
    ConfigError(String),
    InvalidTodoKeyword(String),
    InvalidPriority(String),
    InvalidTitle(String),
    InvalidLevel(usize),
    InvalidTag(String),
    InvalidTimestamp { field: &'static str, value: String },
    InvalidPropertyKey(String),
    InvalidPropertyValue { key: String, reason: String },
    DuplicatePropertyKey(String),
    InvalidDatetreeDate(String),
    DatetreeDateWithoutFlag,
}

impl fmt::Display for OrgModeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OrgModeError::InvalidDirectory(path) => {
                write!(f, "Invalid or inaccessible directory: {path}")
            }
            OrgModeError::InvalidHeadingPath(path) => {
                write!(f, "Invalid heading path: {path}")
            }
            OrgModeError::InvalidElementId(id) => {
                write!(f, "Invalid element id: {id}")
            }
            OrgModeError::InvalidAgendaViewType(input) => {
                write!(f, "Invalid agenda view type: {input}")
            }
            OrgModeError::WalkError(err) => write!(f, "Error walking directory: {err}"),
            OrgModeError::GlobError(err) => write!(f, "Error with glob pattern: {err}"),
            OrgModeError::IoError(err) => write!(f, "IO error: {err}"),
            OrgModeError::ShellExpansionError(path) => write!(f, "Failed to expand path: {path}"),
            OrgModeError::ConfigError(msg) => write!(f, "Configuration error: {msg}"),
            OrgModeError::InvalidTodoKeyword(kw) => {
                write!(f, "Invalid TODO keyword: {kw}")
            }
            OrgModeError::InvalidPriority(p) => write!(f, "Invalid priority: {p}"),
            OrgModeError::InvalidTitle(reason) => write!(f, "Invalid heading title: {reason}"),
            OrgModeError::InvalidLevel(level) => {
                write!(f, "Invalid heading level: {level} (must be 1..=19)")
            }
            OrgModeError::InvalidTag(tag) => {
                write!(f, "Invalid tag '{tag}': tags must match [A-Za-z0-9_@]+")
            }
            OrgModeError::InvalidTimestamp { field, value } => write!(
                f,
                "Invalid timestamp for {field}: '{value}', expected YYYY-MM-DD or YYYY-MM-DD HH:MM"
            ),
            OrgModeError::InvalidPropertyKey(key) => write!(
                f,
                "Invalid property key '{key}': must be non-empty and contain only [A-Za-z0-9_-]"
            ),
            OrgModeError::InvalidPropertyValue { key, reason } => {
                write!(f, "Invalid property value for key '{key}': {reason}")
            }
            OrgModeError::DuplicatePropertyKey(key) => {
                write!(f, "Duplicate property key: '{key}'")
            }
            OrgModeError::InvalidDatetreeDate(value) => {
                write!(f, "Invalid datetree date '{value}': expected YYYY-MM-DD")
            }
            OrgModeError::DatetreeDateWithoutFlag => {
                write!(f, "Datetree date specified without enabling datetree")
            }
        }
    }
}

impl std::error::Error for OrgModeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            OrgModeError::WalkError(err) => Some(err),
            OrgModeError::IoError(err) => Some(err),
            _ => None,
        }
    }
}

impl From<ignore::Error> for OrgModeError {
    fn from(err: ignore::Error) -> Self {
        OrgModeError::WalkError(err)
    }
}

impl From<std::io::Error> for OrgModeError {
    fn from(err: std::io::Error) -> Self {
        OrgModeError::IoError(err)
    }
}

impl From<config::ConfigError> for OrgModeError {
    fn from(err: config::ConfigError) -> Self {
        OrgModeError::ConfigError(err.to_string())
    }
}

impl From<globset::Error> for OrgModeError {
    fn from(err: globset::Error) -> Self {
        OrgModeError::GlobError(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_invalid_priority() {
        let s = format!("{}", OrgModeError::InvalidPriority("X".to_string()));
        assert_eq!(s, "Invalid priority: X");
    }

    #[test]
    fn test_display_invalid_title() {
        let s = format!(
            "{}",
            OrgModeError::InvalidTitle("must not be empty".to_string())
        );
        assert_eq!(s, "Invalid heading title: must not be empty");
    }

    #[test]
    fn test_display_invalid_level() {
        let s = format!("{}", OrgModeError::InvalidLevel(42));
        assert_eq!(s, "Invalid heading level: 42 (must be 1..=19)");
    }

    #[test]
    fn test_display_invalid_tag() {
        let s = format!("{}", OrgModeError::InvalidTag("bad tag".to_string()));
        assert_eq!(s, "Invalid tag 'bad tag': tags must match [A-Za-z0-9_@]+");
    }

    #[test]
    fn test_display_invalid_timestamp() {
        let s = format!(
            "{}",
            OrgModeError::InvalidTimestamp {
                field: "scheduled",
                value: "tomorrow".to_string(),
            }
        );
        assert_eq!(
            s,
            "Invalid timestamp for scheduled: 'tomorrow', expected YYYY-MM-DD or YYYY-MM-DD HH:MM"
        );
    }

    #[test]
    fn test_display_invalid_property_key() {
        let s = format!(
            "{}",
            OrgModeError::InvalidPropertyKey("BAD KEY".to_string())
        );
        assert_eq!(
            s,
            "Invalid property key 'BAD KEY': must be non-empty and contain only [A-Za-z0-9_-]"
        );
    }

    #[test]
    fn test_display_invalid_property_value() {
        let s = format!(
            "{}",
            OrgModeError::InvalidPropertyValue {
                key: "CATEGORY".to_string(),
                reason: "contains newline".to_string(),
            }
        );
        assert_eq!(
            s,
            "Invalid property value for key 'CATEGORY': contains newline"
        );
    }

    #[test]
    fn test_display_duplicate_property_key() {
        let s = format!(
            "{}",
            OrgModeError::DuplicatePropertyKey("CATEGORY".to_string())
        );
        assert_eq!(s, "Duplicate property key: 'CATEGORY'");
    }

    #[test]
    fn test_display_invalid_datetree_date() {
        let s = format!(
            "{}",
            OrgModeError::InvalidDatetreeDate("tomorrow".to_string())
        );
        assert_eq!(s, "Invalid datetree date 'tomorrow': expected YYYY-MM-DD");
    }

    #[test]
    fn test_display_datetree_date_without_flag() {
        let s = format!("{}", OrgModeError::DatetreeDateWithoutFlag);
        assert_eq!(s, "Datetree date specified without enabling datetree");
    }
}
