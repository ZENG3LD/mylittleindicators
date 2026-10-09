//! Regression Models
//! Авторегрессионные модели и регрессионный анализ для временных рядов

pub mod arima;
pub mod garch;
pub mod var;
pub mod polynomial;

pub use arima::Arima;
pub use garch::{Garch, EGarch};
pub use var::Var;
pub use polynomial::{PolynomialRegression, TrendDirection}; 























// Relocated from statistics/ (Phase 1 reorg): OLS R-squared fit metric.
pub mod r_squared;
pub use r_squared::*;
