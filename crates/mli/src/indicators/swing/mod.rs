// Swing detection + swing-point primitives (consolidated 2026-06-15 reorg).
// Gathered from momentum/ (Highest/Lowest primitives), events/ (SwingDetection
// — the canonical swing node), trend_stop/ (SwingStop), statistical_scoring/
// (SwingAge, SwingStrengthScore). NO dedup yet — Phase 2 collapses the three
// analytics that roll their own swing-finding onto SwingDetection.
pub mod highest;
pub mod lowest;
pub mod swing_detection;
pub mod swing_stop;
pub mod swing_age;
pub mod swing_strength_score;

pub use highest::*;
pub use lowest::*;
pub use swing_detection::*;
pub use swing_stop::SwingStop;
pub use swing_age::SwingAge;
pub use swing_strength_score::SwingStrengthScore;

// Relocated from chaos/ (Phase 1 reorg 2026-06-16): 5-bar swing-high/low (Williams Fractals), a pivot/swing detector — not fractal-dimension math.
pub mod williams_fractals;
pub use williams_fractals::*;
