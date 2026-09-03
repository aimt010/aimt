/// Bare level names without `@` prefix, lowercase.
/// Canonical source of truth for the seven AIMT levels.
pub const SUPPORTED_LEVELS: &[&str] = &["aimt", "map", "domain", "region", "node", "file", "frame"];

/// Level markers with `@` prefix, as they appear on line 1.
pub const SUPPORTED_LEVEL_MARKERS: &[&str] = &[
    "@aimt", "@map", "@domain", "@region", "@node", "@file", "@frame",
];

/// Returns true if `name` is one of the seven bare level names.
pub fn is_supported_level(name: &str) -> bool {
    SUPPORTED_LEVELS.contains(&name)
}

/// Returns true if `marker` is one of the seven `@level` markers.
pub fn is_supported_level_marker(marker: &str) -> bool {
    marker.strip_prefix('@').is_some_and(is_supported_level)
}

/// Closed set of seven AIMT levels — explicit domain representation.
/// `@relation` is NOT a level. Canonical order matches `SUPPORTED_LEVELS`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Level {
    Aimt,
    Map,
    Domain,
    Region,
    Node,
    File,
    Frame,
}

impl Level {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Aimt => "aimt",
            Self::Map => "map",
            Self::Domain => "domain",
            Self::Region => "region",
            Self::Node => "node",
            Self::File => "file",
            Self::Frame => "frame",
        }
    }

    pub fn marker(&self) -> &'static str {
        match self {
            Self::Aimt => "@aimt",
            Self::Map => "@map",
            Self::Domain => "@domain",
            Self::Region => "@region",
            Self::Node => "@node",
            Self::File => "@file",
            Self::Frame => "@frame",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        if !is_supported_level(s) {
            return None;
        }
        match s {
            "aimt" => Some(Self::Aimt),
            "map" => Some(Self::Map),
            "domain" => Some(Self::Domain),
            "region" => Some(Self::Region),
            "node" => Some(Self::Node),
            "file" => Some(Self::File),
            "frame" => Some(Self::Frame),
            _ => None,
        }
    }

    pub fn from_bare(bare: &str) -> Option<Self> {
        Self::from_str(bare)
    }

    /// All seven levels in canonical order.
    pub fn all() -> &'static [&'static str] {
        SUPPORTED_LEVELS
    }
}
