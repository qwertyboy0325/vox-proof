use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectMemoryOpenMode {
    Writable,
    ReadOnly,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ProjectMemoryError {
    Io(String),
    Sqlite(String),
    ProjectAlreadyExists,
    ProjectNotFound,
    UnsupportedFormatVersion { found: u32, supported: u32 },
    CanonicalMismatch(String),
    WriterOwnershipHeld,
    ProjectNotWritable,
    InvalidProjectId,
    InvalidDisplayName,
    MissingSourceSessionId,
}

impl fmt::Display for ProjectMemoryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ProjectMemoryError {}
