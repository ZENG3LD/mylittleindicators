//! RoleKind taxonomy — semantic classification of indicator roles.
//!
//! Used to validate that an indicator is appropriate for a given
//! `OperatorClass` slot (e.g., only `Smoother` indicators make sense
//! as the left operand of a `Cross` event with MA semantics).

use crate::BarIndicatorId;

/// Semantic role a bar indicator plays in a strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RoleKind {
    /// Moving average / smoother (SMA, EMA, HMA, Kalman, etc.).
    Smoother,
    /// Bounded oscillator [0..100] or [-100..100] (RSI, Stoch, CCI, etc.).
    OscillatorBounded,
    /// Unbounded oscillator (MACD, ROC, CMO, TSI, etc.).
    OscillatorUnbounded,
    /// Price channel (Bollinger Bands, Keltner, Donchian, etc.).
    Channel,
    /// Volatility measure (ATR, NATR, Chop, RV, etc.).
    Volatility,
    /// Trend strength indicator (ADX, VHF, HURST, etc.).
    TrendStrength,
    /// Volume flow indicator (OBV, CMF, MFI, etc.).
    VolumeFlow,
    /// Pivot / swing detector (Zigzag, Pivot levels, etc.).
    PivotIndicator,
    /// Candlestick pattern detector.
    PatternDetector,
    /// Regime / logic filter (MRF, Hyst, LogicAnd, etc.).
    RegimeFilter,
    /// Trend-stop / trailing-stop indicator (SuperTrend, SSL, etc.).
    ///
    /// Outputs a single value positioned above or below price.
    /// The "side" flip (stop crosses price) is the primary event.
    TrendStop,
    /// Normalized scalar measurement: probability, density, tanh-strength, EMA magnitude.
    ///
    /// Output is a `Single(f64)` in [0,1] or [-1,1]. Not events, not price levels.
    /// Examples: FVG reversion probability, liquidity gap density, swing strength score.
    StatisticalScoring,
    /// Unknown / uncategorised — treated as general-purpose.
    Other,
}

/// Map a `BarIndicatorId` to its canonical `RoleKind`.
///
/// The mapping is a best-effort classification. Indicators that do not
/// clearly fit a category are mapped to `RoleKind::Other`.
pub fn role_kind_for(id: BarIndicatorId) -> RoleKind {
    match id {
        // ── Smoothers ───────────────────────────────────────────────────────
        BarIndicatorId::Sma
        | BarIndicatorId::Ema
        | BarIndicatorId::Wma
        | BarIndicatorId::Hma
        | BarIndicatorId::Rma
        | BarIndicatorId::Dema
        | BarIndicatorId::Tema
        | BarIndicatorId::Tma
        | BarIndicatorId::Trima
        | BarIndicatorId::T3
        | BarIndicatorId::Alma
        | BarIndicatorId::Ama
        | BarIndicatorId::Jma
        | BarIndicatorId::Mcginley
        | BarIndicatorId::Lr
        | BarIndicatorId::Vwap
        | BarIndicatorId::Vwma
        | BarIndicatorId::AvFrama
        | BarIndicatorId::AvVidya
        | BarIndicatorId::Ehlersfa
        | BarIndicatorId::Ehlersz
        | BarIndicatorId::Framaadv
        | BarIndicatorId::Adaptivema
        | BarIndicatorId::Frama
        | BarIndicatorId::Kama
        | BarIndicatorId::Mama
        | BarIndicatorId::Vidya
        | BarIndicatorId::Abgfilter
        | BarIndicatorId::Ekf
        | BarIndicatorId::Kalman
        | BarIndicatorId::Kslope
        | BarIndicatorId::Kslopez
        | BarIndicatorId::Particle
        | BarIndicatorId::Rts
        | BarIndicatorId::Zlsma => RoleKind::Smoother,

        // ── Trend Stops ───────────────────────────────────────────────────────
        BarIndicatorId::Supertrend
        | BarIndicatorId::Ssl
        | BarIndicatorId::GannHilo => RoleKind::TrendStop,

        // ── Bounded Oscillators ──────────────────────────────────────────────
        BarIndicatorId::Rsi
        | BarIndicatorId::StochRsi
        | BarIndicatorId::Stoch
        | BarIndicatorId::Stochkd
        | BarIndicatorId::AdaptiveStoch
        | BarIndicatorId::Cci
        | BarIndicatorId::WilliamsR
        | BarIndicatorId::Mfi
        | BarIndicatorId::IftRsi
        | BarIndicatorId::Demarker
        | BarIndicatorId::ConnorsRsi
        | BarIndicatorId::Imi
        | BarIndicatorId::Kdj
        | BarIndicatorId::Psl
        | BarIndicatorId::Qqe
        | BarIndicatorId::Uo
        | BarIndicatorId::UoSmooth
        | BarIndicatorId::Rsx
        | BarIndicatorId::Rsioma
        | BarIndicatorId::AtrRsi
        | BarIndicatorId::Vwrsi
        | BarIndicatorId::Smi
        | BarIndicatorId::Pzo => RoleKind::OscillatorBounded,

        // ── Unbounded Oscillators ────────────────────────────────────────────
        BarIndicatorId::Macd
        | BarIndicatorId::MacdHist
        | BarIndicatorId::MacdHistZ
        | BarIndicatorId::MacdSignal
        | BarIndicatorId::Roc
        | BarIndicatorId::RocPct
        | BarIndicatorId::Cmo
        | BarIndicatorId::Tsi
        | BarIndicatorId::Ewmac
        | BarIndicatorId::EwmacRobust
        | BarIndicatorId::Ppo
        | BarIndicatorId::PpoSignal
        | BarIndicatorId::Apo
        | BarIndicatorId::Coppock
        | BarIndicatorId::Bop
        | BarIndicatorId::Kst
        | BarIndicatorId::Kvo
        | BarIndicatorId::Trix
        | BarIndicatorId::Stc
        | BarIndicatorId::Pfe
        | BarIndicatorId::Pmo
        | BarIndicatorId::Bias
        | BarIndicatorId::Dpo
        | BarIndicatorId::DpoPct
        | BarIndicatorId::Rvgi
        | BarIndicatorId::MoFisher
        | BarIndicatorId::Dss
        | BarIndicatorId::Gator
        | BarIndicatorId::Cog
        | BarIndicatorId::Didi => RoleKind::OscillatorUnbounded,

        // ── Channels ────────────────────────────────────────────────────────
        BarIndicatorId::Bb
        | BarIndicatorId::Kc
        | BarIndicatorId::Dc
        | BarIndicatorId::Atrchan
        | BarIndicatorId::Envelope
        | BarIndicatorId::Regchan
        | BarIndicatorId::Starc
        | BarIndicatorId::Adaptivebb
        | BarIndicatorId::Vwapchan
        | BarIndicatorId::Projbands
        | BarIndicatorId::Theilsenchan
        | BarIndicatorId::Stddevchan
        | BarIndicatorId::Medchan
        | BarIndicatorId::Adaptivechan
        | BarIndicatorId::Darvas
        | BarIndicatorId::Dpobands
        | BarIndicatorId::Trimabands
        | BarIndicatorId::Qrchan
        | BarIndicatorId::Pivotchan
        | BarIndicatorId::Pricechan
        | BarIndicatorId::Percentilech
        | BarIndicatorId::Ichimoku
        | BarIndicatorId::Ichimokupos
        | BarIndicatorId::Ichimokuthick => RoleKind::Channel,

        // ── Volatility ───────────────────────────────────────────────────────
        BarIndicatorId::Atr
        | BarIndicatorId::Natr
        | BarIndicatorId::Atrp
        | BarIndicatorId::Atrbw
        | BarIndicatorId::Atrc
        | BarIndicatorId::Atrpt
        | BarIndicatorId::Atrz
        | BarIndicatorId::Tr
        | BarIndicatorId::Rvi
        | BarIndicatorId::Rv
        | BarIndicatorId::Rvz
        | BarIndicatorId::Chop
        | BarIndicatorId::Ui
        | BarIndicatorId::Vov
        | BarIndicatorId::Vovp
        | BarIndicatorId::Vovpt
        | BarIndicatorId::Avr
        | BarIndicatorId::Bpv
        | BarIndicatorId::C2cvp
        | BarIndicatorId::Cv
        | BarIndicatorId::Dvr
        | BarIndicatorId::Har
        | BarIndicatorId::Hvc2c
        | BarIndicatorId::Kp
        | BarIndicatorId::Nr
        | BarIndicatorId::Pgry
        | BarIndicatorId::Rbvj
        | BarIndicatorId::Rcb
        | BarIndicatorId::Rp
        | BarIndicatorId::Rq
        | BarIndicatorId::Sqmom
        | BarIndicatorId::Vbd
        | BarIndicatorId::Vbexp
        | BarIndicatorId::VoDc
        | BarIndicatorId::VoKc
        | BarIndicatorId::VoMi
        | BarIndicatorId::VoVr
        | BarIndicatorId::Wvf
        | BarIndicatorId::Fuzzy
        | BarIndicatorId::Abb => RoleKind::Volatility,

        // ── Trend Strength ────────────────────────────────────────────────────
        BarIndicatorId::Adx
        | BarIndicatorId::AdxSlope
        | BarIndicatorId::Vhf
        | BarIndicatorId::VhfMa
        | BarIndicatorId::TrEr
        | BarIndicatorId::Tii
        | BarIndicatorId::Ravi
        | BarIndicatorId::Sdl
        | BarIndicatorId::Apen
        | BarIndicatorId::Sampen
        | BarIndicatorId::Gapo
        | BarIndicatorId::Rwi
        | BarIndicatorId::EmaSlope
        | BarIndicatorId::KamaSlope
        | BarIndicatorId::LrSlope => RoleKind::TrendStrength,

        // ── Volume Flow ───────────────────────────────────────────────────────
        BarIndicatorId::Obv
        | BarIndicatorId::Cmf
        | BarIndicatorId::Vfi
        | BarIndicatorId::Pvt
        | BarIndicatorId::Vpt
        | BarIndicatorId::Ad
        | BarIndicatorId::Cho
        | BarIndicatorId::Eom
        | BarIndicatorId::Vzo
        | BarIndicatorId::NviPvi
        | BarIndicatorId::Poc
        | BarIndicatorId::Pvo
        | BarIndicatorId::Rvol
        | BarIndicatorId::Vdelta
        | BarIndicatorId::Vo
        | BarIndicatorId::Vpin
        | BarIndicatorId::Vroc
        | BarIndicatorId::Vz
        | BarIndicatorId::Vprofile
        | BarIndicatorId::Di
        | BarIndicatorId::Tmf
        | BarIndicatorId::Wad
        | BarIndicatorId::Asi
        | BarIndicatorId::Fi
        | BarIndicatorId::Ii
        | BarIndicatorId::Iip
        | BarIndicatorId::Iir
        | BarIndicatorId::MoObv => RoleKind::VolumeFlow,

        // ── Pivot Indicators ──────────────────────────────────────────────────
        BarIndicatorId::Zigzag
        | BarIndicatorId::Pivot
        | BarIndicatorId::Floorpivot
        | BarIndicatorId::Camarilla
        | BarIndicatorId::Demark
        | BarIndicatorId::Woodie
        | BarIndicatorId::Rmid
        | BarIndicatorId::Rquart
        | BarIndicatorId::Pivavwap
        | BarIndicatorId::Avwap
        | BarIndicatorId::Avwaprev
        | BarIndicatorId::Avwaptouch
        | BarIndicatorId::Hlva => RoleKind::PivotIndicator,

        // ── Statistical Scoring ───────────────────────────────────────────────
        BarIndicatorId::Fvgrev
        | BarIndicatorId::Fvgdur
        | BarIndicatorId::Fvgalt
        | BarIndicatorId::Liqgap
        | BarIndicatorId::Swingstr
        | BarIndicatorId::SwingAge
        | BarIndicatorId::Kcomp
        | BarIndicatorId::Kscr => RoleKind::StatisticalScoring,

        // ── Pattern Detectors ─────────────────────────────────────────────────
        BarIndicatorId::Bos
        | BarIndicatorId::Fvg
        | BarIndicatorId::Candleanatomy
        | BarIndicatorId::Heikinashi
        | BarIndicatorId::Wickspike => RoleKind::PatternDetector,

        // ── Regime Filters ────────────────────────────────────────────────────
        BarIndicatorId::Mrf
        | BarIndicatorId::Hyst
        | BarIndicatorId::Logicand
        | BarIndicatorId::Logicor
        | BarIndicatorId::Logicxor
        | BarIndicatorId::Wcomp
        | BarIndicatorId::Logicsign
        | BarIndicatorId::HaTrend
        | BarIndicatorId::Kregime => RoleKind::RegimeFilter,

        // ── Everything else ───────────────────────────────────────────────────
        _ => RoleKind::Other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sma_is_smoother() {
        assert_eq!(role_kind_for(BarIndicatorId::Sma), RoleKind::Smoother);
    }

    #[test]
    fn rsi_is_oscillator_bounded() {
        assert_eq!(role_kind_for(BarIndicatorId::Rsi), RoleKind::OscillatorBounded);
    }

    #[test]
    fn bb_is_channel() {
        assert_eq!(role_kind_for(BarIndicatorId::Bb), RoleKind::Channel);
    }

    #[test]
    fn atr_is_volatility() {
        assert_eq!(role_kind_for(BarIndicatorId::Atr), RoleKind::Volatility);
    }

    #[test]
    fn adx_is_trend_strength() {
        assert_eq!(role_kind_for(BarIndicatorId::Adx), RoleKind::TrendStrength);
    }

    #[test]
    fn obv_is_volume_flow() {
        assert_eq!(role_kind_for(BarIndicatorId::Obv), RoleKind::VolumeFlow);
    }

    #[test]
    fn wickspike_is_pattern_detector() {
        assert_eq!(role_kind_for(BarIndicatorId::Wickspike), RoleKind::PatternDetector);
    }

    #[test]
    fn mrf_is_regime_filter() {
        assert_eq!(role_kind_for(BarIndicatorId::Mrf), RoleKind::RegimeFilter);
    }

    #[test]
    fn supertrend_is_trend_stop() {
        assert_eq!(role_kind_for(BarIndicatorId::Supertrend), RoleKind::TrendStop);
    }

    #[test]
    fn ssl_is_trend_stop() {
        assert_eq!(role_kind_for(BarIndicatorId::Ssl), RoleKind::TrendStop);
    }

    #[test]
    fn gann_hilo_is_trend_stop() {
        assert_eq!(role_kind_for(BarIndicatorId::GannHilo), RoleKind::TrendStop);
    }

    #[test]
    fn didi_is_oscillator_unbounded() {
        assert_eq!(role_kind_for(BarIndicatorId::Didi), RoleKind::OscillatorUnbounded);
    }

    #[test]
    fn hatrend_is_regime_filter() {
        assert_eq!(role_kind_for(BarIndicatorId::HaTrend), RoleKind::RegimeFilter);
    }

    #[test]
    fn kcomp_is_statistical_scoring() {
        assert_eq!(role_kind_for(BarIndicatorId::Kcomp), RoleKind::StatisticalScoring);
    }

    #[test]
    fn kregime_is_regime_filter() {
        assert_eq!(role_kind_for(BarIndicatorId::Kregime), RoleKind::RegimeFilter);
    }

    #[test]
    fn kscr_is_statistical_scoring() {
        assert_eq!(role_kind_for(BarIndicatorId::Kscr), RoleKind::StatisticalScoring);
    }
}
