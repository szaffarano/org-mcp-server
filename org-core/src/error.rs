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
