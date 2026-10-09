// Auction-event indicators (Phase 1 reorg 2026-06-16).
// Dissolved out of risk_funding/ — these consume the AuctionEvent stream.
pub mod auction_imbalance;
pub mod auction_liquidity_score;
pub mod auction_price_deviation;

pub use auction_imbalance::*;
pub use auction_liquidity_score::*;
pub use auction_price_deviation::*;
