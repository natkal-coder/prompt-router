pub mod router;
pub mod scorer;

pub use router::{Router, RoutingDecision};
pub use scorer::RouteScorer;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Route {
    Local,
    Hybrid,
    CloudGemini,
    CloudClaude,
    CloudCursor,
}

impl std::fmt::Display for Route {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Local => write!(f, "LOCAL"),
            Self::Hybrid => write!(f, "HYBRID"),
            Self::CloudGemini => write!(f, "CLOUD:gemini"),
            Self::CloudClaude => write!(f, "CLOUD:claude"),
            Self::CloudCursor => write!(f, "CLOUD:cursor"),
        }
    }
}
