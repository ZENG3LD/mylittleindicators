//! Per-stream consumer traits — one `*Consumer` trait per non-bar `StreamKind` (order book,
//! OI, funding, liquidation, …). A multi-stream indicator implements the traits for the
//! streams it subscribes to; the factory routes each sample to the matching `update_*`.
//! These define WHAT data the engine ingests, so they live under `engine/`.

pub mod order_book_consumer;
pub mod orderbook_delta_consumer;
pub mod funding_rate_consumer;
pub mod liquidation_consumer;
pub mod mark_price_consumer;
pub mod open_interest_consumer;
pub mod tick_consumer;
pub mod ticker_consumer;
pub mod hybrid_tick_book_consumer;

pub mod agg_trade_consumer;
pub mod auction_event_consumer;
pub mod basis_consumer;
pub mod block_trade_consumer;
pub mod composite_index_consumer;
pub mod funding_settlement_consumer;
pub mod historical_volatility_consumer;
pub mod index_price_consumer;
pub mod insurance_fund_consumer;
pub mod long_short_ratio_consumer;
pub mod market_warning_consumer;
pub mod option_greeks_consumer;
pub mod orderbook_l3_consumer;
pub mod predicted_funding_consumer;
pub mod risk_limit_consumer;
pub mod settlement_event_consumer;
pub mod volatility_index_consumer;

pub use order_book_consumer::OrderBookConsumer;
pub use orderbook_delta_consumer::OrderbookDeltaConsumer;
pub use funding_rate_consumer::FundingRateConsumer;
pub use liquidation_consumer::LiquidationConsumer;
pub use mark_price_consumer::MarkPriceConsumer;
pub use open_interest_consumer::OpenInterestConsumer;
pub use tick_consumer::TickConsumer;
pub use ticker_consumer::TickerConsumer;
pub use hybrid_tick_book_consumer::HybridTickBookConsumer;

pub use agg_trade_consumer::AggTradeConsumer;
pub use auction_event_consumer::AuctionEventConsumer;
pub use basis_consumer::BasisConsumer;
pub use block_trade_consumer::BlockTradeConsumer;
pub use composite_index_consumer::CompositeIndexConsumer;
pub use funding_settlement_consumer::FundingSettlementConsumer;
pub use historical_volatility_consumer::HistoricalVolatilityConsumer;
pub use index_price_consumer::IndexPriceConsumer;
pub use insurance_fund_consumer::InsuranceFundConsumer;
pub use long_short_ratio_consumer::LongShortRatioConsumer;
pub use market_warning_consumer::MarketWarningConsumer;
pub use option_greeks_consumer::OptionGreeksConsumer;
pub use orderbook_l3_consumer::OrderbookL3Consumer;
pub use predicted_funding_consumer::PredictedFundingConsumer;
pub use risk_limit_consumer::RiskLimitConsumer;
pub use settlement_event_consumer::SettlementEventConsumer;
pub use volatility_index_consumer::VolatilityIndexConsumer;
