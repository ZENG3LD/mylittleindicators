//! Config-granularity counter for bar indicators.
//!
//! Reports, per indicator, how many independently-iterable configuration axes it
//! has and how deep nesting goes — the "configurability granularity" a
//! strategy-assembler / auto-picker uses. NOT computational cost.

use crate::bar_indicators::bar_indicator_id::BarIndicatorId;
use crate::bar_indicators::instance_factory::IndicatorConfig;
use crate::bar_indicators::ohlcv_field::OhlcvField;
use crate::bar_indicators::average::moving_average::MovingAverageType;

/// Granularity summary for a single `IndicatorConfig`.
///
/// Each field counts independently-iterable configuration axes at this level.
/// Recursive fields (`total_axes`, `type_choosable_axes`) aggregate over all
/// nested `inner_indicators` as well.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GranularitySpec {
    /// Number of period slots (`periods.len()`)
    pub period_axes: usize,
    /// Number of named MA-type axes (`ma_types.len()`)
    pub ma_type_axes: usize,
    /// Number of additional scalar params (`additional_params.len()`)
    pub scalar_axes: usize,
    /// Number of boolean flag axes (`flags.len()`)
    pub flag_axes: usize,
    /// Number of named enum axes (`enum_params.len()`)
    pub enum_axes: usize,
    /// Whether a configurable OHLCV source is in use (`source != Close`)
    pub source_axis: bool,
    /// Number of direct `inner_indicators`
    pub inner_count: usize,
    /// Nesting depth: 0 = leaf (no inners), 1 = has inners, 2 = inners have inners, …
    pub max_depth: usize,

    // ---- recursive aggregates (include self + all descendants) ----
    /// Sum of all axes across self and all descendants
    pub total_axes_recursive: usize,
    /// Sum of ma_type_axes across self and all descendants
    pub type_choosable_recursive: usize,
}

impl GranularitySpec {
    /// Total independently-iterable axes at this node only (no recursion).
    #[inline]
    pub fn local_axes(&self) -> usize {
        self.period_axes
            + self.ma_type_axes
            + self.scalar_axes
            + self.flag_axes
            + self.enum_axes
            + if self.source_axis { 1 } else { 0 }
    }

    /// Total independently-iterable axes summed recursively over self + all
    /// `inner_indicators` descendants.
    #[inline]
    pub fn total_axes(&self) -> usize {
        self.total_axes_recursive
    }

    /// MA-type axes summed recursively over self + all descendants.
    #[inline]
    pub fn type_choosable_axes(&self) -> usize {
        self.type_choosable_recursive
    }
}

/// Compute the `GranularitySpec` for a filled `IndicatorConfig`.
///
/// Walks the same fields that `IndicatorConfig::compute_param_hash` walks:
/// `periods.len()`, `ma_types.len()`, `additional_params.len()`,
/// `flags.len()`, `enum_params.len()`, `source != Close`, and recurses
/// into `inner_indicators`.
pub fn config_granularity(cfg: &IndicatorConfig) -> GranularitySpec {
    let period_axes = cfg.periods.len();
    let ma_type_axes = cfg.ma_types.len();
    let scalar_axes = cfg.additional_params.len();
    let flag_axes = cfg.flags.len();
    let enum_axes = cfg.enum_params.len();
    let source_axis = cfg.source != OhlcvField::Close;
    let inner_count = cfg.inner_indicators.len();

    let local = period_axes
        + ma_type_axes
        + scalar_axes
        + flag_axes
        + enum_axes
        + if source_axis { 1 } else { 0 };

    let mut child_total = 0usize;
    let mut child_type_choosable = 0usize;
    let mut max_child_depth = 0usize;

    for inner in &cfg.inner_indicators {
        let child_spec = config_granularity(inner);
        child_total += child_spec.total_axes_recursive;
        child_type_choosable += child_spec.type_choosable_recursive;
        if child_spec.max_depth + 1 > max_child_depth + 1 {
            max_child_depth = child_spec.max_depth;
        }
    }

    let max_depth = if inner_count == 0 {
        0
    } else {
        1 + max_child_depth
    };

    GranularitySpec {
        period_axes,
        ma_type_axes,
        scalar_axes,
        flag_axes,
        enum_axes,
        source_axis,
        inner_count,
        max_depth,
        total_axes_recursive: local + child_total,
        type_choosable_recursive: ma_type_axes + child_type_choosable,
    }
}

// ---------------------------------------------------------------------------
// Helpers for canonical_max_config
// ---------------------------------------------------------------------------

/// Common wrap-oscillator extra axes added when an arm calls `wrap_oscillator`.
/// Applies to every arm marked `inner` / `wrap_oscillator` in the audit.
fn add_wrap_axes(cfg: IndicatorConfig) -> IndicatorConfig {
    cfg
        .with_flag("with_divergence", true)
        .with_flag("with_volume_event", true)
        .with_flag("vw_with_strength", true)
        .with_param("vw_baseline_period", 14.0)
        .with_param("vw_spike_threshold", 2.0)
}

/// Produce a config with EVERY reachable slot filled for `id`, so that
/// `config_granularity(&canonical_max_config(id))` reports the indicator's
/// maximum reachable granularity.
///
/// Source of truth: `AUDIT_factory_slots.md`.
pub fn canonical_max_config(id: BarIndicatorId) -> IndicatorConfig {
    let name = format!("{id:?}");
    match id {
        // ---- Averages ----
        BarIndicatorId::Sma => IndicatorConfig::new(id, name, vec![14])
            .with_source(OhlcvField::High),
        BarIndicatorId::Ema => IndicatorConfig::new(id, name, vec![14])
            .with_source(OhlcvField::High),
        BarIndicatorId::Rma => IndicatorConfig::new(id, name, vec![14])
            .with_source(OhlcvField::High),
        BarIndicatorId::Hma => IndicatorConfig::new(id, name, vec![14])
            .with_source(OhlcvField::High),
        BarIndicatorId::Wma => IndicatorConfig::new(id, name, vec![14])
            .with_source(OhlcvField::High),
        BarIndicatorId::Dema => IndicatorConfig::new(id, name, vec![14])
            .with_source(OhlcvField::High),
        BarIndicatorId::Vwma => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::Vwap => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::Trima => IndicatorConfig::new(id, name, vec![14])
            .with_source(OhlcvField::High),
        BarIndicatorId::Alma => IndicatorConfig::new(id, name, vec![14])
            .with_param("offset", 0.85)
            .with_param("sigma", 6.0)
            .with_source(OhlcvField::High),
        BarIndicatorId::T3 => IndicatorConfig::new(id, name, vec![14])
            .with_param("vfactor", 0.7)
            .with_source(OhlcvField::High),
        BarIndicatorId::Mcginley => IndicatorConfig::new(id, name, vec![14])
            .with_source(OhlcvField::High),
        BarIndicatorId::Ama => IndicatorConfig::new(id, name, vec![14, 2, 30])
            .with_source(OhlcvField::High),
        BarIndicatorId::Tma => IndicatorConfig::new(id, name, vec![14])
            .with_source(OhlcvField::High),
        BarIndicatorId::Tema => IndicatorConfig::new(id, name, vec![14])
            .with_source(OhlcvField::High),
        BarIndicatorId::Frama => IndicatorConfig::new(id, name, vec![14])
            .with_source(OhlcvField::High),
        BarIndicatorId::AvFrama => IndicatorConfig::new(id, name, vec![14])
            .with_source(OhlcvField::High),
        BarIndicatorId::Lr => IndicatorConfig::new(id, name, vec![14])
            .with_source(OhlcvField::High),
        BarIndicatorId::Jma => IndicatorConfig::new(id, name, vec![14])
            .with_param("phase", 0.0)
            .with_source(OhlcvField::High),
        BarIndicatorId::AvVidya => IndicatorConfig::new(id, name, vec![14])
            .with_named_ma_type("ma_type", MovingAverageType::EMA)
            .with_source(OhlcvField::High),
        BarIndicatorId::Ehlersfa => IndicatorConfig::new(id, name, vec![14])
            .with_source(OhlcvField::High),
        BarIndicatorId::Ehlersz => IndicatorConfig::new(id, name, vec![14])
            .with_source(OhlcvField::High),
        BarIndicatorId::Framaadv => IndicatorConfig::new(id, name, vec![14])
            .with_param("fractal_method", 0.0),

        // ---- Momentum ----
        BarIndicatorId::Rsi => {
            add_wrap_axes(
                IndicatorConfig::new(id, name, vec![14])
                    .with_named_ma_type("ma_type", MovingAverageType::EMA)
                    .with_source(OhlcvField::High),
            )
        }
        BarIndicatorId::Macd => {
            add_wrap_axes(
                IndicatorConfig::new(id, name, vec![12, 26, 9])
                    .with_named_ma_type("fast_ma_type", MovingAverageType::EMA)
                    .with_named_ma_type("slow_ma_type", MovingAverageType::EMA)
                    .with_named_ma_type("signal_ma_type", MovingAverageType::EMA)
                    .with_source(OhlcvField::High),
            )
        }
        BarIndicatorId::Ppo => {
            add_wrap_axes(
                IndicatorConfig::new(id, name, vec![12, 26, 9])
                    .with_named_ma_type("fast_ma_type", MovingAverageType::EMA)
                    .with_named_ma_type("slow_ma_type", MovingAverageType::EMA)
                    .with_named_ma_type("signal_ma_type", MovingAverageType::EMA)
                    .with_source(OhlcvField::High),
            )
        }
        BarIndicatorId::BbPeriod => IndicatorConfig::new(id, name, vec![20])
            .with_named_ma_type("ma_type", MovingAverageType::EMA)
            .with_param("std_dev", 2.0),
        BarIndicatorId::Atr => IndicatorConfig::new(id, name, vec![14])
            .with_named_ma_type("ma_type", MovingAverageType::EMA),
        BarIndicatorId::Tr => IndicatorConfig::new(id, name, vec![]),
        BarIndicatorId::Bpv => IndicatorConfig::new(id, name, vec![14, 5]),
        BarIndicatorId::Sqmom => IndicatorConfig::new(id, name, vec![20, 14, 20])
            .with_named_ma_type("bb_ma_type", MovingAverageType::EMA)
            .with_named_ma_type("kc_ma_type", MovingAverageType::EMA)
            .with_source(OhlcvField::High),
        BarIndicatorId::Natr => IndicatorConfig::new(id, name, vec![14])
            .with_named_ma_type("ma_type", MovingAverageType::EMA),
        BarIndicatorId::Chop => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::Ui => IndicatorConfig::new(id, name, vec![14])
            .with_source(OhlcvField::High),
        BarIndicatorId::VoMi => IndicatorConfig::new(id, name, vec![14, 5]),
        BarIndicatorId::Adx => IndicatorConfig::new(id, name, vec![14])
            .with_named_ma_type("atr_ma_type", MovingAverageType::EMA),
        BarIndicatorId::DiPlusMinus => IndicatorConfig::new(id, name, vec![14])
            .with_named_ma_type("atr_ma_type", MovingAverageType::EMA),
        BarIndicatorId::Dm => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::VhfMa => IndicatorConfig::new(id, name, vec![14])
            .with_named_ma_type("ma_type", MovingAverageType::EMA)
            .with_source(OhlcvField::High),
        BarIndicatorId::Stochkd => {
            add_wrap_axes(IndicatorConfig::new(id, name, vec![14, 3]))
        }
        BarIndicatorId::WilliamsR => {
            add_wrap_axes(IndicatorConfig::new(id, name, vec![14]))
        }
        BarIndicatorId::Demarker => {
            add_wrap_axes(IndicatorConfig::new(id, name, vec![14]))
        }
        BarIndicatorId::Psar => IndicatorConfig::new(id, name, vec![])
            .with_param("af_start", 0.02)
            .with_param("af_increment", 0.02)
            .with_param("af_max", 0.2),
        BarIndicatorId::Uo => {
            add_wrap_axes(IndicatorConfig::new(id, name, vec![7, 14, 28]))
        }
        BarIndicatorId::UoSmooth => {
            add_wrap_axes(IndicatorConfig::new(id, name, vec![7, 14, 28, 3]))
        }
        BarIndicatorId::Rwi => IndicatorConfig::new(id, name, vec![14])
            .with_named_ma_type("atr_ma_type", MovingAverageType::EMA),
        BarIndicatorId::Bop => {
            add_wrap_axes(IndicatorConfig::new(id, name, vec![]))
        }
        BarIndicatorId::Cfo => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::Rmi => IndicatorConfig::new(id, name, vec![14, 3]),
        BarIndicatorId::Qstick => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::Coppock => {
            add_wrap_axes(
                IndicatorConfig::new(id, name, vec![10, 14, 11])
                    .with_source(OhlcvField::High),
            )
        }
        BarIndicatorId::Apo => {
            add_wrap_axes(
                IndicatorConfig::new(id, name, vec![12, 26])
                    .with_named_ma_type("ma_type", MovingAverageType::EMA),
            )
        }
        BarIndicatorId::Pmo => {
            add_wrap_axes(IndicatorConfig::new(id, name, vec![35, 20, 10]))
        }
        BarIndicatorId::Tsi => {
            add_wrap_axes(
                IndicatorConfig::new(id, name, vec![25, 13, 7])
                    .with_named_ma_type("smoothing_ma_type", MovingAverageType::EMA)
                    .with_named_ma_type("signal_ma_type", MovingAverageType::EMA)
                    .with_source(OhlcvField::High),
            )
        }
        BarIndicatorId::Dpo => {
            add_wrap_axes(
                IndicatorConfig::new(id, name, vec![14])
                    .with_source(OhlcvField::High),
            )
        }
        BarIndicatorId::Kst => {
            add_wrap_axes(
                IndicatorConfig::new(id, name, vec![10, 13, 14, 15, 10, 13, 14, 15, 9])
                    .with_named_ma_type("roc_ma_type", MovingAverageType::EMA)
                    .with_named_ma_type("signal_ma_type", MovingAverageType::EMA)
                    .with_source(OhlcvField::High),
            )
        }
        BarIndicatorId::Rvgi => {
            add_wrap_axes(IndicatorConfig::new(id, name, vec![10, 4]))
        }
        BarIndicatorId::Smi => {
            add_wrap_axes(
                IndicatorConfig::new(id, name, vec![13, 5])
                    .with_named_ma_type("signal_ma_type", MovingAverageType::EMA),
            )
        }
        BarIndicatorId::Stc => {
            add_wrap_axes(
                IndicatorConfig::new(id, name, vec![23, 50, 10, 3])
                    .with_named_ma_type("ma_type", MovingAverageType::EMA)
                    .with_param("signal_period", 3.0),
            )
        }
        BarIndicatorId::ElderImpulse => IndicatorConfig::new(id, name, vec![13])
            .with_named_ma_type("ma_type", MovingAverageType::EMA)
            .with_source(OhlcvField::High),
        BarIndicatorId::ElderRay => IndicatorConfig::new(id, name, vec![13]),
        BarIndicatorId::Vortex => IndicatorConfig::new(id, name, vec![14])
            .with_named_ma_type("atr_ma_type", MovingAverageType::EMA),
        BarIndicatorId::StochRsi => {
            add_wrap_axes(
                IndicatorConfig::new(id, name, vec![14, 14, 3, 3])
                    .with_named_ma_type("k_ma_type", MovingAverageType::EMA)
                    .with_named_ma_type("d_ma_type", MovingAverageType::EMA)
                    .with_source(OhlcvField::High),
            )
        }
        BarIndicatorId::MoFisher => IndicatorConfig::new(id, name, vec![10, 3]),
        BarIndicatorId::Rsx => {
            add_wrap_axes(IndicatorConfig::new(id, name, vec![14]))
        }
        BarIndicatorId::Qqe => {
            add_wrap_axes(
                IndicatorConfig::new(id, name, vec![14, 5])
                    .with_named_ma_type("ma_type", MovingAverageType::EMA)
                    .with_named_ma_type("atr_ma_type", MovingAverageType::EMA)
                    .with_param("threshold_mult", 4.236),
            )
        }
        BarIndicatorId::Kdj => {
            add_wrap_axes(
                IndicatorConfig::new(id, name, vec![9, 3])
                    .with_named_ma_type("d_ma_type", MovingAverageType::EMA),
            )
        }
        BarIndicatorId::ConnorsRsi => {
            add_wrap_axes(
                IndicatorConfig::new(id, name, vec![3, 2, 100])
                    .with_source(OhlcvField::High),
            )
        }
        BarIndicatorId::Trix => {
            add_wrap_axes(
                IndicatorConfig::new(id, name, vec![14, 9])
                    .with_named_ma_type("smoothing_ma_type", MovingAverageType::EMA)
                    .with_named_ma_type("signal_ma_type", MovingAverageType::EMA)
                    .with_source(OhlcvField::High),
            )
        }
        BarIndicatorId::IftRsi => {
            add_wrap_axes(IndicatorConfig::new(id, name, vec![14]))
        }
        BarIndicatorId::DpoPct => {
            add_wrap_axes(IndicatorConfig::new(id, name, vec![14]))
        }
        BarIndicatorId::Dsp => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::RsiPctRank => IndicatorConfig::new(id, name, vec![14, 100]),
        BarIndicatorId::RsiPctBands => IndicatorConfig::new(id, name, vec![14, 100]),
        BarIndicatorId::Dss => IndicatorConfig::new(id, name, vec![10, 3])
            .with_named_ma_type("ma_type", MovingAverageType::EMA),
        BarIndicatorId::Imi => {
            add_wrap_axes(IndicatorConfig::new(id, name, vec![14]))
        }
        BarIndicatorId::VoDc => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::VoKc => IndicatorConfig::new(id, name, vec![14])
            .with_named_ma_type("atr_ma_type", MovingAverageType::EMA)
            .with_param("k", 1.5),
        BarIndicatorId::Rvi => IndicatorConfig::new(id, name, vec![14])
            .with_source(OhlcvField::High),
        BarIndicatorId::VoVr => IndicatorConfig::new(id, name, vec![14, 5])
            .with_named_ma_type("atr_ma_type", MovingAverageType::EMA),
        BarIndicatorId::Har => IndicatorConfig::new(id, name, vec![1, 5, 22])
            .with_param("annualize_factor", 252.0),
        BarIndicatorId::Atrpt => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::Rq => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::Hvc2c => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::Atrchan => IndicatorConfig::new(id, name, vec![14, 20])
            .with_named_ma_type("center_ma", MovingAverageType::EMA)
            .with_named_ma_type("atr_ma", MovingAverageType::EMA)
            .with_param("k", 2.0),
        BarIndicatorId::Kp => IndicatorConfig::new(id, name, vec![14])
            .with_param("k", 1.5),
        BarIndicatorId::Fuzzy => IndicatorConfig::new(id, name, vec![14])
            .with_param("t1", 0.2)
            .with_param("t2", 0.4)
            .with_param("t3", 0.6)
            .with_param("t4", 0.8),
        BarIndicatorId::Ad => IndicatorConfig::new(id, name, vec![]),
        BarIndicatorId::Mfi => {
            add_wrap_axes(IndicatorConfig::new(id, name, vec![14]))
        }
        BarIndicatorId::Wad => IndicatorConfig::new(id, name, vec![]),
        BarIndicatorId::Vdelta => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::Vprofile => IndicatorConfig::new(id, name, vec![])
            .with_param("tick_size", 0.01)
            .with_param("session_duration", 1440.0),
        BarIndicatorId::SessionVwap => IndicatorConfig::new(id, name, vec![]),
        BarIndicatorId::Cvd => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::Rvp => IndicatorConfig::new(id, name, vec![14])
            .with_param("bucket_size", 0.01)
            .with_param("value_area_pct", 0.7),
        BarIndicatorId::Vpt => IndicatorConfig::new(id, name, vec![14])
            .with_source(OhlcvField::High),
        BarIndicatorId::Vroc => IndicatorConfig::new(id, name, vec![14, 5]),
        BarIndicatorId::NviPvi => IndicatorConfig::new(id, name, vec![14, 5])
            .with_source(OhlcvField::High),
        BarIndicatorId::Vpin => IndicatorConfig::new(id, name, vec![])
            .with_param("bucket_size", 0.01)
            .with_param("smoothing_window", 50.0),
        BarIndicatorId::Cmf => {
            add_wrap_axes(IndicatorConfig::new(id, name, vec![20]))
        }
        BarIndicatorId::Eom => IndicatorConfig::new(id, name, vec![14])
            .with_param("scale_factor", 1000000.0),
        BarIndicatorId::Fi => IndicatorConfig::new(id, name, vec![13])
            .with_named_ma_type("ma_type", MovingAverageType::EMA)
            .with_source(OhlcvField::High),
        BarIndicatorId::Cho => {
            add_wrap_axes(
                IndicatorConfig::new(id, name, vec![3, 10])
                    .with_named_ma_type("ma_type", MovingAverageType::EMA),
            )
        }
        BarIndicatorId::Ii => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::Asi => IndicatorConfig::new(id, name, vec![]),
        BarIndicatorId::Di => IndicatorConfig::new(id, name, vec![]),
        BarIndicatorId::Kvo => IndicatorConfig::new(id, name, vec![10, 16, 9]),

        // ---- Channels ----
        BarIndicatorId::Adaptivebb => IndicatorConfig::new(id, name, vec![20])
            .with_param("multiplier", 2.0)
            .with_param("min_period", 10.0)
            .with_param("max_period", 50.0)
            .with_param("min_multiplier", 1.0)
            .with_param("max_multiplier", 3.0)
            .with_flag("auto_mode", true)
            .with_source(OhlcvField::High),
        BarIndicatorId::Bb => IndicatorConfig::new(id, name, vec![20])
            .with_named_ma_type("ma_type", MovingAverageType::EMA)
            .with_param("std_dev", 2.0)
            .with_source(OhlcvField::High),
        BarIndicatorId::Dc => IndicatorConfig::new(id, name, vec![20]),
        BarIndicatorId::Kc => IndicatorConfig::new(id, name, vec![20])
            .with_named_ma_type("ma_type", MovingAverageType::EMA)
            .with_named_ma_type("atr_ma_type", MovingAverageType::EMA)
            .with_param("multiplier", 2.0)
            .with_source(OhlcvField::High),
        BarIndicatorId::Dcwidth => IndicatorConfig::new(id, name, vec![20]),
        BarIndicatorId::Dcpos => IndicatorConfig::new(id, name, vec![20]),
        BarIndicatorId::Keltbw => IndicatorConfig::new(id, name, vec![20])
            .with_named_ma_type("center_ma_type", MovingAverageType::EMA)
            .with_named_ma_type("atr_ma_type", MovingAverageType::EMA)
            .with_param("multiplier", 2.0),
        BarIndicatorId::Keltpos => IndicatorConfig::new(id, name, vec![20])
            .with_named_ma_type("center_ma_type", MovingAverageType::EMA)
            .with_named_ma_type("atr_ma_type", MovingAverageType::EMA)
            .with_param("multiplier", 2.0),
        BarIndicatorId::Keltdist => IndicatorConfig::new(id, name, vec![20])
            .with_named_ma_type("center_ma_type", MovingAverageType::EMA)
            .with_named_ma_type("atr_ma_type", MovingAverageType::EMA)
            .with_param("multiplier", 2.0),
        BarIndicatorId::Pricechan => IndicatorConfig::new(id, name, vec![20]),
        BarIndicatorId::Pchwidth => IndicatorConfig::new(id, name, vec![20])
            .with_named_ma_type("ma_type", MovingAverageType::EMA),
        BarIndicatorId::Pchosc => IndicatorConfig::new(id, name, vec![20])
            .with_named_ma_type("ma_type", MovingAverageType::EMA),
        BarIndicatorId::Vwapchan => IndicatorConfig::new(id, name, vec![20])
            .with_param("std_dev", 2.0),
        BarIndicatorId::Vwapchanwidth => IndicatorConfig::new(id, name, vec![20])
            .with_param("std_dev", 2.0),
        BarIndicatorId::Regchan => IndicatorConfig::new(id, name, vec![20])
            .with_param("std_dev", 2.0)
            .with_enum("regression_mode", "Standard")
            .with_source(OhlcvField::High),
        BarIndicatorId::Regchanwidth => IndicatorConfig::new(id, name, vec![20])
            .with_param("std_dev", 2.0),
        BarIndicatorId::Envelope => IndicatorConfig::new(id, name, vec![20])
            .with_named_ma_type("ma_type", MovingAverageType::EMA)
            .with_param("pct", 2.0)
            .with_enum("envelope_mode", "Fixed")
            .with_source(OhlcvField::High),
        BarIndicatorId::Envbw => IndicatorConfig::new(id, name, vec![20])
            .with_named_ma_type("ma_type", MovingAverageType::EMA)
            .with_param("pct", 2.0),
        BarIndicatorId::Stddevchan => IndicatorConfig::new(id, name, vec![20])
            .with_param("std_dev", 2.0)
            .with_enum("std_dev_mode", "Simple")
            .with_enum("regression_source", "Close")
            .with_source(OhlcvField::High),
        BarIndicatorId::Stddevwidth => IndicatorConfig::new(id, name, vec![20])
            .with_param("std_dev", 2.0),
        BarIndicatorId::Starc => IndicatorConfig::new(id, name, vec![5, 15])
            .with_named_ma_type("ma_type", MovingAverageType::EMA)
            .with_named_ma_type("atr_ma_type", MovingAverageType::EMA)
            .with_param("k", 2.0),
        BarIndicatorId::Ichimoku => IndicatorConfig::new(id, name, vec![]),
        BarIndicatorId::Ichimokuthick => IndicatorConfig::new(id, name, vec![]),
        BarIndicatorId::Ichimokupos => IndicatorConfig::new(id, name, vec![]),
        BarIndicatorId::Pivot => IndicatorConfig::new(id, name, vec![1]),
        BarIndicatorId::Floorpivot => IndicatorConfig::new(id, name, vec![1]),
        BarIndicatorId::Camarilla => IndicatorConfig::new(id, name, vec![1]),
        BarIndicatorId::Woodie => IndicatorConfig::new(id, name, vec![1]),
        BarIndicatorId::Demark => IndicatorConfig::new(id, name, vec![1]),
        BarIndicatorId::Cpr => IndicatorConfig::new(id, name, vec![]),
        BarIndicatorId::Pivotchan => IndicatorConfig::new(id, name, vec![]),
        BarIndicatorId::Medchan => IndicatorConfig::new(id, name, vec![20])
            .with_param("mad_multiplier", 2.0)
            .with_enum("median_mode", "Simple")
            .with_enum("median_source", "Close")
            .with_source(OhlcvField::High),
        BarIndicatorId::Medchanpos => IndicatorConfig::new(id, name, vec![20]),
        BarIndicatorId::Qrchan => IndicatorConfig::new(id, name, vec![20])
            .with_param("bandwidth", 0.5),
        BarIndicatorId::Trimabands => IndicatorConfig::new(id, name, vec![20])
            .with_param("multiplier", 2.0)
            .with_source(OhlcvField::High),
        BarIndicatorId::Dpobands => IndicatorConfig::new(id, name, vec![20, 5])
            .with_named_ma_type("ma_type", MovingAverageType::EMA)
            .with_param("multiplier", 2.0),
        BarIndicatorId::Percentilech => IndicatorConfig::new(id, name, vec![50])
            .with_param("lower_q", 0.1)
            .with_param("upper_q", 0.9)
            .with_source(OhlcvField::High),
        BarIndicatorId::Adaptivechan => IndicatorConfig::new(id, name, vec![20])
            .with_named_ma_type("atr_ma_type", MovingAverageType::EMA)
            .with_param("adaptation_mode", 0.0)
            .with_param("center_line_type", 0.0)
            .with_param("volatility_lookback", 20.0),
        BarIndicatorId::Volprofchan => IndicatorConfig::new(id, name, vec![50])
            .with_param("mode", 0.0)
            .with_param("vp_period", 20.0)
            .with_param("last_n_bars", 100.0)
            .with_param("num_bins", 50.0)
            .with_param("value_area_percent", 0.7),

        // ---- Oscillators with wrap ----
        BarIndicatorId::Roc => {
            add_wrap_axes(
                IndicatorConfig::new(id, name, vec![14])
                    .with_param("use_log", 0.0)
                    .with_source(OhlcvField::High),
            )
        }
        BarIndicatorId::Cci => {
            add_wrap_axes(
                IndicatorConfig::new(id, name, vec![20])
                    .with_named_ma_type("ma_type", MovingAverageType::EMA)
                    .with_param("scalar", 0.015),
            )
        }
        BarIndicatorId::Stoch => {
            add_wrap_axes(IndicatorConfig::new(id, name, vec![14, 3]))
        }
        BarIndicatorId::Obv => {
            add_wrap_axes(
                IndicatorConfig::new(id, name, vec![])
                    .with_source(OhlcvField::High),
            )
        }
        BarIndicatorId::MoObv => IndicatorConfig::new(id, name, vec![])
            .with_source(OhlcvField::High),
        BarIndicatorId::Vhf => IndicatorConfig::new(id, name, vec![14])
            .with_source(OhlcvField::High),
        BarIndicatorId::Psl => {
            add_wrap_axes(
                IndicatorConfig::new(id, name, vec![20])
                    .with_source(OhlcvField::High),
            )
        }
        BarIndicatorId::Cmo => {
            add_wrap_axes(
                IndicatorConfig::new(id, name, vec![14])
                    .with_source(OhlcvField::High),
            )
        }
        BarIndicatorId::Bias => {
            add_wrap_axes(
                IndicatorConfig::new(id, name, vec![14])
                    .with_source(OhlcvField::High),
            )
        }
        BarIndicatorId::Amat => IndicatorConfig::new(id, name, vec![8, 21, 3])
            .with_named_ma_type("fast_ma", MovingAverageType::EMA)
            .with_named_ma_type("slow_ma", MovingAverageType::EMA)
            .with_source(OhlcvField::High),

        // ---- Book indicators ----
        BarIndicatorId::BookImb => IndicatorConfig::new(id, name, vec![]),
        BarIndicatorId::BookMicroprice => IndicatorConfig::new(id, name, vec![]),
        BarIndicatorId::BookSlope => IndicatorConfig::new(id, name, vec![]),
        BarIndicatorId::Ofi => IndicatorConfig::new(id, name, vec![14])
            .with_param("tick_size", 0.01),
        BarIndicatorId::QueueImb => IndicatorConfig::new(id, name, vec![]),
        BarIndicatorId::LiquiditySweep => IndicatorConfig::new(id, name, vec![]),
        BarIndicatorId::BookPressure => IndicatorConfig::new(id, name, vec![14])
            .with_param("levels", 5.0),
        BarIndicatorId::SpreadDistribution => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::OrderBookVelocity => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::WallDetector => IndicatorConfig::new(id, name, vec![20])
            .with_param("percentile_threshold", 95.0)
            .with_param("levels_to_sample", 10.0),
        BarIndicatorId::BookDepthChange => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::IcebergDetector => IndicatorConfig::new(id, name, vec![])
            .with_param("price_bucket", 0.01)
            .with_param("threshold", 3.0),
        BarIndicatorId::LevelReplenishRate => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::BookChurnRate => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::HiddenLiquidityDetector => IndicatorConfig::new(id, name, vec![14])
            .with_param("price_bucket", 0.01),
        BarIndicatorId::TradeBookAbsorption => IndicatorConfig::new(id, name, vec![14])
            .with_param("ratio", 0.7),
        BarIndicatorId::SweepImpactAnalyzer => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::BidAskAsymmetry => IndicatorConfig::new(id, name, vec![])
            .with_param("top_n", 5.0),
        BarIndicatorId::BidAskBounceRate => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::MidPriceVelocity => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::BestLevelVolatility => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::LayerConcentration => IndicatorConfig::new(id, name, vec![])
            .with_param("top_n", 5.0),
        BarIndicatorId::PriceLevelDensity => IndicatorConfig::new(id, name, vec![])
            .with_param("top_n", 5.0),

        // ---- Funding / OI ----
        BarIndicatorId::FundingMomentum => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::FundingZScore => IndicatorConfig::new(id, name, vec![50]),
        BarIndicatorId::OiChangeRate => IndicatorConfig::new(id, name, vec![]),
        BarIndicatorId::FundingPriceDivergence => IndicatorConfig::new(id, name, vec![14, 5]),
        BarIndicatorId::OiZScore => IndicatorConfig::new(id, name, vec![50]),
        BarIndicatorId::OiMomentum => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::OiPercentile => IndicatorConfig::new(id, name, vec![100]),
        BarIndicatorId::LongSqueezeDetector => IndicatorConfig::new(id, name, vec![]),
        BarIndicatorId::OiPriceCorrelation => IndicatorConfig::new(id, name, vec![20]),
        BarIndicatorId::MarkPriceVsLast => IndicatorConfig::new(id, name, vec![]),
        BarIndicatorId::IndexPriceMomentum => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::Volume24hMomentum => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::HighLowRangeRatio => IndicatorConfig::new(id, name, vec![]),
        BarIndicatorId::PriceChange24hZScore => IndicatorConfig::new(id, name, vec![50]),
        BarIndicatorId::LiquidationRate => IndicatorConfig::new(id, name, vec![])
            .with_param("window_ms", 60000.0),
        BarIndicatorId::LiquidationVolumeImbalance => IndicatorConfig::new(id, name, vec![])
            .with_param("window_ms", 60000.0),
        BarIndicatorId::LiquidationCascade => IndicatorConfig::new(id, name, vec![])
            .with_param("window_ms", 60000.0)
            .with_param("threshold_count", 3.0),
        BarIndicatorId::LiquidationVolumeVelocity => IndicatorConfig::new(id, name, vec![])
            .with_param("window_ms", 60000.0),
        BarIndicatorId::StopHuntDetector => IndicatorConfig::new(id, name, vec![])
            .with_param("spike_threshold_usd", 100000.0)
            .with_param("reversal_window_ms", 5000.0),
        BarIndicatorId::LiquidationClusterDetector => IndicatorConfig::new(id, name, vec![])
            .with_param("price_bucket", 0.01)
            .with_param("window_ms", 60000.0)
            .with_param("min_cluster_count", 3.0),
        BarIndicatorId::LiquidationCooldown => IndicatorConfig::new(id, name, vec![]),
        BarIndicatorId::ClQueueImb => IndicatorConfig::new(id, name, vec![]),
        BarIndicatorId::MarketMicro => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::OrderBookSlope => IndicatorConfig::new(id, name, vec![]),
        BarIndicatorId::OrderFlowImb => IndicatorConfig::new(id, name, vec![14])
            .with_param("tick_size", 0.01),
        BarIndicatorId::TickVolume => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::VwapLevels => IndicatorConfig::new(id, name, vec![20])
            .with_param("price_precision", 2.0),
        BarIndicatorId::FootprintChart => IndicatorConfig::new(id, name, vec![])
            .with_param("price_bucket", 0.01),
        BarIndicatorId::FootprintImbalance => IndicatorConfig::new(id, name, vec![])
            .with_param("price_bucket", 0.01)
            .with_param("threshold_pct", 70.0),
        BarIndicatorId::FootprintPoc => IndicatorConfig::new(id, name, vec![])
            .with_param("price_bucket", 0.01),
        BarIndicatorId::AbsorptionDetector => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::TradeClusterDetector => IndicatorConfig::new(id, name, vec![])
            .with_param("price_bucket", 0.01)
            .with_param("cluster_threshold", 3.0)
            .with_param("window_ms", 60000.0),
        BarIndicatorId::Sampen => IndicatorConfig::new(id, name, vec![2])
            .with_param("pattern_length", 2.0)
            .with_param("threshold_ratio", 0.2)
            .with_source(OhlcvField::High),
        BarIndicatorId::Xmil => IndicatorConfig::new(id, name, vec![50])
            .with_param("bins", 10.0)
            .with_param("clip_abs", 3.0)
            .with_param("lag_1", 1.0)
            .with_param("lag_2", 2.0)
            .with_param("lag_3", 3.0)
            .with_param("lag_4", 4.0)
            .with_param("lag_5", 5.0),

        // ---- Trend Stop ----
        BarIndicatorId::Supertrend => IndicatorConfig::new(id, name, vec![7])
            .with_named_ma_type("atr_ma_type", MovingAverageType::EMA)
            .with_param("multiplier", 3.0),
        BarIndicatorId::Ssl => IndicatorConfig::new(id, name, vec![10])
            .with_named_ma_type("ma_type", MovingAverageType::EMA),
        BarIndicatorId::Gmma => IndicatorConfig::new(id, name, vec![]),
        BarIndicatorId::Tii => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::Ravi => IndicatorConfig::new(id, name, vec![7, 65]),
        BarIndicatorId::Didi => IndicatorConfig::new(id, name, vec![3, 8, 20]),
        BarIndicatorId::HaTrend => IndicatorConfig::new(id, name, vec![]),
        BarIndicatorId::Er => IndicatorConfig::new(id, name, vec![14])
            .with_source(OhlcvField::High),
        BarIndicatorId::ErRing => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::SpreadAnalyzer => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::Vr => IndicatorConfig::new(id, name, vec![14, 5]),
        BarIndicatorId::VrAgg => IndicatorConfig::new(id, name, vec![2, 4, 8, 16, 32, 64]),
        BarIndicatorId::ArchLm => IndicatorConfig::new(id, name, vec![5, 10]),
        BarIndicatorId::Kpss => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::Adf => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::Pacf => IndicatorConfig::new(id, name, vec![50, 10]),
        BarIndicatorId::LjungBox => IndicatorConfig::new(id, name, vec![50, 10]),
        BarIndicatorId::Psars => IndicatorConfig::new(id, name, vec![])
            .with_param("af_start", 0.02)
            .with_param("af_increment", 0.02)
            .with_param("af_max", 0.2),
        BarIndicatorId::Supts => IndicatorConfig::new(id, name, vec![7])
            .with_named_ma_type("atr_ma_type", MovingAverageType::EMA)
            .with_param("multiplier", 3.0),
        BarIndicatorId::Atrts => IndicatorConfig::new(id, name, vec![14])
            .with_param("multiplier", 3.0),
        BarIndicatorId::Chand => IndicatorConfig::new(id, name, vec![22])
            .with_param("multiplier", 3.0),
        BarIndicatorId::Volts => IndicatorConfig::new(id, name, vec![20])
            .with_named_ma_type("ma_type", MovingAverageType::EMA)
            .with_param("multiplier", 2.0)
            .with_param("volatility_type", 0.0),
        BarIndicatorId::Kelts => IndicatorConfig::new(id, name, vec![20])
            .with_named_ma_type("center_ma", MovingAverageType::EMA)
            .with_named_ma_type("atr_ma", MovingAverageType::EMA)
            .with_param("multiplier", 2.0),
        BarIndicatorId::Dons => IndicatorConfig::new(id, name, vec![20, 5])
            .with_param("offset", 0.0)
            .with_param("use_percentage", 0.0),
        BarIndicatorId::Cks => IndicatorConfig::new(id, name, vec![10, 1, 5])
            .with_named_ma_type("atr_ma_type", MovingAverageType::EMA)
            .with_param("k", 3.0),
        BarIndicatorId::Donbo => IndicatorConfig::new(id, name, vec![20]),
        BarIndicatorId::Alligator => IndicatorConfig::new(id, name, vec![]),
        BarIndicatorId::Ao => IndicatorConfig::new(id, name, vec![]),
        BarIndicatorId::Ac => IndicatorConfig::new(id, name, vec![]),
        BarIndicatorId::WilliamsMfi => IndicatorConfig::new(id, name, vec![]),
        BarIndicatorId::Esine => IndicatorConfig::new(id, name, vec![])
            .with_param("alpha", 0.07),
        BarIndicatorId::Cyber => IndicatorConfig::new(id, name, vec![])
            .with_param("alpha", 0.07),
        BarIndicatorId::TsSwings => IndicatorConfig::new(id, name, vec![5])
            .with_param("min_swing_size", 0.5)
            .with_param("offset", 0.0)
            .with_param("use_percentage", 0.0),
        BarIndicatorId::VoltsAtr => IndicatorConfig::new(id, name, vec![20])
            .with_named_ma_type("ma_type", MovingAverageType::EMA)
            .with_param("multiplier", 2.0),
        BarIndicatorId::Hdc => IndicatorConfig::new(id, name, vec![])
            .with_source(OhlcvField::High),
        BarIndicatorId::Kalman => IndicatorConfig::new(id, name, vec![])
            .with_param("dt", 1.0)
            .with_param("process_noise", 1e-5)
            .with_param("measurement_noise", 1e-2),
        BarIndicatorId::Ekf => IndicatorConfig::new(id, name, vec![])
            .with_param("dt", 1.0)
            .with_param("process_noise_std", 1e-3)
            .with_param("measurement_noise_std", 1e-2)
            .with_param("friction", 0.0)
            .with_param("observation_type", 0.0),
        BarIndicatorId::Ukf => IndicatorConfig::new(id, name, vec![])
            .with_param("dt", 1.0)
            .with_param("process_noise_std", 1e-3)
            .with_param("measurement_noise_std", 1e-2)
            .with_param("ut_alpha", 1e-3)
            .with_param("ut_beta", 2.0)
            .with_param("ut_kappa", 0.0),
        BarIndicatorId::Particle => IndicatorConfig::new(id, name, vec![100])
            .with_param("dt", 1.0)
            .with_param("process_noise_std", 1e-3)
            .with_param("measurement_noise_std", 1e-2)
            .with_param("resampling", 1.0),
        BarIndicatorId::Adaptivema => IndicatorConfig::new(id, name, vec![14])
            .with_param("adaptation_mode", 0.0)
            .with_param("efficiency_method", 0.0),
        BarIndicatorId::Kama => IndicatorConfig::new(id, name, vec![10, 2, 30])
            .with_source(OhlcvField::High),
        BarIndicatorId::Vidya => IndicatorConfig::new(id, name, vec![14])
            .with_param("cmo_ma_type", 0.0),
        BarIndicatorId::Mama => IndicatorConfig::new(id, name, vec![])
            .with_named_ma_type("ma_type", MovingAverageType::EMA)
            .with_param("fast_limit", 0.5)
            .with_param("slow_limit", 0.05)
            .with_source(OhlcvField::High),
        BarIndicatorId::Ess => IndicatorConfig::new(id, name, vec![])
            .with_param("period", 14.0)
            .with_source(OhlcvField::High),
        BarIndicatorId::AtrRsi => {
            add_wrap_axes(IndicatorConfig::new(id, name, vec![]))
        }
        BarIndicatorId::Vwrsi => {
            add_wrap_axes(
                IndicatorConfig::new(id, name, vec![14, 14])
                    .with_source(OhlcvField::High),
            )
        }
        BarIndicatorId::Rsioma => {
            add_wrap_axes(
                IndicatorConfig::new(id, name, vec![14, 14])
                    .with_named_ma_type("ma_type", MovingAverageType::EMA),
            )
        }
        BarIndicatorId::Tdi => IndicatorConfig::new(id, name, vec![13, 2, 7])
            .with_named_ma_type("signal_ma_type", MovingAverageType::EMA),
        BarIndicatorId::Thresh => IndicatorConfig::new(id, name, vec![14])
            .with_param("lower", 20.0)
            .with_param("upper", 80.0),
        BarIndicatorId::Logicand => IndicatorConfig::new(id, name, vec![14, 14]),
        BarIndicatorId::Logicor => IndicatorConfig::new(id, name, vec![14, 14]),
        BarIndicatorId::Logicxor => IndicatorConfig::new(id, name, vec![14, 14]),
        BarIndicatorId::Logicsign => IndicatorConfig::new(id, name, vec![14, 14]),
        BarIndicatorId::Rv => IndicatorConfig::new(id, name, vec![20])
            .with_param("annualize_factor", 252.0),
        BarIndicatorId::Rvz => IndicatorConfig::new(id, name, vec![20, 100]),
        BarIndicatorId::Autocorr => IndicatorConfig::new(id, name, vec![20, 5]),
        BarIndicatorId::Hmom => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::RocPct => {
            add_wrap_axes(IndicatorConfig::new(id, name, vec![14, 100]))
        }
        BarIndicatorId::Nr => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::Mrf => IndicatorConfig::new(id, name, vec![]),
        BarIndicatorId::AdaptiveStoch => {
            add_wrap_axes(IndicatorConfig::new(id, name, vec![]))
        }
        BarIndicatorId::Vbexp => IndicatorConfig::new(id, name, vec![]),
        BarIndicatorId::Cusum => IndicatorConfig::new(id, name, vec![])
            .with_param("threshold", 1.0),
        BarIndicatorId::StCusum => IndicatorConfig::new(id, name, vec![])
            .with_param("threshold", 1.0),
        BarIndicatorId::Mi => IndicatorConfig::new(id, name, vec![50, 5])
            .with_param("bins", 10.0)
            .with_param("clip_abs", 3.0),
        BarIndicatorId::Te => IndicatorConfig::new(id, name, vec![50, 5])
            .with_param("bins", 10.0)
            .with_param("clip_abs", 3.0),
        BarIndicatorId::Kld => IndicatorConfig::new(id, name, vec![50])
            .with_param("bins", 10.0)
            .with_param("clip_abs", 3.0),
        BarIndicatorId::Jsd => IndicatorConfig::new(id, name, vec![50])
            .with_param("bins", 10.0)
            .with_param("clip_abs", 3.0),
        BarIndicatorId::Fisher => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::Infog => IndicatorConfig::new(id, name, vec![50])
            .with_param("bins", 10.0)
            .with_param("clip_abs", 3.0),
        BarIndicatorId::Lz => IndicatorConfig::new(id, name, vec![50]),
        BarIndicatorId::Hyst => IndicatorConfig::new(id, name, vec![14])
            .with_param("lower", 20.0)
            .with_param("upper", 80.0),
        BarIndicatorId::Wcomp => IndicatorConfig::new(id, name, vec![5, 10, 20, 40])
            .with_named_ma_type("ma_type", MovingAverageType::EMA)
            .with_param("w1", 0.25)
            .with_param("w2", 0.25)
            .with_param("w3", 0.25)
            .with_param("w4", 0.25)
            .with_param("normalize", 1.0),
        BarIndicatorId::EhlersRocket => IndicatorConfig::new(id, name, vec![])
            .with_source(OhlcvField::High),
        BarIndicatorId::EhlersCc => IndicatorConfig::new(id, name, vec![])
            .with_param("alpha", 0.07),
        BarIndicatorId::Zlsma => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::VwapDist => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::Hlva => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::SweepRev => IndicatorConfig::new(id, name, vec![20, 5])
            .with_named_ma_type("ma_type", MovingAverageType::EMA)
            .with_param("close_quartile", 0.25)
            .with_param("weight_k", 1.5)
            .with_param("confirm_next_bar", 1.0),
        BarIndicatorId::Conden => IndicatorConfig::new(id, name, vec![50])
            .with_param("bins", 10.0)
            .with_param("clip_abs", 3.0),
        BarIndicatorId::HalfLifeMr => IndicatorConfig::new(id, name, vec![50]),
        BarIndicatorId::ResidStat => IndicatorConfig::new(id, name, vec![50]),
        BarIndicatorId::Coint => IndicatorConfig::new(id, name, vec![50])
            .with_named_ma_type("ma_type", MovingAverageType::EMA),
        BarIndicatorId::EgCoint => IndicatorConfig::new(id, name, vec![50])
            .with_named_ma_type("ma_type", MovingAverageType::EMA),
        BarIndicatorId::EgAdf => IndicatorConfig::new(id, name, vec![50, 5])
            .with_named_ma_type("ma_type", MovingAverageType::EMA),
        BarIndicatorId::EgTrend => IndicatorConfig::new(id, name, vec![50]),
        BarIndicatorId::MarketCipher => IndicatorConfig::new(id, name, vec![])
            .with_source(OhlcvField::High),
        BarIndicatorId::AdxSlope => IndicatorConfig::new(id, name, vec![14])
            .with_named_ma_type("atr_ma_type", MovingAverageType::EMA),
        BarIndicatorId::GannHilo => IndicatorConfig::new(id, name, vec![5]),
        BarIndicatorId::Hampel => IndicatorConfig::new(id, name, vec![14])
            .with_param("k", 3.0),
        BarIndicatorId::Theilsenchan => IndicatorConfig::new(id, name, vec![50])
            .with_param("k", 2.0),
        BarIndicatorId::Projbands => IndicatorConfig::new(id, name, vec![20])
            .with_param("k", 2.0)
            .with_source(OhlcvField::High),
        BarIndicatorId::Avr => IndicatorConfig::new(id, name, vec![]),
        BarIndicatorId::NeuralMom => IndicatorConfig::new(id, name, vec![]),
        BarIndicatorId::Candleanatomy => IndicatorConfig::new(id, name, vec![])
            .with_param("long_wick_ratio_threshold", 0.3),
        BarIndicatorId::Swingstr => IndicatorConfig::new(id, name, vec![5, 3]),
        BarIndicatorId::Liqgap => IndicatorConfig::new(id, name, vec![20])
            .with_param("threshold", 2.0),
        BarIndicatorId::Wickspike => IndicatorConfig::new(id, name, vec![20]),
        BarIndicatorId::SwingAge => IndicatorConfig::new(id, name, vec![5]),
        BarIndicatorId::Rcb => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::Stft => IndicatorConfig::new(id, name, vec![64, 16]),
        BarIndicatorId::Fvg => IndicatorConfig::new(id, name, vec![]),
        BarIndicatorId::Bos => IndicatorConfig::new(id, name, vec![20]),
        BarIndicatorId::Arima => IndicatorConfig::new(id, name, vec![1, 1, 1]),
        BarIndicatorId::Arimax => IndicatorConfig::new(id, name, vec![1, 1, 1, 0]),
        BarIndicatorId::Garch => IndicatorConfig::new(id, name, vec![1, 1]),
        BarIndicatorId::Egarch => IndicatorConfig::new(id, name, vec![1, 1]),
        BarIndicatorId::Var => IndicatorConfig::new(id, name, vec![2]),
        BarIndicatorId::PolyReg => IndicatorConfig::new(id, name, vec![20])
            .with_source(OhlcvField::High),
        BarIndicatorId::Aroon => IndicatorConfig::new(id, name, vec![25]),
        BarIndicatorId::AroonOsc => IndicatorConfig::new(id, name, vec![25]),
        BarIndicatorId::AroonUp => IndicatorConfig::new(id, name, vec![25]),
        BarIndicatorId::AroonDown => IndicatorConfig::new(id, name, vec![25]),
        BarIndicatorId::Highest => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::Lowest => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::Pressure => IndicatorConfig::new(id, name, vec![14])
            .with_named_ma_type("ma_type", MovingAverageType::EMA)
            .with_named_ma_type("atr_ma_type", MovingAverageType::EMA),
        BarIndicatorId::Tmf => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::Rc => IndicatorConfig::new(id, name, vec![14, 20, 30, 40, 50, 60, 70, 80, 90])
            .with_param("ser_low_cut", 0.02),
        BarIndicatorId::Avwap => IndicatorConfig::new(id, name, vec![]),
        BarIndicatorId::Pivavwap => IndicatorConfig::new(id, name, vec![5]),
        BarIndicatorId::Atrp => IndicatorConfig::new(id, name, vec![14, 5])
            .with_named_ma_type("ma_type", MovingAverageType::EMA),
        BarIndicatorId::Atrbw => IndicatorConfig::new(id, name, vec![14])
            .with_named_ma_type("ma_type", MovingAverageType::EMA),
        BarIndicatorId::Atrz => IndicatorConfig::new(id, name, vec![14, 100])
            .with_named_ma_type("ma_type", MovingAverageType::EMA),
        BarIndicatorId::C2cvp => IndicatorConfig::new(id, name, vec![20, 100]),
        BarIndicatorId::Pvt => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::Vz => IndicatorConfig::new(id, name, vec![20]),
        BarIndicatorId::Sdl => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::EmaSlope => IndicatorConfig::new(id, name, vec![9, 5]),
        BarIndicatorId::Rmid => IndicatorConfig::new(id, name, vec![20]),
        BarIndicatorId::Pgry => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::RangeAtr => IndicatorConfig::new(id, name, vec![14])
            .with_named_ma_type("ma_type", MovingAverageType::EMA),
        BarIndicatorId::Rquart => IndicatorConfig::new(id, name, vec![20]),
        BarIndicatorId::Fft => IndicatorConfig::new(id, name, vec![64])
            .with_param("sampling_rate", 1.0),
        BarIndicatorId::Wave => IndicatorConfig::new(id, name, vec![64])
            .with_param("wavelet_type", 0.0)
            .with_source(OhlcvField::High),
        BarIndicatorId::Hilb => IndicatorConfig::new(id, name, vec![64])
            .with_param("sampling_rate", 1.0)
            .with_source(OhlcvField::High),
        BarIndicatorId::Sent => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::Shmpr => IndicatorConfig::new(id, name, vec![50])
            .with_param("target_fraction", 0.5),
        BarIndicatorId::Screst => IndicatorConfig::new(id, name, vec![64]),
        BarIndicatorId::Sentent => IndicatorConfig::new(id, name, vec![14])
            .with_param("smoothing_alpha", 0.1),
        BarIndicatorId::Ser => IndicatorConfig::new(id, name, vec![50])
            .with_param("low_cut_fraction", 0.1),
        BarIndicatorId::Srollrp => IndicatorConfig::new(id, name, vec![64]),
        BarIndicatorId::Sslopez => IndicatorConfig::new(id, name, vec![64, 5]),
        BarIndicatorId::Sflux => IndicatorConfig::new(id, name, vec![64])
            .with_param("low_cut_fraction", 0.1)
            .with_param("high_cut_fraction", 0.9),
        BarIndicatorId::Sflatp => IndicatorConfig::new(id, name, vec![64])
            .with_param("low_cut_fraction", 0.1)
            .with_param("high_cut_fraction", 0.9),
        BarIndicatorId::Sbprhl => IndicatorConfig::new(id, name, vec![64]),
        BarIndicatorId::Sbwf => IndicatorConfig::new(id, name, vec![64]),
        BarIndicatorId::Sbp => IndicatorConfig::new(id, name, vec![64])
            .with_param("low_cut_fraction", 0.1)
            .with_param("high_cut_fraction", 0.9),
        BarIndicatorId::Butter => IndicatorConfig::new(id, name, vec![64])
            .with_param("cutoff_hz", 0.1)
            .with_param("sampling_rate", 1.0)
            .with_param("filter_type", 0.0)
            .with_param("low_cut_hz", 0.05)
            .with_param("high_cut_hz", 0.3),
        BarIndicatorId::Cheby => IndicatorConfig::new(id, name, vec![64])
            .with_param("cutoff_fraction", 0.1)
            .with_param("ripple_db", 0.5)
            .with_param("cheby_type", 1.0)
            .with_param("filter_type", 0.0),
        BarIndicatorId::Sg => IndicatorConfig::new(id, name, vec![11, 2])
            .with_param("derivative", 0.0),
        BarIndicatorId::Iip => IndicatorConfig::new(id, name, vec![]),
        BarIndicatorId::Iir => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::Abgfilter => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::Kcomp => IndicatorConfig::new(id, name, vec![14, 20, 10, 5, 30])
            .with_named_ma_type("atr_ma", MovingAverageType::EMA)
            .with_param("dt", 1.0)
            .with_param("q", 1e-4)
            .with_param("r", 1e-2)
            .with_param("decay", 0.99)
            .with_param("w_regime", 0.4)
            .with_param("w_atr", 0.3)
            .with_param("w_vov", 0.3),
        BarIndicatorId::Kregime => IndicatorConfig::new(id, name, vec![14])
            .with_param("dt", 1.0)
            .with_param("process_noise", 1e-5)
            .with_param("measurement_noise", 1e-2),
        BarIndicatorId::Kscr => IndicatorConfig::new(id, name, vec![14])
            .with_param("dt", 1.0)
            .with_param("process_noise", 1e-5)
            .with_param("measurement_noise", 1e-2)
            .with_param("decay", 0.99),
        BarIndicatorId::Kslope => IndicatorConfig::new(id, name, vec![14])
            .with_param("dt", 1.0)
            .with_param("process_noise", 1e-5)
            .with_param("measurement_noise", 1e-2),
        BarIndicatorId::Kslopez => IndicatorConfig::new(id, name, vec![14])
            .with_param("dt", 1.0)
            .with_param("process_noise", 1e-5)
            .with_param("measurement_noise", 1e-2),
        BarIndicatorId::Screstp => IndicatorConfig::new(id, name, vec![64, 5]),
        BarIndicatorId::Slmpr => IndicatorConfig::new(id, name, vec![64])
            .with_param("low_cut_fraction", 0.1)
            .with_param("high_cut_fraction", 0.9),
        BarIndicatorId::Sroll95 => IndicatorConfig::new(id, name, vec![64]),
        BarIndicatorId::Srollp => IndicatorConfig::new(id, name, vec![64, 5])
            .with_param("rolloff_percent", 95.0),
        BarIndicatorId::Sslopep => IndicatorConfig::new(id, name, vec![64, 5]),
        BarIndicatorId::Ssloperp => IndicatorConfig::new(id, name, vec![64, 5]),
        BarIndicatorId::AdfKpss => IndicatorConfig::new(id, name, vec![14, 5])
            .with_param("use_trend", 0.0),
        BarIndicatorId::ArchLmPval => IndicatorConfig::new(id, name, vec![5, 10]),
        BarIndicatorId::BpCusum => IndicatorConfig::new(id, name, vec![20])
            .with_param("threshold", 1.0)
            .with_param("kappa", 0.5),
        BarIndicatorId::KpssTrend => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::KpssZ => IndicatorConfig::new(id, name, vec![14, 50]),
        BarIndicatorId::Pp => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::PvCoherence => IndicatorConfig::new(id, name, vec![64]),
        BarIndicatorId::RSquared => IndicatorConfig::new(id, name, vec![20]),
        BarIndicatorId::VrZAgg => IndicatorConfig::new(id, name, vec![100])
            .with_param("pair_1_short", 2.0)
            .with_param("pair_1_long", 4.0)
            .with_param("pair_2_short", 8.0)
            .with_param("pair_2_long", 16.0)
            .with_param("pair_3_short", 32.0)
            .with_param("pair_3_long", 64.0),
        BarIndicatorId::Za => IndicatorConfig::new(id, name, vec![50]),
        BarIndicatorId::Dvr => IndicatorConfig::new(id, name, vec![]),
        BarIndicatorId::Bbmetrics => IndicatorConfig::new(id, name, vec![20])
            .with_named_ma_type("ma_type", MovingAverageType::EMA)
            .with_param("std_dev", 2.0),
        BarIndicatorId::Dcmetrics => IndicatorConfig::new(id, name, vec![20]),
        BarIndicatorId::Kcmetrics => IndicatorConfig::new(id, name, vec![20])
            .with_named_ma_type("center_ma_type", MovingAverageType::EMA)
            .with_named_ma_type("atr_ma_type", MovingAverageType::EMA)
            .with_param("atr_mult", 2.0),
        BarIndicatorId::Percentb => IndicatorConfig::new(id, name, vec![20])
            .with_named_ma_type("ma_type", MovingAverageType::EMA)
            .with_param("std_mult", 2.0),
        BarIndicatorId::Atrc => IndicatorConfig::new(id, name, vec![14, 20])
            .with_named_ma_type("ma_type", MovingAverageType::EMA)
            .with_named_ma_type("atr_ma_type", MovingAverageType::EMA)
            .with_param("multiplier", 2.0),
        BarIndicatorId::Rbvj => IndicatorConfig::new(id, name, vec![20])
            .with_param("annualize_factor", 252.0),
        BarIndicatorId::Vovp => IndicatorConfig::new(id, name, vec![20, 5]),
        BarIndicatorId::Vovpt => IndicatorConfig::new(id, name, vec![20, 5])
            .with_param("alpha", 0.9),
        BarIndicatorId::Pvo => IndicatorConfig::new(id, name, vec![12, 26, 9])
            .with_named_ma_type("fast_ma_type", MovingAverageType::EMA)
            .with_named_ma_type("slow_ma_type", MovingAverageType::EMA)
            .with_named_ma_type("signal_ma_type", MovingAverageType::EMA),
        BarIndicatorId::Pzo => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::Rvol => IndicatorConfig::new(id, name, vec![20]),
        BarIndicatorId::Vo => IndicatorConfig::new(id, name, vec![5, 10]),
        BarIndicatorId::DistLevels => IndicatorConfig::new(id, name, vec![20, 5])
            .with_param("low_pct", 0.02)
            .with_param("high_pct", 0.05),
        BarIndicatorId::AvwapDist => IndicatorConfig::new(id, name, vec![]),
        BarIndicatorId::Avwaprev => IndicatorConfig::new(id, name, vec![20]),
        BarIndicatorId::Avwaptouch => IndicatorConfig::new(id, name, vec![50])
            .with_param("touch_threshold", 0.002),
        BarIndicatorId::TrEr => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::Heikinashi => IndicatorConfig::new(id, name, vec![]),
        BarIndicatorId::Abb => IndicatorConfig::new(id, name, vec![20, 14, 14])
            .with_named_ma_type("ma_type", MovingAverageType::EMA)
            .with_named_ma_type("atr_ma_type", MovingAverageType::EMA)
            .with_param("multiplier", 2.0),
        BarIndicatorId::Cv => IndicatorConfig::new(id, name, vec![14, 10]),
        BarIndicatorId::Rp => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::Vov => IndicatorConfig::new(id, name, vec![14, 5])
            .with_named_ma_type("atr_ma_type", MovingAverageType::EMA)
            .with_enum("vov_source", "AbsReturn"),
        BarIndicatorId::Vprb => IndicatorConfig::new(id, name, vec![14, 5])
            .with_named_ma_type("atr_ma_type", MovingAverageType::EMA),
        BarIndicatorId::Wvf => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::Darvas => IndicatorConfig::new(id, name, vec![10]),
        BarIndicatorId::Cog => {
            add_wrap_axes(IndicatorConfig::new(id, name, vec![14]))
        }
        BarIndicatorId::Pfe => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::EwmacRobust => IndicatorConfig::new(id, name, vec![8, 16, 32]),
        BarIndicatorId::Gapo => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::Gator => {
            add_wrap_axes(
                IndicatorConfig::new(id, name, vec![5, 8])
                    .with_named_ma_type("ma_type", MovingAverageType::EMA)
                    .with_source(OhlcvField::High),
            )
        }
        BarIndicatorId::MacdHist => {
            add_wrap_axes(
                IndicatorConfig::new(id, name, vec![12, 26, 9])
                    .with_named_ma_type("fast_ma_type", MovingAverageType::EMA)
                    .with_named_ma_type("slow_ma_type", MovingAverageType::EMA)
                    .with_named_ma_type("signal_ma_type", MovingAverageType::EMA)
                    .with_source(OhlcvField::High),
            )
        }
        BarIndicatorId::MacdSignal => {
            add_wrap_axes(
                IndicatorConfig::new(id, name, vec![12, 26, 9])
                    .with_named_ma_type("fast_ma_type", MovingAverageType::EMA)
                    .with_named_ma_type("slow_ma_type", MovingAverageType::EMA)
                    .with_named_ma_type("signal_ma_type", MovingAverageType::EMA)
                    .with_source(OhlcvField::High),
            )
        }
        BarIndicatorId::PpoSignal => IndicatorConfig::new(id, name, vec![12, 26, 9])
            .with_named_ma_type("fast_ma_type", MovingAverageType::EMA)
            .with_named_ma_type("slow_ma_type", MovingAverageType::EMA)
            .with_named_ma_type("signal_ma_type", MovingAverageType::EMA),
        BarIndicatorId::Scf => IndicatorConfig::new(id, name, vec![32]),
        BarIndicatorId::Sentr => IndicatorConfig::new(id, name, vec![14])
            .with_param("smoothing_alpha", 0.1),
        BarIndicatorId::Sflat => IndicatorConfig::new(id, name, vec![64]),
        BarIndicatorId::Sroll => IndicatorConfig::new(id, name, vec![64])
            .with_param("target_fraction", 0.5),
        BarIndicatorId::Sslope => IndicatorConfig::new(id, name, vec![64]),
        BarIndicatorId::KamaSlope => IndicatorConfig::new(id, name, vec![10]),
        BarIndicatorId::LrSlope => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::Rts => IndicatorConfig::new(id, name, vec![]),
        BarIndicatorId::Vfi => {
            add_wrap_axes(IndicatorConfig::new(id, name, vec![130]))
        }
        BarIndicatorId::Vzo => {
            add_wrap_axes(IndicatorConfig::new(id, name, vec![14]))
        }
        BarIndicatorId::HourDay => IndicatorConfig::new(id, name, vec![]),
        BarIndicatorId::WeekMonth => IndicatorConfig::new(id, name, vec![]),
        BarIndicatorId::SoqEoq => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::Tenc => IndicatorConfig::new(id, name, vec![]),
        BarIndicatorId::Weekday => IndicatorConfig::new(id, name, vec![]),
        BarIndicatorId::Session => IndicatorConfig::new(id, name, vec![]),
        BarIndicatorId::MonthQtr => IndicatorConfig::new(id, name, vec![]),
        BarIndicatorId::DomWoq => IndicatorConfig::new(id, name, vec![]),
        BarIndicatorId::RelTrendPos => IndicatorConfig::new(id, name, vec![14])
            .with_named_ma_type("ma_type", MovingAverageType::EMA),
        BarIndicatorId::MonthTurn => IndicatorConfig::new(id, name, vec![5]),
        BarIndicatorId::QtrTurn => IndicatorConfig::new(id, name, vec![5]),
        BarIndicatorId::SomEom => IndicatorConfig::new(id, name, vec![5]),
        BarIndicatorId::SowEow => IndicatorConfig::new(id, name, vec![5]),
        BarIndicatorId::HolidayProx => IndicatorConfig::new(id, name, vec![5]),
        BarIndicatorId::Vbd => IndicatorConfig::new(id, name, vec![])
            .with_named_ma_type("atr_ma_type", MovingAverageType::EMA)
            .with_param("mild_threshold", 1.0)
            .with_param("strong_threshold", 2.0)
            .with_param("extreme_threshold", 3.0)
            .with_param("squeeze_threshold", 0.5),
        BarIndicatorId::Poc => IndicatorConfig::new(id, name, vec![50, 10])
            .with_param("min_volume_threshold", 0.0),
        BarIndicatorId::Fvgalt => IndicatorConfig::new(id, name, vec![])
            .with_param("alpha", 0.1),
        BarIndicatorId::Fvgdur => IndicatorConfig::new(id, name, vec![20, 5]),
        BarIndicatorId::Fvgrev => IndicatorConfig::new(id, name, vec![20]),
        BarIndicatorId::DayWeekMonth => IndicatorConfig::new(id, name, vec![]),
        BarIndicatorId::LongShortRatioMomentum => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::LongShortExtremeDetector => IndicatorConfig::new(id, name, vec![])
            .with_param("upper_threshold", 0.7)
            .with_param("lower_threshold", 0.3),
        BarIndicatorId::RatioVsPriceDivergence => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::AggTradeFlowImbalance => IndicatorConfig::new(id, name, vec![])
            .with_param("window_ms", 60000.0),
        BarIndicatorId::AggTradeSizeDistribution => IndicatorConfig::new(id, name, vec![20]),
        BarIndicatorId::PriceVsIndexSpread => IndicatorConfig::new(id, name, vec![]),
        BarIndicatorId::IndexComponentDrift => IndicatorConfig::new(id, name, vec![]),
        BarIndicatorId::IndexCorrelationBreakdown => IndicatorConfig::new(id, name, vec![20]),
        BarIndicatorId::BasisMomentum => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::BasisExtreme => IndicatorConfig::new(id, name, vec![50]),
        BarIndicatorId::BasisZScore => IndicatorConfig::new(id, name, vec![50]),
        BarIndicatorId::HvMomentum => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::HvSpike => IndicatorConfig::new(id, name, vec![14])
            .with_param("multiplier", 2.0),
        BarIndicatorId::VolIdxSpike => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::VolIdxMomentum => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::DeltaExposureFlow => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::GammaSqueezeDetector => IndicatorConfig::new(id, name, vec![])
            .with_param("gamma_threshold", 0.01)
            .with_param("price_move_threshold", 0.005),
        BarIndicatorId::IvSkew => IndicatorConfig::new(id, name, vec![]),
        BarIndicatorId::CharmTracker => IndicatorConfig::new(id, name, vec![]),
        BarIndicatorId::VegaExposureFlow => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::ThetaDecayTracker => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::PinRiskDetector => IndicatorConfig::new(id, name, vec![])
            .with_param("delta_target", 0.5)
            .with_param("delta_tolerance", 0.1)
            .with_param("theta_threshold", -0.01),
        BarIndicatorId::AnnualizedFundingRate => IndicatorConfig::new(id, name, vec![])
            .with_param("funding_periods_per_day", 3.0),
        BarIndicatorId::FundingDirectionShift => IndicatorConfig::new(id, name, vec![]),
        BarIndicatorId::FundingExtremeAlert => IndicatorConfig::new(id, name, vec![20])
            .with_param("sigma_threshold", 2.0),
        BarIndicatorId::MarkPriceMomentum => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::MarkPriceVolatility => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::MarkPriceGapDetector => IndicatorConfig::new(id, name, vec![20])
            .with_param("sigma_threshold", 2.0),
        BarIndicatorId::FundDepletionRate => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::InsuranceFundMomentum => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::SettlementApproachSignal => IndicatorConfig::new(id, name, vec![14])
            .with_param("max_window_ms", 3600000.0),
        BarIndicatorId::FundStressDetector => IndicatorConfig::new(id, name, vec![20])
            .with_param("threshold", 2.0),
        BarIndicatorId::SettlementPriceMomentum => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::SettlementVsMarkSpread => IndicatorConfig::new(id, name, vec![]),
        BarIndicatorId::BlockTradeFlow => IndicatorConfig::new(id, name, vec![])
            .with_param("window_ms", 60000.0),
        BarIndicatorId::BlockTradeImpact => IndicatorConfig::new(id, name, vec![])
            .with_param("window_ms", 60000.0),
        BarIndicatorId::L3OrderRate => IndicatorConfig::new(id, name, vec![])
            .with_param("window_ms", 60000.0),
        BarIndicatorId::L3LargeOrderTracker => IndicatorConfig::new(id, name, vec![20])
            .with_param("threshold_multiplier", 3.0),
        BarIndicatorId::L3CancelRatio => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::BlockTradeSizeAnomaly => IndicatorConfig::new(id, name, vec![20]),
        BarIndicatorId::QuoteStuffingDetector => IndicatorConfig::new(id, name, vec![])
            .with_param("window_ms", 1000.0)
            .with_param("rate_threshold", 100.0),
        BarIndicatorId::LeverageReductionWarning => IndicatorConfig::new(id, name, vec![]),
        BarIndicatorId::MmrTracker => IndicatorConfig::new(id, name, vec![]),
        BarIndicatorId::RiskLimitProximity => IndicatorConfig::new(id, name, vec![]),
        BarIndicatorId::FundingDrift => IndicatorConfig::new(id, name, vec![]),
        BarIndicatorId::FundingTimeDecay => IndicatorConfig::new(id, name, vec![14])
            .with_param("max_window_ms", 28800000.0),
        BarIndicatorId::PredictedFundingExtreme => IndicatorConfig::new(id, name, vec![])
            .with_param("threshold", 0.01),
        BarIndicatorId::SettledFundingMomentum => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::FundingSettlementImpact => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::AuctionLiquidityScore => IndicatorConfig::new(id, name, vec![20]),
        BarIndicatorId::AuctionPriceDeviation => IndicatorConfig::new(id, name, vec![]),
        BarIndicatorId::AuctionImbalance => IndicatorConfig::new(id, name, vec![20]),
        BarIndicatorId::WarningRate => IndicatorConfig::new(id, name, vec![])
            .with_param("window_ms", 60000.0),
        BarIndicatorId::WarningFrequencyFilter => IndicatorConfig::new(id, name, vec![])
            .with_param("min_interval_ms", 1000.0),
        BarIndicatorId::L3SpooferScore => IndicatorConfig::new(id, name, vec![20])
            .with_param("large_size_multiplier", 3.0),
        BarIndicatorId::QuoteLifecycleTracker => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::TickerSpreadRatio => IndicatorConfig::new(id, name, vec![]),
        BarIndicatorId::Volume24hZScore => IndicatorConfig::new(id, name, vec![20]),
        BarIndicatorId::FundingOiPressure => IndicatorConfig::new(id, name, vec![]),
        BarIndicatorId::IvHvSpread => IndicatorConfig::new(id, name, vec![]),
        BarIndicatorId::SqueezeProbability => IndicatorConfig::new(id, name, vec![])
            .with_param("window_ms", 60000.0)
            .with_param("max_expected_liq", 1000000.0),
        BarIndicatorId::FundingSentimentAlignment => IndicatorConfig::new(id, name, vec![]),
        BarIndicatorId::VolRegimeEntry => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::BlockTradeVolumeRatio => IndicatorConfig::new(id, name, vec![])
            .with_param("window_ms", 60000.0),
        BarIndicatorId::CapitulationDetector => IndicatorConfig::new(id, name, vec![20])
            .with_param("volume_spike_threshold", 3.0)
            .with_param("window_ms", 60000.0),
        BarIndicatorId::IndexTrackingError => IndicatorConfig::new(id, name, vec![20]),
        BarIndicatorId::MarketStressComposite => IndicatorConfig::new(id, name, vec![20])
            .with_param("window_ms", 60000.0)
            .with_param("max_expected_liq", 1000000.0)
            .with_param("depletion_slope_threshold", 0.01),
        BarIndicatorId::RiskOffDetector => IndicatorConfig::new(id, name, vec![20])
            .with_param("window_ms", 60000.0)
            .with_param("vol_threshold", 2.0)
            .with_param("funding_threshold", 0.01),
        BarIndicatorId::SentimentComposite => IndicatorConfig::new(id, name, vec![])
            .with_param("window_ms", 60000.0),
        BarIndicatorId::CompoundSqueezeProbability => IndicatorConfig::new(id, name, vec![])
            .with_param("window_ms", 60000.0)
            .with_param("max_expected_liq", 1000000.0),
        BarIndicatorId::TpoSessionBalance => IndicatorConfig::new(id, name, vec![])
            .with_param("window_ms", 28800000.0)
            .with_param("price_bucket", 0.01),
        BarIndicatorId::CompositeWeightDrift => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::AdaptiveWindowSelector => IndicatorConfig::new(id, name, vec![20, 5])
            .with_param("volatility_threshold", 0.01),
        BarIndicatorId::AdaptiveThreshold => IndicatorConfig::new(id, name, vec![20])
            .with_param("multiplier", 2.0),

        // ---- signal processing continued ----
        BarIndicatorId::MacdHistZ => IndicatorConfig::new(id, name, vec![12, 26, 9, 50])
            .with_named_ma_type("ma_type", MovingAverageType::EMA),
        BarIndicatorId::PriceZscore => IndicatorConfig::new(id, name, vec![50])
            .with_named_ma_type("ma_type", MovingAverageType::EMA),
        BarIndicatorId::Zmad => IndicatorConfig::new(id, name, vec![20]),
        BarIndicatorId::MomZscore => IndicatorConfig::new(id, name, vec![14, 50]),
        BarIndicatorId::Ewmac => IndicatorConfig::new(id, name, vec![8, 32]),
        BarIndicatorId::Roof => IndicatorConfig::new(id, name, vec![])
            .with_param("hp_alpha", 0.07)
            .with_param("lp_alpha", 0.07),
        BarIndicatorId::Decyc => IndicatorConfig::new(id, name, vec![])
            .with_param("period", 20.0)
            .with_param("alpha", 0.07),
        BarIndicatorId::Eit => IndicatorConfig::new(id, name, vec![])
            .with_param("alpha", 0.07)
            .with_source(OhlcvField::High),
        BarIndicatorId::Shannon => IndicatorConfig::new(id, name, vec![50])
            .with_param("bins", 10.0)
            .with_source(OhlcvField::High),
        BarIndicatorId::Apen => IndicatorConfig::new(id, name, vec![2])
            .with_param("m", 2.0)
            .with_param("r", 0.2)
            .with_source(OhlcvField::High),
        BarIndicatorId::Pe => IndicatorConfig::new(id, name, vec![5])
            .with_param("m", 3.0)
            .with_param("r", 0.0)
            .with_source(OhlcvField::High),
        BarIndicatorId::FractalDim => IndicatorConfig::new(id, name, vec![30, 5])
            .with_source(OhlcvField::High),
        BarIndicatorId::Hurst => IndicatorConfig::new(id, name, vec![100]),
        BarIndicatorId::HurstPct => IndicatorConfig::new(id, name, vec![100]),
        BarIndicatorId::ChaosOsc => IndicatorConfig::new(id, name, vec![14])
            .with_param("complexity_weight", 0.33)
            .with_param("persistence_weight", 0.33)
            .with_param("volatility_weight", 0.34),
        BarIndicatorId::Fractals => IndicatorConfig::new(id, name, vec![]),
        BarIndicatorId::Dfa => IndicatorConfig::new(id, name, vec![10, 20, 40, 80]),
        BarIndicatorId::DfaPct => IndicatorConfig::new(id, name, vec![10, 20, 40, 80, 100]),
        BarIndicatorId::RsiZscore => IndicatorConfig::new(id, name, vec![14, 50]),
        BarIndicatorId::TradeFlowImbalance => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::UptickDowntickVolume => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::AggressorImbalance => IndicatorConfig::new(id, name, vec![14]),
        BarIndicatorId::LargeTradeFilter => IndicatorConfig::new(id, name, vec![20])
            .with_param("multiplier", 3.0),
        BarIndicatorId::VwapDeviation => IndicatorConfig::new(id, name, vec![])
            .with_param("window_ms", 3600000.0),
        BarIndicatorId::TradeRunDetector => IndicatorConfig::new(id, name, vec![])
            .with_param("min_run_length", 3.0),
        BarIndicatorId::SizeWeightedDirectionalMomentum => IndicatorConfig::new(id, name, vec![])
            .with_param("window_ms", 60000.0),
        BarIndicatorId::TickFrequencyAnomaly => IndicatorConfig::new(id, name, vec![])
            .with_param("short_window_ms", 1000.0)
            .with_param("long_window_ms", 60000.0),
        BarIndicatorId::AggressorBurstDetector => IndicatorConfig::new(id, name, vec![])
            .with_param("burst_window_ms", 1000.0)
            .with_param("min_count", 5.0)
            .with_param("directional_threshold", 0.7),
        BarIndicatorId::LargeTickMomentum => IndicatorConfig::new(id, name, vec![])
            .with_param("size_threshold", 10.0)
            .with_param("window_ms", 60000.0),
        BarIndicatorId::ValueAreaTracker => IndicatorConfig::new(id, name, vec![])
            .with_param("window_ms", 28800000.0)
            .with_param("price_bucket", 0.01)
            .with_param("value_area_pct", 0.7),
        BarIndicatorId::VolumeImbalanceZone => IndicatorConfig::new(id, name, vec![])
            .with_param("window_ms", 3600000.0)
            .with_param("delta_threshold", 0.3),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bar_indicators::bar_indicator_id::BarIndicatorId;
    use crate::bar_indicators::instance_factory::IndicatorConfig;

    // ---- Piece 1 unit tests ----

    #[test]
    fn leaf_sma_granularity() {
        let cfg = IndicatorConfig::new(BarIndicatorId::Sma, "sma".into(), vec![14])
            .with_source(OhlcvField::High);
        let spec = config_granularity(&cfg);
        assert_eq!(spec.period_axes, 1);
        assert_eq!(spec.ma_type_axes, 0);
        assert_eq!(spec.scalar_axes, 0);
        assert_eq!(spec.flag_axes, 0);
        assert_eq!(spec.enum_axes, 0);
        assert!(spec.source_axis);
        assert_eq!(spec.inner_count, 0);
        assert_eq!(spec.max_depth, 0);
        assert_eq!(spec.total_axes(), 2); // 1 period + 1 source
        assert_eq!(spec.type_choosable_axes(), 0);
    }

    #[test]
    fn macd_granularity() {
        let cfg = IndicatorConfig::new(BarIndicatorId::Macd, "macd".into(), vec![12, 26, 9])
            .with_named_ma_type("fast_ma_type", MovingAverageType::EMA)
            .with_named_ma_type("slow_ma_type", MovingAverageType::EMA)
            .with_named_ma_type("signal_ma_type", MovingAverageType::EMA)
            .with_source(OhlcvField::High);
        let spec = config_granularity(&cfg);
        assert_eq!(spec.period_axes, 3);
        assert_eq!(spec.ma_type_axes, 3);
        assert!(spec.source_axis);
        assert_eq!(spec.max_depth, 0);
        assert_eq!(spec.total_axes(), 7); // 3p + 3ma + 1src
        assert_eq!(spec.type_choosable_axes(), 3);
    }

    #[test]
    fn nested_two_levels_granularity() {
        // Build manually: outer has 2 inners, one of which has 1 inner
        let leaf = IndicatorConfig::new(BarIndicatorId::Sma, "sma".into(), vec![5]);
        let mid = IndicatorConfig::new(BarIndicatorId::Ema, "ema".into(), vec![9])
            .with_inner(leaf);
        let outer = IndicatorConfig::new(BarIndicatorId::Rsi, "rsi".into(), vec![14])
            .with_source(OhlcvField::High)
            .with_inner(mid)
            .with_inner(IndicatorConfig::new(BarIndicatorId::Wma, "wma".into(), vec![3]));

        let spec = config_granularity(&outer);
        assert_eq!(spec.max_depth, 2);
        assert_eq!(spec.inner_count, 2);
        // outer: 1p + 1src = 2; mid: 1p = 1; leaf: 1p = 1; extra_inner: 1p = 1 => total = 5
        assert_eq!(spec.total_axes(), 5);
    }

    #[test]
    fn all_ids_have_canonical_config_no_panic() {
        let all: Vec<BarIndicatorId> = BarIndicatorId::all().collect();
        let total = all.len();
        let mut fallback_count = 0usize;

        for id in all {
            let cfg = canonical_max_config(id);
            // Must not panic, and granularity must return
            let spec = config_granularity(&cfg);
            // total_axes must be finite (always true for usize but the call must not panic)
            let _ = spec.total_axes();

            // Detect fallback: name will be `{id:?}` format, periods empty,
            // no params — but we detect it more reliably by checking id matches
            // (fallback always has correct id since we pass it through)
            // Instead, detect by zero local_axes AND empty name:
            // Actually, our match is exhaustive above — no `_ =>` fallback needed.
            // This counter will stay 0 if all arms are present.
            if cfg.periods.is_empty()
                && cfg.additional_params.is_empty()
                && cfg.ma_types.is_empty()
                && cfg.flags.is_empty()
                && cfg.enum_params.is_empty()
                && cfg.source == OhlcvField::Close
            {
                fallback_count += 1;
            }
        }

        // Report — the test always passes; the user sees the numbers via
        // `cargo test -- --nocapture`
        eprintln!(
            "canonical_max_config coverage: {total} ids total, \
             {fallback_count} appear fully zero-config (leaf/zero-axis ids)"
        );
    }
}
