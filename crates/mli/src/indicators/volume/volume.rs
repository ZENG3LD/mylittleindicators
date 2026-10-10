//! Raw per-bar volume, drawn as a histogram on the price pane.

use crate::contract::{
    Color, Cost, Family, HistogramStyle, Indicator, Output, Render, RenderOutput, RenderSpec,
    SourceAxis, UpdateComplexity,
};
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::stream_kind::StreamKind;

/// Passthrough of one bar's volume.
#[derive(Debug, Clone, Copy, Default)]
pub struct Volume {
    current: f64,
    ready: bool,
}

impl Volume {
    /// Create a new `Volume` indicator.
    pub fn new() -> Self {
        Self::default()
    }

    /// Store `volume` as the current output.
    pub fn feed(&mut self, volume: f64) {
        self.current = volume;
        self.ready = true;
    }

    /// Last volume, without advancing state.
    pub fn value(&self) -> f64 {
        self.current
    }

    /// `true` after at least one bar has been fed.
    pub fn is_ready(&self) -> bool {
        self.ready
    }

    /// Clears the latched volume.
    pub fn reset(&mut self) {
        *self = Self::new();
    }
}

/// Unit config — raw volume has no parameters.
#[derive(Debug, Clone, Copy, PartialEq, mli_contract_macros::ConfigAxes)]
pub struct VolumeConfig;

impl Indicator for Volume {
    const ID: IndicatorId = IndicatorId::Volume;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[OhlcvField::Volume]));
    const NEEDS_VOLUME: bool = true;
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    const OUTPUTS: &'static [Output] = &[Output::magnitude(IndicatorOutputId::Volume)];
    type Config = VolumeConfig;
    type Runtime = Volume;

    fn create(_cfg: VolumeConfig) -> Volume {
        Volume::new()
    }
}

impl crate::contract::Config for VolumeConfig {
    fn defaults() -> Self {
        VolumeConfig
    }
    fn machine_defaults() -> Self {
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}

impl crate::contract::GpuCube for Volume {}

impl Render for Volume {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .output(RenderOutput::histogram(
                IndicatorOutputId::Volume,
                "Volume",
                Color::hex(0x26A69A),
            ))
            .histogram_style(HistogramStyle::FromBottom)
            .precision(0)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::MarketSample;
    use crate::engine::contract_engine::IndicatorOrder;

    #[test]
    fn factory_passes_bar_volume_through() {
        // `IndicatorOrder` is the whole-universe enum. It does not fit the
        // default Windows test-thread stack.
        std::thread::Builder::new()
            .stack_size(16 * 1024 * 1024)
            .spawn(|| {
                let mut f = IndicatorOrder::Volume(
                    <<Volume as Indicator>::Config as crate::contract::Config>::defaults(),
                )
                .build_solo()
                .unwrap();
                f.feed(
                    0,
                    MarketSample::Bar {
                        open: 100.0,
                        high: 101.0,
                        low: 99.0,
                        close: 100.5,
                        volume: 12345.0,
                    },
                );
                let v = f.read(IndicatorOutputId::Volume);
                assert!((v - 12345.0).abs() < 1e-9, "expected 12345 volume, got {v}");
            })
            .expect("spawn")
            .join()
            .unwrap_or_else(|payload| std::panic::resume_unwind(payload));
    }
}
