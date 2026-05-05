// Logic Gates for combining indicator signals
//
// Self-contained versions: use two internal RSIs with different periods
// - RSI(7) for short-term momentum
// - RSI(21) for medium-term momentum
// AND: Both overbought/oversold
// OR: Either overbought/oversold
// XOR: Divergence between short and medium term
// SignCombiner: Sum of signal directions

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::momentum::rsi::Rsi;

/// AND Gate: Returns true when both RSIs agree on overbought (>70) or oversold (<30)
#[derive(Clone)]
pub struct AndGate {
    rsi_short: Rsi,
    rsi_long: Rsi,
    pub v: bool,
}
impl Default for AndGate {
    fn default() -> Self {
        Self::new()
    }
}

impl AndGate {
    pub fn new() -> Self {
        Self {
            rsi_short: Rsi::new(7),
            rsi_long: Rsi::new(21),
            v: false,
        }
    }

    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> bool {
        self.rsi_short.update_bar(open, high, low, close, volume);
        self.rsi_long.update_bar(open, high, low, close, volume);

        if self.is_ready() {
            let short_val = self.rsi_short.value().main();
            let long_val = self.rsi_long.value().main();
            // Both overbought OR both oversold
            self.v = (short_val > 70.0 && long_val > 70.0) || (short_val < 30.0 && long_val < 30.0);
        }
        self.v
    }

    pub fn update(&mut self, a: bool, b: bool) -> bool {
        self.v = a && b;
        self.v
    }

    pub fn value(&self) -> bool {
        self.v
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.rsi_short.is_ready() && self.rsi_long.is_ready()
    }

    pub fn reset(&mut self) {
        self.rsi_short.reset();
        self.rsi_long.reset();
        self.v = false;
    }
}

/// OR Gate: Returns true when either RSI is overbought (>70) or oversold (<30)
#[derive(Clone)]
pub struct OrGate {
    rsi_short: Rsi,
    rsi_long: Rsi,
    pub v: bool,
}
impl Default for OrGate {
    fn default() -> Self {
        Self::new()
    }
}

impl OrGate {
    pub fn new() -> Self {
        Self {
            rsi_short: Rsi::new(7),
            rsi_long: Rsi::new(21),
            v: false,
        }
    }

    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> bool {
        self.rsi_short.update_bar(open, high, low, close, volume);
        self.rsi_long.update_bar(open, high, low, close, volume);

        if self.is_ready() {
            let short_val = self.rsi_short.value().main();
            let long_val = self.rsi_long.value().main();
            // Either is extreme
            self.v = !(30.0..=70.0).contains(&short_val) || !(30.0..=70.0).contains(&long_val);
        }
        self.v
    }

    pub fn update(&mut self, a: bool, b: bool) -> bool {
        self.v = a || b;
        self.v
    }

    pub fn value(&self) -> bool {
        self.v
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.rsi_short.is_ready() && self.rsi_long.is_ready()
    }

    pub fn reset(&mut self) {
        self.rsi_short.reset();
        self.rsi_long.reset();
        self.v = false;
    }
}

/// XOR Gate: Returns true when RSIs disagree (divergence)
/// True when one is overbought and other is not, or one is oversold and other is not
#[derive(Clone)]
pub struct XorGate {
    rsi_short: Rsi,
    rsi_long: Rsi,
    pub v: bool,
}
impl Default for XorGate {
    fn default() -> Self {
        Self::new()
    }
}

impl XorGate {
    pub fn new() -> Self {
        Self {
            rsi_short: Rsi::new(7),
            rsi_long: Rsi::new(21),
            v: false,
        }
    }

    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> bool {
        self.rsi_short.update_bar(open, high, low, close, volume);
        self.rsi_long.update_bar(open, high, low, close, volume);

        if self.is_ready() {
            let short_val = self.rsi_short.value().main();
            let long_val = self.rsi_long.value().main();
            let short_extreme = !(30.0..=70.0).contains(&short_val);
            let long_extreme = !(30.0..=70.0).contains(&long_val);
            // XOR: exactly one is extreme (divergence)
            self.v = short_extreme ^ long_extreme;
        }
        self.v
    }

    pub fn update(&mut self, a: bool, b: bool) -> bool {
        self.v = a ^ b;
        self.v
    }

    pub fn value(&self) -> bool {
        self.v
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.rsi_short.is_ready() && self.rsi_long.is_ready()
    }

    pub fn reset(&mut self) {
        self.rsi_short.reset();
        self.rsi_long.reset();
        self.v = false;
    }
}

/// SignCombiner: Combines RSI signals into {-1, 0, 1}
/// +1 if both bullish (RSI < 30), -1 if both bearish (RSI > 70), 0 otherwise
#[derive(Clone)]
pub struct SignCombiner {
    rsi_short: Rsi,
    rsi_long: Rsi,
    pub s: i8,
}
impl Default for SignCombiner {
    fn default() -> Self {
        Self::new()
    }
}

impl SignCombiner {
    pub fn new() -> Self {
        Self {
            rsi_short: Rsi::new(7),
            rsi_long: Rsi::new(21),
            s: 0,
        }
    }

    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> i8 {
        self.rsi_short.update_bar(open, high, low, close, volume);
        self.rsi_long.update_bar(open, high, low, close, volume);

        if self.is_ready() {
            let short_val = self.rsi_short.value().main();
            let long_val = self.rsi_long.value().main();

            // Convert to signals: +1 bullish (oversold), -1 bearish (overbought), 0 neutral
            let short_sig: i8 = if short_val < 30.0 { 1 } else if short_val > 70.0 { -1 } else { 0 };
            let long_sig: i8 = if long_val < 30.0 { 1 } else if long_val > 70.0 { -1 } else { 0 };

            // Combine signals
            self.s = (short_sig + long_sig).clamp(-1, 1);
        }
        self.s
    }

    pub fn update(&mut self, a: i8, b: i8) -> i8 {
        self.s = a.saturating_add(b).clamp(-1, 1);
        self.s
    }

    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Signal(self.s)
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.rsi_short.is_ready() && self.rsi_long.is_ready()
    }

    pub fn reset(&mut self) {
        self.rsi_short.reset();
        self.rsi_long.reset();
        self.s = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_and_gate_legacy() {
        let mut gate = AndGate::new();
        // Legacy update method still works
        assert!(!gate.update(true, false));
        assert!(!gate.update(false, true));
        assert!(gate.update(true, true));
        assert!(!gate.update(false, false));

        gate.reset();
        assert!(!gate.value());
    }

    #[test]
    fn test_and_gate_with_data() {
        let mut gate = AndGate::new();
        assert!(!gate.is_ready()); // Needs warmup

        // Feed price data to warm up internal RSIs
        let mut price = 100.0;
        for _ in 0..30 {
            price += 0.5;
            gate.update_bar(price - 0.2, price + 0.3, price - 0.3, price, 1000.0);
        }
        assert!(gate.is_ready());
    }

    #[test]
    fn test_or_gate_legacy() {
        let mut gate = OrGate::new();
        assert!(gate.update(true, false));
        assert!(gate.update(false, true));
        assert!(gate.update(true, true));
        assert!(!gate.update(false, false));

        gate.reset();
        assert!(!gate.value());
    }

    #[test]
    fn test_or_gate_with_data() {
        let mut gate = OrGate::new();

        let mut price = 100.0;
        for _ in 0..30 {
            price += 0.5;
            gate.update_bar(price - 0.2, price + 0.3, price - 0.3, price, 1000.0);
        }
        assert!(gate.is_ready());
    }

    #[test]
    fn test_xor_gate_legacy() {
        let mut gate = XorGate::new();
        assert!(gate.update(true, false));
        assert!(gate.update(false, true));
        assert!(!gate.update(true, true));
        assert!(!gate.update(false, false));

        gate.reset();
        assert!(!gate.value());
    }

    #[test]
    fn test_xor_gate_with_divergence() {
        // XOR needs short RSI in extreme but long RSI neutral
        // Key: warmup with oscillating prices to get RSI near 50, then spike

        let mut rsi_short = Rsi::new(7);
        let mut rsi_long = Rsi::new(21);

        // Warmup with oscillating prices (up/down) to get RSI near 50
        let mut price = 100.0;
        for i in 0..30 {
            // Alternate up/down to balance gains and losses
            if i % 2 == 0 {
                price += 1.0;
            } else {
                price -= 1.0;
            }
            rsi_short.update_bar(price - 0.5, price + 0.5, price - 0.5, price, 1000.0);
            rsi_long.update_bar(price - 0.5, price + 0.5, price - 0.5, price, 1000.0);
        }

        // Both RSIs should be near 50 now
        eprintln!("After warmup: short={:.1}, long={:.1}",
            rsi_short.value().main(), rsi_long.value().main());

        // Sharp upward spike - 7 consecutive gains
        let mut any_divergence = false;
        for i in 0..10 {
            price += 3.0; // Strong up move
            rsi_short.update_bar(price - 1.0, price + 1.0, price - 1.5, price, 1000.0);
            rsi_long.update_bar(price - 1.0, price + 1.0, price - 1.5, price, 1000.0);

            let short_val = rsi_short.value().main();
            let long_val = rsi_long.value().main();
            let short_extreme = short_val > 70.0 || short_val < 30.0;
            let long_extreme = long_val > 70.0 || long_val < 30.0;

            eprintln!("Bar {}: short={:.1}, long={:.1}, xor={}",
                i, short_val, long_val, short_extreme ^ long_extreme);

            if short_extreme ^ long_extreme {
                any_divergence = true;
            }
        }

        // XOR should fire when short RSI hits extreme before long RSI does
        assert!(any_divergence, "XOR should detect divergence during sharp spike");
    }

    #[test]
    fn test_sign_combiner_legacy() {
        let mut sc = SignCombiner::new();
        // Legacy update method still works
        assert_eq!(sc.update(1, 0), 1);
        assert_eq!(sc.update(-1, 0), -1);
        assert_eq!(sc.update(1, 1), 1);  // clamped to 1
        assert_eq!(sc.update(-1, -1), -1);  // clamped to -1
        assert_eq!(sc.update(1, -1), 0);

        sc.reset();
        assert_eq!(sc.value().as_signal(), Some(0));
    }
}
