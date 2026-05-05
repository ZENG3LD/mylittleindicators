//! Regression Models
//! Авторегрессионные модели и регрессионный анализ для временных рядов

pub mod arima;
pub mod garch;
pub mod var;
pub mod polynomial;
pub mod regression_catalog;

pub use arima::{Arima, ArimaX};
pub use garch::{Garch, EGarch};
pub use var::Var;
pub use polynomial::{PolynomialRegression, TrendDirection}; 






















