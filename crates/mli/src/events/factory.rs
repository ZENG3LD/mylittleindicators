//! 0.1.8 event factory over [`ContractFactory`].
//!
//! The detectors live in the contract. This module is the old
//! `EventId` / `EventInstance::create` door. It does not own a second
//! set of cores. Four master ids were never absorbed; `create` returns
//! [`EventError::NotInContract`] for those.

use crate::contract::MarketSample;
use crate::engine::contract_engine::{
    output_ids_of, ContractFactory, FillError, IndicatorOrder,
};
use crate::engine::indicator_id::IndicatorId;

const WIDE_STACK: usize = 16 * 1024 * 1024;

/// Identifiers of the public event door.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventId {
    BosEventDetector,
    CandlePattern,
    Confluence,
    CrossAssetBeta,
    DirectionDetector,
    Divergence,
    FvgEventDetector,
    LineCross,
    OscillatorWithDivergence,
    OscillatorWithVolumeWeight,
    PairsCointegrationProxy,
    Pivot,
    PriceLineCross,
    RegimeGate,
    RelativePosition,
    RelativeStrengthCross,
    StatisticalWickDetector,
    SwingDetection,
    Threshold,
    VolatilityRegimeDetector,
    VolumeEventDetector,
}

impl EventId {
    pub fn as_str(self) -> &'static str {
        match self {
            EventId::BosEventDetector => "BosEventDetector",
            EventId::CandlePattern => "CandlePattern",
            EventId::Confluence => "Confluence",
            EventId::CrossAssetBeta => "CrossAssetBeta",
            EventId::DirectionDetector => "DirectionDetector",
            EventId::Divergence => "Divergence",
            EventId::FvgEventDetector => "FvgEventDetector",
            EventId::LineCross => "LineCross",
            EventId::OscillatorWithDivergence => "OscillatorWithDivergence",
            EventId::OscillatorWithVolumeWeight => "OscillatorWithVolumeWeight",
            EventId::PairsCointegrationProxy => "PairsCointegrationProxy",
            EventId::Pivot => "Pivot",
            EventId::PriceLineCross => "PriceLineCross",
            EventId::RegimeGate => "RegimeGate",
            EventId::RelativePosition => "RelativePosition",
            EventId::RelativeStrengthCross => "RelativeStrengthCross",
            EventId::StatisticalWickDetector => "StatisticalWickDetector",
            EventId::SwingDetection => "SwingDetection",
            EventId::Threshold => "Threshold",
            EventId::VolatilityRegimeDetector => "VolatilityRegimeDetector",
            EventId::VolumeEventDetector => "VolumeEventDetector",
        }
    }

    /// Contract member this id builds. `None` when the master list still
    /// names a detector the contract does not have.
    pub fn indicator_id(self) -> Option<IndicatorId> {
        Some(match self {
            EventId::BosEventDetector => IndicatorId::Bos,
            EventId::CandlePattern => IndicatorId::CandlePattern,
            EventId::Confluence => IndicatorId::Confluence,
            EventId::DirectionDetector => IndicatorId::DirDetect,
            EventId::Divergence => IndicatorId::Divergence,
            EventId::FvgEventDetector => IndicatorId::Fvg,
            EventId::LineCross => IndicatorId::LineCross,
            EventId::OscillatorWithVolumeWeight => IndicatorId::OscVolWeight,
            EventId::Pivot => IndicatorId::NbarPivot,
            EventId::PriceLineCross => IndicatorId::PriceLineCross,
            EventId::RegimeGate => IndicatorId::RegimeGate,
            EventId::RelativePosition => IndicatorId::RelPosition,
            EventId::StatisticalWickDetector => IndicatorId::Wickspike,
            EventId::SwingDetection => IndicatorId::SwingDetect,
            EventId::Threshold => IndicatorId::ThreshEdge,
            EventId::VolatilityRegimeDetector => IndicatorId::VolRegimeDetect,
            EventId::VolumeEventDetector => IndicatorId::VolEvent,
            EventId::CrossAssetBeta
            | EventId::OscillatorWithDivergence
            | EventId::PairsCointegrationProxy
            | EventId::RelativeStrengthCross => return None,
        })
    }
}

#[derive(Debug)]
pub enum EventError {
    NotInContract(EventId),
    Fill(FillError),
}

/// One absorbed detector, built from contract defaults.
pub struct EventInstance {
    id: EventId,
    indicator: IndicatorId,
    factory: Box<ContractFactory>,
}

impl EventInstance {
    /// Build the contract member for `id` on a wide stack and return it boxed.
    pub fn create(id: EventId) -> Result<Self, EventError> {
        let indicator = id.indicator_id().ok_or(EventError::NotInContract(id))?;
        let factory = std::thread::Builder::new()
            .stack_size(WIDE_STACK)
            .spawn(move || {
                IndicatorOrder::from_defaults(indicator)
                    .expect("event id maps to a contract member")
                    .build_solo()
                    .map(|factory| Box::new(factory))
            })
            .expect("wide stack")
            .join()
            .expect("event build")
            .map_err(EventError::Fill)?;
        Ok(Self {
            id,
            indicator,
            factory,
        })
    }

    pub fn id(&self) -> EventId {
        self.id
    }

    pub fn indicator_id(&self) -> IndicatorId {
        self.indicator
    }

    pub fn feed(&mut self, ts: i64, sample: MarketSample) {
        self.factory.feed(ts, sample);
    }

    pub fn read(&self) -> f64 {
        match output_ids_of(self.indicator).first().copied() {
            Some(output) => self.factory.read(output),
            None => 0.0,
        }
    }

    pub fn is_ready(&self) -> bool {
        self.factory.is_ready()
    }

    pub fn reset(&mut self) {
        self.factory.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direction_detector_builds_from_the_contract() {
        let ev = EventInstance::create(EventId::DirectionDetector).expect("dir detect");
        assert_eq!(ev.id(), EventId::DirectionDetector);
        assert_eq!(ev.indicator_id(), IndicatorId::DirDetect);
        assert!(!ev.is_ready());
    }

    #[test]
    fn unabsorbed_ids_stay_out() {
        for id in [
            EventId::CrossAssetBeta,
            EventId::OscillatorWithDivergence,
            EventId::PairsCointegrationProxy,
            EventId::RelativeStrengthCross,
        ] {
            match EventInstance::create(id) {
                Err(EventError::NotInContract(got)) => assert_eq!(got, id),
                Ok(_) => panic!("{} must not build a second core", id.as_str()),
                Err(EventError::Fill(err)) => panic!("{} filled: {}", id.as_str(), err.reason),
            }
        }
    }
}
