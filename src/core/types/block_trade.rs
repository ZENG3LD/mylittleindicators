//! Block trade — large OTC-reported trade event.

/// Block trade event.
///
/// Large trades reported separately from the regular order book (OTC desk or
/// block trade facility).
/// `symbol` omitted — mli is symbol-agnostic.
#[derive(Debug, Clone)]
pub struct BlockTrade {
    /// Exchange-assigned block trade identifier.
    pub block_id: String,
    /// Execution price.
    pub price: f64,
    /// Execution quantity in base asset.
    pub quantity: f64,
    /// `true` = buyer aggressor (buy block), `false` = seller aggressor (sell block).
    pub is_buy: bool,
    /// Event timestamp in milliseconds.
    pub timestamp: i64,
    /// `true` = price is expressed as implied volatility rather than a currency amount.
    pub is_iv: bool,
}
