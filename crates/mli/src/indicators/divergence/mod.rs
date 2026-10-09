// Price/oscillator divergence detector. Relocated from events/ (events decompose
// to their domain folders). The naive lookback variant was removed 2026-06-23 —
// the swing-pivot implementation is now the canonical `Divergence` (regular+hidden,
// multi-pivot linear-regression over swing points, optional ATR-normalised strength;
// box-free inner oscillator slot). Both were box-free; the richer algorithm won.
pub mod divergence;

pub use divergence::*;
