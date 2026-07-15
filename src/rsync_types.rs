use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;

/// Represents the type of update being performed
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateType {
    /// < - file is being transferred to remote host (sent)
    Sent,
    /// > - file is being transferred to local host (received)
    Received,
    /// c - local change/creation is occurring
    LocalChange,
    /// h - item is a hard link to another item
    HardLink,
    /// . - item is not being updated (but might have attribute changes)
    NotUpdated,
    /// * - rest of itemized-output contains a message (e.g. "deleting")
    Message,
}

/// Represents the type of file
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    /// f - regular file
    File,
    /// d - directory
    Directory,
    /// L - symlink
    Symlink,
    /// D - device
    Device,
    /// S - special file (named sockets, fifos)
    Special,
}

/// Represents the status of each attribute
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttributeStatus {
    /// . - no change
    NoChange,
    /// + - newly created item
    NewlyCreated,
    /// (space) - identical item
    Identical,
    /// ? - unknown attribute (when talking to older rsync)
    Unknown,
    /// Specific character indicating the attribute changed
    Changed,
}

/// Represents all the attributes that can be tracked
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attributes {
    /// c - checksum/content changed
    pub checksum: AttributeStatus,
    /// s - size changed
    pub size: AttributeStatus,
    /// t/T - modification time changed
    pub time: AttributeStatus,
    /// p - permissions changed
    pub permissions: AttributeStatus,
    /// o - owner changed
    pub owner: AttributeStatus,
    /// g - group changed
    pub group: AttributeStatus,
    /// u - reserved for future use
    pub reserved: AttributeStatus,
    /// a - ACL information changed
    pub acl: AttributeStatus,
    /// x - extended attribute information changed
    pub extended_attrs: AttributeStatus,
}

/// Represents a parsed rsync itemize-changes line
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RsyncItem {
    pub update_type: UpdateType,
    pub file_type: Option<FileType>,
    pub attributes: Option<Attributes>,
    pub path: PathBuf,
    /// For symlinks and hardlinks, this contains the target path
    pub link_target: Option<PathBuf>,
    /// For messages like "*deleting", this contains the message text
    pub message: Option<String>,
}

/// Result of parsing a single line
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseResult {
    /// Successfully parsed rsync item
    Item(RsyncItem),
    /// Line doesn't match expected format (might be other output)
    InvalidFormat(String),
    /// Empty line
    Empty,
}

impl fmt::Display for UpdateType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let c = match self {
            UpdateType::Sent => '<',
            UpdateType::Received => '>',
            UpdateType::LocalChange => 'c',
            UpdateType::HardLink => 'h',
            UpdateType::NotUpdated => '.',
            UpdateType::Message => '*',
        };
        write!(f, "{}", c)
    }
}

impl fmt::Display for FileType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let c = match self {
            FileType::File => 'f',
            FileType::Directory => 'd',
            FileType::Symlink => 'L',
            FileType::Device => 'D',
            FileType::Special => 'S',
        };
        write!(f, "{}", c)
    }
}

impl fmt::Display for AttributeStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let c = match self {
            AttributeStatus::NoChange => '.',
            AttributeStatus::NewlyCreated => '+',
            AttributeStatus::Identical => ' ',
            AttributeStatus::Unknown => '?',
            AttributeStatus::Changed => 'c', // This is context-dependent
        };
        write!(f, "{}", c)
    }
}

impl FromStr for UpdateType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "<" => Ok(UpdateType::Sent),
            ">" => Ok(UpdateType::Received),
            "c" => Ok(UpdateType::LocalChange),
            "h" => Ok(UpdateType::HardLink),
            "." => Ok(UpdateType::NotUpdated),
            "*" => Ok(UpdateType::Message),
            _ => Err(format!("Invalid update type: {}", s)),
        }
    }
}

impl FromStr for FileType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "f" => Ok(FileType::File),
            "d" => Ok(FileType::Directory),
            "L" => Ok(FileType::Symlink),
            "D" => Ok(FileType::Device),
            "S" => Ok(FileType::Special),
            _ => Err(format!("Invalid file type: {}", s)),
        }
    }
}

impl Default for Attributes {
    fn default() -> Self {
        Attributes {
            checksum: AttributeStatus::NoChange,
            size: AttributeStatus::NoChange,
            time: AttributeStatus::NoChange,
            permissions: AttributeStatus::NoChange,
            owner: AttributeStatus::NoChange,
            group: AttributeStatus::NoChange,
            reserved: AttributeStatus::NoChange,
            acl: AttributeStatus::NoChange,
            extended_attrs: AttributeStatus::NoChange,
        }
    }
}
