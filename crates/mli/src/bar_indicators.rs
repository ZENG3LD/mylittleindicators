//! 0.1.8 module paths over the contract cores. Not a second catalog.
//! `mlc-promo` calls these. MLC does not.

pub mod average {
    pub mod sma {
        pub use crate::indicators::average::sma::Sma;
    }
}

pub mod momentum {
    pub use crate::indicators::momentum::rsi::Rsi;
}

pub mod channels {
    pub use crate::indicators::channels::bollinger_bands::BollingerBands;
}
