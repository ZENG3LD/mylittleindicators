//! Enum classifying every kind of historical data stream that can feed indicators.

/// Classification of every historical data stream kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum StreamKind {
    /// OHLCV bars (default, backwards-compatible).
    Bar,
    /// Tick stream (every public trade).
    Tick,
    /// OrderBook snapshots.
    OrderBook,
    /// OrderBook delta updates.
    OrderbookDelta,
    /// Funding rate events.
    Funding,
    /// Mark price updates.
    MarkPrice,
    /// Open interest snapshots.
    OpenInterest,
    /// Liquidation events.
    Liquidation,
    /// 24h ticker snapshots.
    Ticker,
    /// Aggregated trade events.
    AggTrade,
    /// Long/short ratio snapshots.
    LongShortRatio,
    /// Option Greeks.
    OptionGreeks,
    /// Volatility index.
    VolatilityIndex,
    /// Historical volatility series.
    HistoricalVolatility,
    /// Basis (futures-spot).
    Basis,
    /// Index price.
    IndexPrice,
    /// Composite index.
    CompositeIndex,
    /// Insurance fund.
    InsuranceFund,
    /// Settlement event.
    Settlement,
    /// Block trade.
    BlockTrade,
    /// OrderBook L3 (per-order).
    OrderbookL3,
    /// Risk limit changes.
    RiskLimit,
    /// Predicted funding.
    PredictedFunding,
    /// Funding settlement.
    FundingSettlement,
    /// Auction event.
    Auction,
    /// Market warning.
    MarketWarning,
}

impl StreamKind {
    /// Returns name suitable for storage file path.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Bar => "bar",
            Self::Tick => "tick",
            Self::OrderBook => "orderbook",
            Self::OrderbookDelta => "orderbook_delta",
            Self::Funding => "funding",
            Self::MarkPrice => "mark_price",
            Self::OpenInterest => "open_interest",
            Self::Liquidation => "liquidation",
            Self::Ticker => "ticker",
            Self::AggTrade => "agg_trade",
            Self::LongShortRatio => "long_short_ratio",
            Self::OptionGreeks => "option_greeks",
            Self::VolatilityIndex => "volatility_index",
            Self::HistoricalVolatility => "historical_volatility",
            Self::Basis => "basis",
            Self::IndexPrice => "index_price",
            Self::CompositeIndex => "composite_index",
            Self::InsuranceFund => "insurance_fund",
            Self::Settlement => "settlement",
            Self::BlockTrade => "block_trade",
            Self::OrderbookL3 => "orderbook_l3",
            Self::RiskLimit => "risk_limit",
            Self::PredictedFunding => "predicted_funding",
            Self::FundingSettlement => "funding_settlement",
            Self::Auction => "auction",
            Self::MarketWarning => "market_warning",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::StreamKind;

    #[test]
    fn bar_as_str() {
        assert_eq!(StreamKind::Bar.as_str(), "bar");
    }

    #[test]
    fn all_kinds_unique_str() {
        use std::collections::HashSet;
        let all = [
            StreamKind::Bar,
            StreamKind::Tick,
            StreamKind::OrderBook,
            StreamKind::OrderbookDelta,
            StreamKind::Funding,
            StreamKind::MarkPrice,
            StreamKind::OpenInterest,
            StreamKind::Liquidation,
            StreamKind::Ticker,
            StreamKind::AggTrade,
            StreamKind::LongShortRatio,
            StreamKind::OptionGreeks,
            StreamKind::VolatilityIndex,
            StreamKind::HistoricalVolatility,
            StreamKind::Basis,
            StreamKind::IndexPrice,
            StreamKind::CompositeIndex,
            StreamKind::InsuranceFund,
            StreamKind::Settlement,
            StreamKind::BlockTrade,
            StreamKind::OrderbookL3,
            StreamKind::RiskLimit,
            StreamKind::PredictedFunding,
            StreamKind::FundingSettlement,
            StreamKind::Auction,
            StreamKind::MarketWarning,
        ];
        let strs: HashSet<&str> = all.iter().map(|k| k.as_str()).collect();
        assert_eq!(strs.len(), all.len(), "duplicate as_str values");
    }
}
