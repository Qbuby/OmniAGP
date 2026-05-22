use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum License {
    CC0,
    CCBY,
    CCBYSA,
    MIT,
    Commercial,
    Custom(String),
}

impl License {
    pub fn allows_commercial_use(&self) -> bool {
        matches!(self, Self::CC0 | Self::CCBY | Self::MIT | Self::Commercial)
    }

    pub fn requires_attribution(&self) -> bool {
        matches!(self, Self::CCBY | Self::CCBYSA)
    }

    pub fn is_copyleft(&self) -> bool {
        matches!(self, Self::CCBYSA)
    }
}

impl std::fmt::Display for License {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CC0 => write!(f, "CC0 (Public Domain)"),
            Self::CCBY => write!(f, "CC-BY 4.0"),
            Self::CCBYSA => write!(f, "CC-BY-SA 4.0"),
            Self::MIT => write!(f, "MIT"),
            Self::Commercial => write!(f, "Commercial License"),
            Self::Custom(s) => write!(f, "Custom: {}", s),
        }
    }
}
