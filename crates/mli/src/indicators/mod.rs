// Indicator catalog — every category of contract-backed indicator (incl. the absorbed
// event-detectors, which are ordinary indicators). The contract engine (`crate::engine`)
// registers all of these; impls live here in category folders, a detector being just a
// kind of indicator.

pub mod average;
pub mod momentum;
pub mod volatility;
pub mod ratio;
// pub mod box_factory;
pub mod book;
pub mod volume;
pub mod clusters;
pub mod chaos;
pub mod entropy;
pub mod regression;
pub mod signal_processing;
pub mod kalman;
pub mod channels;
pub mod accumulation;
pub mod levels;
pub mod calendar;
pub mod auction;
pub mod risk;
pub mod settlement;
pub mod composite_index;
pub mod structure;
pub mod signal_logic;
pub mod statistical_scoring;
pub mod divergence;
pub mod regime;
pub mod swing;
pub mod trend;
pub mod trend_stop;
pub mod candles;
pub mod statistics;
pub mod utils;

pub mod tick_advanced;
pub mod liquidations;
pub mod sentiment;
pub mod book_advanced;
pub mod index_basis;
pub mod volatility_advanced;
pub mod greeks;
pub mod stress;
pub mod microstructure;
pub mod open_interest;
pub mod funding_advanced;
pub mod mark_price_advanced;
pub mod composites;
pub mod ticker_advanced;
