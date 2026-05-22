//! Single timestamped event in a merged historical stream.

use crate::core::types::{
    AggTrade, AuctionEvent, Basis, Bar, BlockTrade, CompositeIndex, FundingRate,
    FundingSettlement, HistoricalVolatility, IndexPrice, InsuranceFund, Liquidation,
    LongShortRatio, MarkPrice, MarketWarning, OpenInterest, OptionGreeks, OrderBook,
    OrderbookDelta, OrderbookL3Event, PredictedFunding, RiskLimit, SettlementEvent,
    Tick, Ticker, VolatilityIndex,
};

/// One historical event with its timestamp.
///
/// Used to feed the backtest warmup pipeline, which dispatches each variant to
/// the appropriate `update_*` method on indicator instances.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum TimedEvent {
    Bar(Bar),
    Tick(Tick),
    OrderBook(OrderBook),
    OrderbookDelta(OrderbookDelta),
    Funding(FundingRate),
    MarkPrice(MarkPrice),
    OpenInterest(OpenInterest),
    Liquidation(Liquidation),
    Ticker(Ticker),
    AggTrade(AggTrade),
    LongShortRatio(LongShortRatio),
    OptionGreeks(OptionGreeks),
    VolatilityIndex(VolatilityIndex),
    HistoricalVolatility(HistoricalVolatility),
    Basis(Basis),
    IndexPrice(IndexPrice),
    CompositeIndex(CompositeIndex),
    InsuranceFund(InsuranceFund),
    Settlement(SettlementEvent),
    BlockTrade(BlockTrade),
    OrderbookL3(OrderbookL3Event),
    RiskLimit(RiskLimit),
    PredictedFunding(PredictedFunding),
    FundingSettlement(FundingSettlement),
    Auction(AuctionEvent),
    MarketWarning(MarketWarning),
}

impl TimedEvent {
    /// Returns the event timestamp in milliseconds.
    ///
    /// Note: `Bar` uses field `time`, `Tick` uses field `time`. All other types use `timestamp`.
    pub fn timestamp_ms(&self) -> i64 {
        match self {
            Self::Bar(b) => b.time,
            Self::Tick(t) => t.time,
            Self::OrderBook(b) => b.timestamp,
            Self::OrderbookDelta(d) => d.timestamp,
            Self::Funding(f) => f.timestamp,
            Self::MarkPrice(m) => m.timestamp,
            Self::OpenInterest(oi) => oi.timestamp,
            Self::Liquidation(l) => l.timestamp,
            Self::Ticker(t) => t.timestamp,
            Self::AggTrade(a) => a.timestamp,
            Self::LongShortRatio(r) => r.timestamp,
            Self::OptionGreeks(g) => g.timestamp,
            Self::VolatilityIndex(v) => v.timestamp,
            Self::HistoricalVolatility(h) => h.timestamp,
            Self::Basis(b) => b.timestamp,
            Self::IndexPrice(ip) => ip.timestamp,
            Self::CompositeIndex(ci) => ci.timestamp,
            Self::InsuranceFund(i) => i.timestamp,
            Self::Settlement(s) => s.timestamp,
            Self::BlockTrade(bt) => bt.timestamp,
            Self::OrderbookL3(l3) => l3.timestamp,
            Self::RiskLimit(r) => r.timestamp,
            Self::PredictedFunding(pf) => pf.timestamp,
            Self::FundingSettlement(fs) => fs.timestamp,
            Self::Auction(a) => a.timestamp,
            Self::MarketWarning(w) => w.timestamp,
        }
    }

    pub fn stream_kind(&self) -> super::StreamKind {
        use super::StreamKind;
        match self {
            Self::Bar(_) => StreamKind::Bar,
            Self::Tick(_) => StreamKind::Tick,
            Self::OrderBook(_) => StreamKind::OrderBook,
            Self::OrderbookDelta(_) => StreamKind::OrderbookDelta,
            Self::Funding(_) => StreamKind::Funding,
            Self::MarkPrice(_) => StreamKind::MarkPrice,
            Self::OpenInterest(_) => StreamKind::OpenInterest,
            Self::Liquidation(_) => StreamKind::Liquidation,
            Self::Ticker(_) => StreamKind::Ticker,
            Self::AggTrade(_) => StreamKind::AggTrade,
            Self::LongShortRatio(_) => StreamKind::LongShortRatio,
            Self::OptionGreeks(_) => StreamKind::OptionGreeks,
            Self::VolatilityIndex(_) => StreamKind::VolatilityIndex,
            Self::HistoricalVolatility(_) => StreamKind::HistoricalVolatility,
            Self::Basis(_) => StreamKind::Basis,
            Self::IndexPrice(_) => StreamKind::IndexPrice,
            Self::CompositeIndex(_) => StreamKind::CompositeIndex,
            Self::InsuranceFund(_) => StreamKind::InsuranceFund,
            Self::Settlement(_) => StreamKind::Settlement,
            Self::BlockTrade(_) => StreamKind::BlockTrade,
            Self::OrderbookL3(_) => StreamKind::OrderbookL3,
            Self::RiskLimit(_) => StreamKind::RiskLimit,
            Self::PredictedFunding(_) => StreamKind::PredictedFunding,
            Self::FundingSettlement(_) => StreamKind::FundingSettlement,
            Self::Auction(_) => StreamKind::Auction,
            Self::MarketWarning(_) => StreamKind::MarketWarning,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::TimedEvent;
    use crate::core::types::{Bar, FundingRate};

    #[test]
    fn bar_timestamp_uses_time_field() {
        let bar = Bar::new(1_000_000, 100.0, 110.0, 90.0, 105.0, 1000.0);
        let ev = TimedEvent::Bar(bar);
        assert_eq!(ev.timestamp_ms(), 1_000_000);
    }

    #[test]
    fn funding_timestamp() {
        let f = FundingRate {
            rate: 0.0001,
            next_funding_time: None,
            timestamp: 2_000_000,
        };
        let ev = TimedEvent::Funding(f);
        assert_eq!(ev.timestamp_ms(), 2_000_000);
        assert_eq!(ev.stream_kind(), crate::data_loader::StreamKind::Funding);
    }
}
