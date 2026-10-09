//! Unified input sample — one enum covering every `StreamKind` an indicator
//! can consume. The box-free [`super::super::bar_indicators::contract_factory::ContractFactory`]
//! dispatches through this instead of per-stream `update_*` methods, so the
//! factory macro and callers need only one feed path.

use crate::core::types::{
    AggTrade, AuctionEvent, Basis, BlockTrade, CompositeIndex, FundingRate, FundingSettlement,
    HistoricalVolatility, IndexPrice, InsuranceFund, Liquidation, LongShortRatio, MarkPrice,
    MarketWarning, OptionGreeks, OpenInterest, OrderBook, OrderbookDelta, OrderbookL3Event,
    PredictedFunding, RiskLimit, SettlementEvent, Tick, Ticker, VolatilityIndex,
};
use crate::engine::stream_kind::StreamKind;

/// One market sample — the unified input type for all stream kinds.
///
/// Lifetime `'a` borrows the event data from the caller; no heap allocation.
/// The `Bar` variant carries the five OHLCV scalars inline. All other variants
/// borrow from an existing struct.
#[derive(Debug, Clone, Copy)]
pub enum MarketSample<'a> {
    /// OHLCV bar. Pure price/volume — NO timestamp. The bar's wall-clock travels as the
    /// orthogonal `feed(ts, sample)` coordinate (the THIRD axis), forwarded to a core only
    /// when it declares `Time` / `+time`; time stays out of the ordinary price core.
    Bar {
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: f64,
    },
    /// Single public trade (tick).
    Tick(&'a Tick),
    /// Public trade combined with the current L2 order-book snapshot.
    TickWithBook(&'a Tick, &'a OrderBook),
    /// L2 order-book snapshot.
    OrderBook(&'a OrderBook),
    /// Incremental L2 order-book delta.
    OrderbookDelta(&'a OrderbookDelta),
    /// Funding rate event.
    Funding(&'a FundingRate),
    /// Open interest snapshot.
    OpenInterest(&'a OpenInterest),
    /// Mark price update.
    MarkPrice(&'a MarkPrice),
    /// 24-hour ticker snapshot.
    Ticker(&'a Ticker),
    /// Liquidation event.
    Liquidation(&'a Liquidation),
    /// Long/short ratio snapshot.
    LongShortRatio(&'a LongShortRatio),
    /// Aggregated trade event.
    AggTrade(&'a AggTrade),
    /// Composite index update.
    CompositeIndex(&'a CompositeIndex),
    /// Index price update.
    IndexPrice(&'a IndexPrice),
    /// Historical volatility series point.
    HistoricalVolatility(&'a HistoricalVolatility),
    /// Insurance fund update.
    InsuranceFund(&'a InsuranceFund),
    /// Basis (futures-spot spread).
    Basis(&'a Basis),
    /// Option Greeks snapshot.
    OptionGreeks(&'a OptionGreeks),
    /// Volatility index update.
    VolatilityIndex(&'a VolatilityIndex),
    /// Block trade event.
    BlockTrade(&'a BlockTrade),
    /// Market warning event.
    MarketWarning(&'a MarketWarning),
    /// Per-order L3 order-book event.
    OrderbookL3(&'a OrderbookL3Event),
    /// Settlement event.
    Settlement(&'a SettlementEvent),
    /// Risk limit change event.
    RiskLimit(&'a RiskLimit),
    /// Predicted funding rate.
    PredictedFunding(&'a PredictedFunding),
    /// Funding settlement event.
    FundingSettlement(&'a FundingSettlement),
    /// Auction (open/close) event.
    Auction(&'a AuctionEvent),
}

impl MarketSample<'_> {
    /// The [`StreamKind`] this sample corresponds to.
    ///
    /// `TickWithBook` maps to `Tick` (the primary stream); the book is auxiliary.
    pub fn kind(&self) -> StreamKind {
        match self {
            Self::Bar { .. } => StreamKind::Bar,
            Self::Tick(_) | Self::TickWithBook(_, _) => StreamKind::Tick,
            Self::OrderBook(_) => StreamKind::OrderBook,
            Self::OrderbookDelta(_) => StreamKind::OrderbookDelta,
            Self::Funding(_) => StreamKind::Funding,
            Self::OpenInterest(_) => StreamKind::OpenInterest,
            Self::MarkPrice(_) => StreamKind::MarkPrice,
            Self::Ticker(_) => StreamKind::Ticker,
            Self::Liquidation(_) => StreamKind::Liquidation,
            Self::LongShortRatio(_) => StreamKind::LongShortRatio,
            Self::AggTrade(_) => StreamKind::AggTrade,
            Self::CompositeIndex(_) => StreamKind::CompositeIndex,
            Self::IndexPrice(_) => StreamKind::IndexPrice,
            Self::HistoricalVolatility(_) => StreamKind::HistoricalVolatility,
            Self::InsuranceFund(_) => StreamKind::InsuranceFund,
            Self::Basis(_) => StreamKind::Basis,
            Self::OptionGreeks(_) => StreamKind::OptionGreeks,
            Self::VolatilityIndex(_) => StreamKind::VolatilityIndex,
            Self::BlockTrade(_) => StreamKind::BlockTrade,
            Self::MarketWarning(_) => StreamKind::MarketWarning,
            Self::OrderbookL3(_) => StreamKind::OrderbookL3,
            Self::Settlement(_) => StreamKind::Settlement,
            Self::RiskLimit(_) => StreamKind::RiskLimit,
            Self::PredictedFunding(_) => StreamKind::PredictedFunding,
            Self::FundingSettlement(_) => StreamKind::FundingSettlement,
            Self::Auction(_) => StreamKind::Auction,
        }
    }
}
