//! Live indicator validator — drives all 571 BarIndicatorId variants against
//! real exchange data from digdigdig3-station and produces a pass/fail matrix.
//!
//! Stream routing is driven by `IndicatorSignature.input_stream` from the
//! master catalog.  Indicators without a catalog entry default to `StreamKind::Bar`.
//!
//! Usage:
//!   mli-collector-indicator-validator [--duration-secs N]
//!   Default duration = 300 s.

use std::{
    collections::HashMap,
    panic::{self, AssertUnwindSafe},
    time::{Duration, Instant},
};

use anyhow::Result;
use digdigdig3_station::{PersistenceConfig, Station, Stream, SubscriptionSet};
use digdigdig3::core::types::{AccountType, ExchangeId};
use digdigdig3::core::websocket::KlineInterval;
use mylittleindicators::bar_indicators::{
    bar_indicator_id::BarIndicatorId,
    indicator_value::IndicatorValue,
    instance_factory::{IndicatorConfig, IndicatorInstance},
};
use mylittleindicators::catalog::MasterIndicatorCatalog;
use mylittleindicators::core::types::{
    AggTrade, FundingRate, Liquidation, MarkPrice, OpenInterest, OrderBook,
    OrderBookLevel, Ticker, Tick, TradeSide,
};
use mylittleindicators::data_loader::stream_kind::StreamKind;
use serde::Serialize;
use tokio::time::timeout;

// ─────────────────────────────────────────────────────────────────────────────
// All BarIndicatorId variants — compile-time exhaustive list
// ─────────────────────────────────────────────────────────────────────────────

fn all_ids() -> Vec<BarIndicatorId> {
    use BarIndicatorId::*;
    vec![
        // Average
        Alma, Ama, AvFrama, AvVidya, Dema, Ehlersfa, Ehlersz, Ema, Framaadv, Hma, Jma, Lr,
        Mcginley, Rma, Sma, T3, Tema, Tma, Trima, Vwap, Vwma, Wma,
        // Momentum
        AdaptiveStoch, Adx, DiPlusMinus, Amat, Apo, Aroon, AroonDown, AroonOsc, AroonUp, AtrRsi,
        BbPeriod, Bias, Bop, Cci, Cfo, Cmo, Cog, ConnorsRsi, Coppock, Demarker, Dm, Dpo, DpoPct,
        Dsp, Dss, EhlersCc, EhlersRocket, ElderImpulse, ElderRay, EmaSlope, Ewmac, EwmacRobust,
        Gapo, Gator, Highest, IftRsi, Imi, Kdj, Kst, Kvo, Lowest, Macd, MacdHist, MacdHistZ,
        MacdSignal, MarketCipher, MoFisher, MoObv, MomZscore, MtfMomDiv, NeuralMom, Pfe, Pmo,
        Ppo, PpoSignal, Pressure, Psar, Psl, Qqe, Qstick, Rmi, Roc, RocPct, Rsi, RsiPctBands,
        RsiPctRank, RsiZscore, Rsioma, Rsx, Rvgi, Rwi, Smi, Stc, Stoch, StochRsi, Stochkd,
        SweepRev, SwingAge, Tdi, Trix, Tsi, Uo, UoSmooth, Vhf, VhfMa, Vortex, Vwrsi,
        WilliamsR, Zigzag,
        // Signal Processing
        Autocorr, Butter, Cheby, Cusum, Cyber, Decyc, Esine, Ess, Fft, Hampel, Hdc, Hilb, Hmom,
        Hyst, Logicand, Logicor, Logicxor, Logicsign, Lz, Mrf, Rc, Roof, Sbp, Sbprhl, Sbwf, Scf,
        Screst, Screstp, Sent, Sentent, Sentr, Ser, Sflat, Sflatp, Sflux, Sg, Shmpr, Slmpr,
        Sroll, Sroll95, Srollp, Srollrp, Sslope, Sslopep, Ssloperp, Sslopez, Stft, Tenc, Thresh,
        Wave, Wcomp, Zmad,
        // Channels
        Adaptivebb, Adaptivechan, Atrchan, Bb, Bbmetrics, Darvas, Dc, Dcmetrics, Dcpos, Dcwidth,
        Dpobands, Envbw, Envelope, Ichimoku, Ichimokupos, Ichimokuthick, Kc, Kcmetrics, Keltbw,
        Keltdist, Keltpos, Medchan, Medchanpos, Pchosc, Pchwidth, Percentb, Percentilech,
        Pivotchan, Pricechan, Projbands, Qrchan, Regchan, Regchanwidth, Starc, Stddevchan,
        Stddevwidth, Theilsenchan, Trimabands, Volprofchan, Vwapchan, Vwapchanwidth,
        // Volatility
        Abb, Atr, Atrbw, Atrc, Atrp, Atrpt, Atrz, Avr, Bpv, C2cvp, Chop, Cv, Dvr, Fuzzy, Har,
        Hvc2c, Kp, Natr, Nr, Pgry, Rbvj, Rcb, Rp, Rq, Rv, Rvi, Rvz, Sqmom, Tr, Ui, Vbd, Vbexp,
        VoDc, VoKc, VoMi, VoVr, Vov, Vovp, Vovpt, Vprb, Wvf,
        // Levels
        Avwap, Avwaprev, Avwaptouch, Bos, Camarilla, Demark, Floorpivot, Fvg, Fvgalt, Fvgdur,
        Fvgrev, Hlva, Liqgap, Pivavwap, Pivot, Rmid, Rquart, Swingstr, Woodie,
        // Candles
        Candleanatomy, Heikinashi, Wickspike,
        // Volume
        Cvd, Mfi, NviPvi, Poc, Pvo, Pvt, Pzo, Rvol, Rvp, SessionVwap, Vdelta, Vfi, Vo, Vpin,
        Vprofile, Vpt, Vroc, Vz, Vzo, TradeFlowImbalance, UptickDowntickVolume, AggressorImbalance,
        LargeTradeFilter,
        // Trend
        AdxSlope, Didi, Eit, GannHilo, Gmma, HaTrend, KamaSlope, LrSlope, Ravi, Sdl, Ssl,
        Supertrend, Tii, TrEr, Zlsma,
        // Accumulation
        Ad, Asi, Cho, Cmf, Di, Eom, Fi, Ii, Iip, Iir, Obv, Tmf, Wad,
        // Adaptive
        Adaptivema, Frama, Kama, Mama, Vidya,
        // Entropy
        Apen, Conden, Fisher, Infog, Jsd, Kld, Mi, Pe, Sampen, Shannon, Te, Xmil,
        // Kalman
        Abgfilter, Ekf, Kalman, Kcomp, Kregime, Kscr, Kslope, Kslopez, Particle, Rts, Ukf,
        // Trend Stop
        Atrts, Chand, Cks, Donbo, Dons, Kelts, Psars, Supts, TsSwings, Volts, VoltsAtr,
        // Chaos
        Ac, Alligator, Ao, ChaosOsc, Dfa, DfaPct, FractalDim, Fractals, Hurst, HurstPct,
        WilliamsMfi,
        // Divergence
        CciDiv, HiddenDiv, MacdDiv, MacdHistDiv, ObvDiv, RsiDiv, StochDiv, VolDiv, WilliamsDiv,
        ZigzagDiv,
        // Regression
        Arima, Arimax, Egarch, Garch, PolyReg, Var,
        // Ratio
        Er, ErRing, RangeAtr, SpreadAnalyzer,
        // Tick Advanced
        VwapDeviation, TradeRunDetector, SizeWeightedDirectionalMomentum, TickFrequencyAnomaly,
        AggressorBurstDetector, LargeTickMomentum, ValueAreaTracker, VolumeImbalanceZone,
        // Clusters
        ClQueueImb, MarketMicro, OrderBookSlope, OrderFlowImb, TickVolume, VwapLevels,
        FootprintChart, FootprintImbalance, FootprintPoc, AbsorptionDetector, TradeClusterDetector,
        // Position
        AvwapDist, Cpr, DayWeekMonth, DistLevels, DomWoq, HolidayProx, HourDay, MonthQtr,
        MonthTurn, QtrTurn, RelTrendPos, Session, SomEom, SoqEoq, SowEow, VwapDist, WeekMonth,
        Weekday,
        // Statistics
        Adf, AdfKpss, ArchLm, ArchLmPval, BpCusum, Coint, EgAdf, EgCoint, EgTrend, HalfLifeMr,
        Kpss, KpssTrend, KpssZ, LjungBox, Pacf, Pp, PriceZscore, PvCoherence, RSquared,
        ResidStat, StCusum, Vr, VrAgg, VrZAgg, Za,
        // Book
        BookImb, BookMicroprice, BookSlope, Ofi, QueueImb, LiquiditySweep, BookPressure,
        SpreadDistribution, OrderBookVelocity, WallDetector, BookDepthChange,
        // Book delta
        IcebergDetector, LevelReplenishRate, BookChurnRate,
        // Hybrid Tick+Book
        HiddenLiquidityDetector, TradeBookAbsorption, SweepImpactAnalyzer,
        // Book Advanced
        BidAskAsymmetry, BidAskBounceRate, MidPriceVelocity, BestLevelVolatility,
        LayerConcentration, PriceLevelDensity,
        // Funding / OI
        FundingMomentum, FundingZScore, OiChangeRate, FundingPriceDivergence,
        // Open Interest
        OiZScore, OiMomentum, OiPercentile, LongSqueezeDetector, OiPriceCorrelation,
        // MarkPrice
        MarkPriceVsLast, IndexPriceMomentum,
        // MarkPrice advanced
        MarkPriceMomentum, MarkPriceVolatility, MarkPriceGapDetector,
        // Ticker
        Volume24hMomentum, HighLowRangeRatio, PriceChange24hZScore,
        // Liquidation
        LiquidationRate, LiquidationVolumeImbalance, LiquidationCascade,
        LiquidationVolumeVelocity, StopHuntDetector, LiquidationClusterDetector, LiquidationCooldown,
        // Sentiment
        LongShortRatioMomentum, LongShortExtremeDetector, RatioVsPriceDivergence,
        AggTradeFlowImbalance, AggTradeSizeDistribution,
        // Index/Basis
        PriceVsIndexSpread, IndexComponentDrift, IndexCorrelationBreakdown, BasisMomentum,
        BasisExtreme, BasisZScore,
        // Volatility advanced
        HvMomentum, HvSpike, VolIdxSpike, VolIdxMomentum,
        // Greeks
        DeltaExposureFlow, GammaSqueezeDetector, IvSkew,
        // Greeks advanced
        CharmTracker, VegaExposureFlow, ThetaDecayTracker, PinRiskDetector,
        // Stress
        FundDepletionRate, FundStressDetector, InsuranceFundMomentum, SettlementApproachSignal,
        SettlementPriceMomentum, SettlementVsMarkSpread,
        // Microstructure
        BlockTradeFlow, BlockTradeImpact, L3OrderRate, L3LargeOrderTracker, L3CancelRatio,
        BlockTradeSizeAnomaly, QuoteStuffingDetector, L3SpooferScore, QuoteLifecycleTracker,
        // Risk
        LeverageReductionWarning, MmrTracker, RiskLimitProximity,
        // Funding indicators
        FundingDrift, FundingTimeDecay, PredictedFundingExtreme, SettledFundingMomentum,
        FundingSettlementImpact,
        // Funding advanced
        AnnualizedFundingRate, FundingDirectionShift, FundingExtremeAlert,
        // Misc
        AuctionLiquidityScore, AuctionPriceDeviation, AuctionImbalance, WarningRate,
        WarningFrequencyFilter,
        // Ticker advanced
        TickerSpreadRatio, Volume24hZScore,
        // Cross-stream composites
        FundingOiPressure, IvHvSpread, SqueezeProbability, FundingSentimentAlignment,
        VolRegimeEntry, BlockTradeVolumeRatio, CapitulationDetector, IndexTrackingError,
        // Category C composites
        MarketStressComposite, RiskOffDetector, SentimentComposite, CompoundSqueezeProbability,
        TpoSessionBalance, CompositeWeightDrift, AdaptiveWindowSelector, AdaptiveThreshold,
        PairsCointegrationProxy, CrossAssetBeta, RelativeStrengthCross,
    ]
}

// ─────────────────────────────────────────────────────────────────────────────
// Category from id — used for per-category breakdown
// ─────────────────────────────────────────────────────────────────────────────

fn category_of(id: BarIndicatorId) -> &'static str {
    use BarIndicatorId::*;
    match id {
        Alma | Ama | AvFrama | AvVidya | Dema | Ehlersfa | Ehlersz | Ema | Framaadv | Hma | Jma
        | Lr | Mcginley | Rma | Sma | T3 | Tema | Tma | Trima | Vwap | Vwma | Wma => "average",

        AdaptiveStoch | Adx | DiPlusMinus | Amat | Apo | Aroon | AroonDown | AroonOsc | AroonUp
        | AtrRsi | BbPeriod | Bias | Bop | Cci | Cfo | Cmo | Cog | ConnorsRsi | Coppock
        | Demarker | Dm | Dpo | DpoPct | Dsp | Dss | EhlersCc | EhlersRocket | ElderImpulse
        | ElderRay | EmaSlope | Ewmac | EwmacRobust | Gapo | Gator | Highest | IftRsi | Imi
        | Kdj | Kst | Kvo | Lowest | Macd | MacdHist | MacdHistZ | MacdSignal | MarketCipher
        | MoFisher | MoObv | MomZscore | MtfMomDiv | NeuralMom | Pfe | Pmo | Ppo | PpoSignal
        | Pressure | Psar | Psl | Qqe | Qstick | Rmi | Roc | RocPct | Rsi | RsiPctBands
        | RsiPctRank | RsiZscore | Rsioma | Rsx | Rvgi | Rwi | Smi | Stc | Stoch | StochRsi
        | Stochkd | SweepRev | SwingAge | Tdi | Trix | Tsi | Uo | UoSmooth | Vhf | VhfMa
        | Vortex | Vwrsi | WilliamsR | Zigzag => "momentum",

        Autocorr | Butter | Cheby | Cusum | Cyber | Decyc | Esine | Ess | Fft | Hampel | Hdc
        | Hilb | Hmom | Hyst | Logicand | Logicor | Logicxor | Logicsign | Lz | Mrf | Rc | Roof
        | Sbp | Sbprhl | Sbwf | Scf | Screst | Screstp | Sent | Sentent | Sentr | Ser | Sflat
        | Sflatp | Sflux | Sg | Shmpr | Slmpr | Sroll | Sroll95 | Srollp | Srollrp | Sslope
        | Sslopep | Ssloperp | Sslopez | Stft | Tenc | Thresh | Wave | Wcomp | Zmad => {
            "signal_processing"
        }

        Adaptivebb | Adaptivechan | Atrchan | Bb | Bbmetrics | Darvas | Dc | Dcmetrics | Dcpos
        | Dcwidth | Dpobands | Envbw | Envelope | Ichimoku | Ichimokupos | Ichimokuthick | Kc
        | Kcmetrics | Keltbw | Keltdist | Keltpos | Medchan | Medchanpos | Pchosc | Pchwidth
        | Percentb | Percentilech | Pivotchan | Pricechan | Projbands | Qrchan | Regchan
        | Regchanwidth | Starc | Stddevchan | Stddevwidth | Theilsenchan | Trimabands
        | Volprofchan | Vwapchan | Vwapchanwidth => "channels",

        Abb | Atr | Atrbw | Atrc | Atrp | Atrpt | Atrz | Avr | Bpv | C2cvp | Chop | Cv | Dvr
        | Fuzzy | Har | Hvc2c | Kp | Natr | Nr | Pgry | Rbvj | Rcb | Rp | Rq | Rv | Rvi | Rvz
        | Sqmom | Tr | Ui | Vbd | Vbexp | VoDc | VoKc | VoMi | VoVr | Vov | Vovp | Vovpt
        | Vprb | Wvf => "volatility",

        Avwap | Avwaprev | Avwaptouch | Bos | Camarilla | Demark | Floorpivot | Fvg | Fvgalt
        | Fvgdur | Fvgrev | Hlva | Liqgap | Pivavwap | Pivot | Rmid | Rquart | Swingstr
        | Woodie => "levels",

        Candleanatomy | Heikinashi | Wickspike => "candles",

        Cvd | Mfi | NviPvi | Poc | Pvo | Pvt | Pzo | Rvol | Rvp | SessionVwap | Vdelta | Vfi
        | Vo | Vpin | Vprofile | Vpt | Vroc | Vz | Vzo | TradeFlowImbalance
        | UptickDowntickVolume | AggressorImbalance | LargeTradeFilter => "volume",

        AdxSlope | Didi | Eit | GannHilo | Gmma | HaTrend | KamaSlope | LrSlope | Ravi | Sdl
        | Ssl | Supertrend | Tii | TrEr | Zlsma => "trend",

        Ad | Asi | Cho | Cmf | Di | Eom | Fi | Ii | Iip | Iir | Obv | Tmf | Wad => {
            "accumulation"
        }

        Adaptivema | Frama | Kama | Mama | Vidya => "adaptive",

        Apen | Conden | Fisher | Infog | Jsd | Kld | Mi | Pe | Sampen | Shannon | Te | Xmil => {
            "entropy"
        }

        Abgfilter | Ekf | Kalman | Kcomp | Kregime | Kscr | Kslope | Kslopez | Particle | Rts
        | Ukf => "kalman",

        Atrts | Chand | Cks | Donbo | Dons | Kelts | Psars | Supts | TsSwings | Volts
        | VoltsAtr => "trend_stop",

        Ac | Alligator | Ao | ChaosOsc | Dfa | DfaPct | FractalDim | Fractals | Hurst
        | HurstPct | WilliamsMfi => "chaos",

        CciDiv | HiddenDiv | MacdDiv | MacdHistDiv | ObvDiv | RsiDiv | StochDiv | VolDiv
        | WilliamsDiv | ZigzagDiv => "divergence",

        Arima | Arimax | Egarch | Garch | PolyReg | Var => "regression",

        Er | ErRing | RangeAtr | SpreadAnalyzer => "ratio",

        VwapDeviation | TradeRunDetector | SizeWeightedDirectionalMomentum
        | TickFrequencyAnomaly | AggressorBurstDetector | LargeTickMomentum | ValueAreaTracker
        | VolumeImbalanceZone => "tick_advanced",

        ClQueueImb | MarketMicro | OrderBookSlope | OrderFlowImb | TickVolume | VwapLevels
        | FootprintChart | FootprintImbalance | FootprintPoc | AbsorptionDetector
        | TradeClusterDetector => "clusters",

        AvwapDist | Cpr | DayWeekMonth | DistLevels | DomWoq | HolidayProx | HourDay
        | MonthQtr | MonthTurn | QtrTurn | RelTrendPos | Session | SomEom | SoqEoq | SowEow
        | VwapDist | WeekMonth | Weekday => "position",

        Adf | AdfKpss | ArchLm | ArchLmPval | BpCusum | Coint | EgAdf | EgCoint | EgTrend
        | HalfLifeMr | Kpss | KpssTrend | KpssZ | LjungBox | Pacf | Pp | PriceZscore
        | PvCoherence | RSquared | ResidStat | StCusum | Vr | VrAgg | VrZAgg | Za => {
            "statistics"
        }

        BookImb | BookMicroprice | BookSlope | Ofi | QueueImb | LiquiditySweep | BookPressure
        | SpreadDistribution | OrderBookVelocity | WallDetector | BookDepthChange => "book",

        IcebergDetector | LevelReplenishRate | BookChurnRate => "book_delta",

        HiddenLiquidityDetector | TradeBookAbsorption | SweepImpactAnalyzer => "hybrid_tick_book",

        BidAskAsymmetry | BidAskBounceRate | MidPriceVelocity | BestLevelVolatility
        | LayerConcentration | PriceLevelDensity => "book_advanced",

        FundingMomentum | FundingZScore | OiChangeRate | FundingPriceDivergence => "funding_oi",

        OiZScore | OiMomentum | OiPercentile | LongSqueezeDetector | OiPriceCorrelation => {
            "open_interest"
        }

        MarkPriceVsLast | IndexPriceMomentum => "mark_price",

        MarkPriceMomentum | MarkPriceVolatility | MarkPriceGapDetector => "mark_price_advanced",

        Volume24hMomentum | HighLowRangeRatio | PriceChange24hZScore => "ticker",

        LiquidationRate | LiquidationVolumeImbalance | LiquidationCascade
        | LiquidationVolumeVelocity | StopHuntDetector | LiquidationClusterDetector
        | LiquidationCooldown => "liquidations",

        LongShortRatioMomentum | LongShortExtremeDetector | RatioVsPriceDivergence
        | AggTradeFlowImbalance | AggTradeSizeDistribution => "sentiment",

        PriceVsIndexSpread | IndexComponentDrift | IndexCorrelationBreakdown | BasisMomentum
        | BasisExtreme | BasisZScore => "index_basis",

        HvMomentum | HvSpike | VolIdxSpike | VolIdxMomentum => "volatility_advanced",

        DeltaExposureFlow | GammaSqueezeDetector | IvSkew => "greeks",

        CharmTracker | VegaExposureFlow | ThetaDecayTracker | PinRiskDetector => {
            "greeks_advanced"
        }

        FundDepletionRate | FundStressDetector | InsuranceFundMomentum
        | SettlementApproachSignal | SettlementPriceMomentum | SettlementVsMarkSpread => "stress",

        BlockTradeFlow | BlockTradeImpact | L3OrderRate | L3LargeOrderTracker | L3CancelRatio
        | BlockTradeSizeAnomaly | QuoteStuffingDetector | L3SpooferScore
        | QuoteLifecycleTracker => "microstructure",

        LeverageReductionWarning | MmrTracker | RiskLimitProximity => "risk",

        FundingDrift | FundingTimeDecay | PredictedFundingExtreme | SettledFundingMomentum
        | FundingSettlementImpact => "risk_funding",

        AnnualizedFundingRate | FundingDirectionShift | FundingExtremeAlert => "funding_advanced",

        AuctionLiquidityScore | AuctionPriceDeviation | AuctionImbalance | WarningRate
        | WarningFrequencyFilter => "misc",

        TickerSpreadRatio | Volume24hZScore => "ticker_advanced",

        FundingOiPressure | IvHvSpread | SqueezeProbability | FundingSentimentAlignment
        | VolRegimeEntry | BlockTradeVolumeRatio | CapitulationDetector | IndexTrackingError => {
            "composites"
        }

        MarketStressComposite | RiskOffDetector | SentimentComposite
        | CompoundSqueezeProbability | TpoSessionBalance | CompositeWeightDrift
        | AdaptiveWindowSelector | AdaptiveThreshold | PairsCointegrationProxy | CrossAssetBeta
        | RelativeStrengthCross => "composites_c",
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Build stream-kind routing table from master catalog
// ─────────────────────────────────────────────────────────────────────────────

/// Build a `HashMap<BarIndicatorId, StreamKind>` by reading `sig.input_stream`
/// from every signature that has a `machine_id`.  Indicators absent from the
/// catalog get `StreamKind::Bar` as fallback (they are treated as OHLCV until
/// someone catalogues them).
fn build_stream_kind_map(
    all_indicator_ids: &[BarIndicatorId],
) -> (HashMap<BarIndicatorId, StreamKind>, usize, usize) {
    let catalog = MasterIndicatorCatalog::new();

    // Collect catalog-sourced routing.
    let mut map: HashMap<BarIndicatorId, StreamKind> = HashMap::new();
    let mut n_catalogued: usize = 0;

    for sig in catalog.iter_signatures() {
        if let Some(machine_id) = sig.machine_id {
            map.insert(machine_id, sig.input_stream);
            n_catalogued += 1;
        }
    }

    // Fill fallback for any id not in catalog.
    let n_uncatalogued = all_indicator_ids
        .iter()
        .filter(|id| !map.contains_key(id))
        .count();

    for &id in all_indicator_ids {
        map.entry(id).or_insert(StreamKind::Bar);
    }

    (map, n_catalogued, n_uncatalogued)
}

// ─────────────────────────────────────────────────────────────────────────────
// Per-indicator state
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
struct IndicatorRecord {
    id: String,
    category: String,
    stream_kind: String,
    matched_signature: bool,
    status: String,     // "create_failed" | "panic" | "never_received_event" | "never_ready" | "always_zero" | "always_nan_inf" | "pass"
    events_received: u64,
    panic_count: u64,
    is_ready: bool,
    ready_at_event: Option<u64>,
    last_value_repr: String,
    max_abs_value: f64,
    has_finite_nonzero: bool,
    create_error: Option<String>,
}

struct IndicatorState {
    id: BarIndicatorId,
    category: &'static str,
    stream_kind: StreamKind,
    matched_signature: bool,
    instance: Option<IndicatorInstance>,
    create_error: Option<String>,
    events_received: u64,
    panic_count: u64,
    is_ready: bool,
    ready_at_event: Option<u64>,
    last_value: Option<IndicatorValue>,
    max_abs_value: f64,
    has_finite_nonzero: bool,
}

impl IndicatorState {
    fn new(id: BarIndicatorId, stream_kind: StreamKind, matched_signature: bool) -> Self {
        let category = category_of(id);
        let cfg = IndicatorConfig::new(id, format!("{id:?}"), vec![14]);
        let result = panic::catch_unwind(AssertUnwindSafe(|| IndicatorInstance::create(&cfg)));
        let (instance, create_error) = match result {
            Ok(Ok(inst)) => (Some(inst), None),
            Ok(Err(e)) => (None, Some(e)),
            Err(_) => (None, Some("panic during create".to_string())),
        };
        Self {
            id,
            category,
            stream_kind,
            matched_signature,
            instance,
            create_error,
            events_received: 0,
            panic_count: 0,
            is_ready: false,
            ready_at_event: None,
            last_value: None,
            max_abs_value: 0.0,
            has_finite_nonzero: false,
        }
    }

    fn record_value(&mut self, val: IndicatorValue) {
        self.last_value = Some(val);
        let fs = extract_f64s(&val);
        for v in &fs {
            if v.is_finite() && *v != 0.0 {
                self.has_finite_nonzero = true;
            }
            let av = v.abs();
            if av > self.max_abs_value && av.is_finite() {
                self.max_abs_value = av;
            }
        }
    }

    fn try_update_bar(&mut self, o: f64, h: f64, l: f64, c: f64, v: f64, ts: i64) {
        let inst = match &mut self.instance {
            Some(i) => i,
            None => return,
        };
        self.events_received += 1;
        let result = panic::catch_unwind(AssertUnwindSafe(|| {
            inst.update_bar(o, h, l, c, v, Some(ts))
        }));
        match result {
            Ok(val) => {
                if !self.is_ready {
                    let r = panic::catch_unwind(AssertUnwindSafe(|| inst.is_ready()));
                    if matches!(r, Ok(true)) {
                        self.is_ready = true;
                        self.ready_at_event = Some(self.events_received);
                    }
                }
                self.record_value(val);
            }
            Err(_) => {
                self.panic_count += 1;
            }
        }
    }

    fn try_update_orderbook(&mut self, book: &OrderBook) {
        let inst = match &mut self.instance {
            Some(i) => i,
            None => return,
        };
        self.events_received += 1;
        let result = panic::catch_unwind(AssertUnwindSafe(|| inst.update_orderbook(book)));
        match result {
            Ok(val) => {
                if !self.is_ready {
                    let r = panic::catch_unwind(AssertUnwindSafe(|| inst.is_ready()));
                    if matches!(r, Ok(true)) {
                        self.is_ready = true;
                        self.ready_at_event = Some(self.events_received);
                    }
                }
                self.record_value(val);
            }
            Err(_) => { self.panic_count += 1; }
        }
    }

    fn try_update_funding(&mut self, fr: &FundingRate) {
        let inst = match &mut self.instance {
            Some(i) => i,
            None => return,
        };
        self.events_received += 1;
        let result = panic::catch_unwind(AssertUnwindSafe(|| inst.update_funding(fr)));
        match result {
            Ok(val) => {
                if !self.is_ready {
                    let r = panic::catch_unwind(AssertUnwindSafe(|| inst.is_ready()));
                    if matches!(r, Ok(true)) {
                        self.is_ready = true;
                        self.ready_at_event = Some(self.events_received);
                    }
                }
                self.record_value(val);
            }
            Err(_) => { self.panic_count += 1; }
        }
    }

    fn try_update_oi(&mut self, oi: &OpenInterest) {
        let inst = match &mut self.instance {
            Some(i) => i,
            None => return,
        };
        self.events_received += 1;
        let result = panic::catch_unwind(AssertUnwindSafe(|| inst.update_oi(oi)));
        match result {
            Ok(val) => {
                if !self.is_ready {
                    let r = panic::catch_unwind(AssertUnwindSafe(|| inst.is_ready()));
                    if matches!(r, Ok(true)) {
                        self.is_ready = true;
                        self.ready_at_event = Some(self.events_received);
                    }
                }
                self.record_value(val);
            }
            Err(_) => { self.panic_count += 1; }
        }
    }

    fn try_update_mark(&mut self, mp: &MarkPrice) {
        let inst = match &mut self.instance {
            Some(i) => i,
            None => return,
        };
        self.events_received += 1;
        let result = panic::catch_unwind(AssertUnwindSafe(|| inst.update_mark(mp)));
        match result {
            Ok(val) => {
                if !self.is_ready {
                    let r = panic::catch_unwind(AssertUnwindSafe(|| inst.is_ready()));
                    if matches!(r, Ok(true)) {
                        self.is_ready = true;
                        self.ready_at_event = Some(self.events_received);
                    }
                }
                self.record_value(val);
            }
            Err(_) => { self.panic_count += 1; }
        }
    }

    fn try_update_ticker(&mut self, ticker: &Ticker) {
        let inst = match &mut self.instance {
            Some(i) => i,
            None => return,
        };
        self.events_received += 1;
        let result = panic::catch_unwind(AssertUnwindSafe(|| inst.update_ticker(ticker)));
        match result {
            Ok(val) => {
                if !self.is_ready {
                    let r = panic::catch_unwind(AssertUnwindSafe(|| inst.is_ready()));
                    if matches!(r, Ok(true)) {
                        self.is_ready = true;
                        self.ready_at_event = Some(self.events_received);
                    }
                }
                self.record_value(val);
            }
            Err(_) => { self.panic_count += 1; }
        }
    }

    fn try_update_tick(&mut self, tick: &Tick) {
        let inst = match &mut self.instance {
            Some(i) => i,
            None => return,
        };
        self.events_received += 1;
        let result = panic::catch_unwind(AssertUnwindSafe(|| inst.update_tick(tick)));
        match result {
            Ok(val) => {
                if !self.is_ready {
                    let r = panic::catch_unwind(AssertUnwindSafe(|| inst.is_ready()));
                    if matches!(r, Ok(true)) {
                        self.is_ready = true;
                        self.ready_at_event = Some(self.events_received);
                    }
                }
                self.record_value(val);
            }
            Err(_) => { self.panic_count += 1; }
        }
    }

    fn try_update_liquidation(&mut self, liq: &Liquidation) {
        let inst = match &mut self.instance {
            Some(i) => i,
            None => return,
        };
        self.events_received += 1;
        let result = panic::catch_unwind(AssertUnwindSafe(|| inst.update_liquidation(liq)));
        match result {
            Ok(val) => {
                if !self.is_ready {
                    let r = panic::catch_unwind(AssertUnwindSafe(|| inst.is_ready()));
                    if matches!(r, Ok(true)) {
                        self.is_ready = true;
                        self.ready_at_event = Some(self.events_received);
                    }
                }
                self.record_value(val);
            }
            Err(_) => { self.panic_count += 1; }
        }
    }

    fn try_update_agg_trade(&mut self, t: &AggTrade) {
        let inst = match &mut self.instance {
            Some(i) => i,
            None => return,
        };
        self.events_received += 1;
        let result = panic::catch_unwind(AssertUnwindSafe(|| inst.update_agg_trade(t)));
        match result {
            Ok(val) => {
                if !self.is_ready {
                    let r = panic::catch_unwind(AssertUnwindSafe(|| inst.is_ready()));
                    if matches!(r, Ok(true)) {
                        self.is_ready = true;
                        self.ready_at_event = Some(self.events_received);
                    }
                }
                self.record_value(val);
            }
            Err(_) => { self.panic_count += 1; }
        }
    }

    fn finalize_record(&self) -> IndicatorRecord {
        let status = if self.create_error.is_some() {
            "create_failed".to_string()
        } else if self.panic_count > 0 && self.events_received == self.panic_count {
            "panic".to_string()
        } else if self.events_received == 0 {
            "never_received_event".to_string()
        } else if !self.is_ready {
            "never_ready".to_string()
        } else if !self.has_finite_nonzero {
            // Check if all values were NaN/Inf or all zero
            let all_nan = self.last_value.map(|v| {
                let fs = extract_f64s(&v);
                fs.iter().all(|x| !x.is_finite())
            }).unwrap_or(false);
            if all_nan { "always_nan_inf".to_string() } else { "always_zero".to_string() }
        } else {
            "pass".to_string()
        };

        IndicatorRecord {
            id: format!("{:?}", self.id),
            category: self.category.to_string(),
            stream_kind: self.stream_kind.as_str().to_string(),
            matched_signature: self.matched_signature,
            status,
            events_received: self.events_received,
            panic_count: self.panic_count,
            is_ready: self.is_ready,
            ready_at_event: self.ready_at_event,
            last_value_repr: self.last_value.map(|v| format!("{v:?}")).unwrap_or_default(),
            max_abs_value: self.max_abs_value,
            has_finite_nonzero: self.has_finite_nonzero,
            create_error: self.create_error.clone(),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Extract all f64 values from an IndicatorValue
// ─────────────────────────────────────────────────────────────────────────────

fn extract_f64s(val: &IndicatorValue) -> Vec<f64> {
    match val {
        IndicatorValue::Single(v) => vec![*v],
        IndicatorValue::Signal(s) => vec![*s as f64],
        IndicatorValue::Flag(b) => vec![if *b { 1.0 } else { 0.0 }],
        IndicatorValue::Double(a, b) => vec![*a, *b],
        IndicatorValue::Triple(a, b, c) => vec![*a, *b, *c],
        IndicatorValue::Channel3 { upper, middle, lower } => vec![*upper, *middle, *lower],
        IndicatorValue::Macd { line, signal, histogram } => vec![*line, *signal, *histogram],
        IndicatorValue::Ichimoku { tenkan, kijun, senkou_a, senkou_b, chikou } => {
            vec![*tenkan, *kijun, *senkou_a, *senkou_b, *chikou]
        }
        IndicatorValue::Candle { open, high, low, close } => vec![*open, *high, *low, *close],
        IndicatorValue::ChannelExtended { upper, middle, lower, bandwidth, percent_b } => {
            vec![*upper, *middle, *lower, *bandwidth, *percent_b]
        }
        IndicatorValue::Adaptive { value, period, alpha } => vec![*value, *period, *alpha],
        IndicatorValue::StatTest { statistic, p_value, .. } => vec![*statistic, *p_value],
        IndicatorValue::Volatility { total, close_close, high_low } => {
            vec![*total, *close_close, *high_low]
        }
        IndicatorValue::ValueFlag(v, _) => vec![*v],
        IndicatorValue::DoubleFlag(_, _) => vec![],
        IndicatorValue::FuzzyCandle { direction, size, body_size, upper_wick, lower_wick } => {
            vec![
                *direction as f64,
                *size as f64,
                *body_size as f64,
                *upper_wick as f64,
                *lower_wick as f64,
            ]
        }
        IndicatorValue::CandleAnatomy { body, upper_wick, lower_wick, .. } => {
            vec![*body, *upper_wick, *lower_wick]
        }
        IndicatorValue::Hilbert { amplitude, phase, frequency } => {
            vec![*amplitude, *phase, *frequency]
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers: convert station data points → core indicator types
// ─────────────────────────────────────────────────────────────────────────────

fn obs_to_orderbook(p: &digdigdig3_station::data::ObSnapshotPoint) -> OrderBook {
    OrderBook {
        bids: p.bids.iter().map(|(pr, sz)| OrderBookLevel::new(*pr, *sz)).collect(),
        asks: p.asks.iter().map(|(pr, sz)| OrderBookLevel::new(*pr, *sz)).collect(),
        timestamp: p.ts_ms,
        sequence: None,
        last_update_id: None,
        first_update_id: None,
        prev_update_id: None,
        event_time: None,
        transaction_time: None,
        checksum: None,
    }
}

fn funding_point_to_core(
    p: &digdigdig3_station::data::FundingRatePoint,
) -> FundingRate {
    FundingRate {
        rate: p.rate,
        next_funding_time: if p.next_funding_time_ms == 0 { None } else { Some(p.next_funding_time_ms) },
        timestamp: p.ts_ms,
    }
}

fn oi_point_to_core(p: &digdigdig3_station::data::OpenInterestPoint) -> OpenInterest {
    OpenInterest {
        open_interest: p.open_interest,
        open_interest_value: if p.open_interest_value.is_nan() { None } else { Some(p.open_interest_value) },
        timestamp: p.ts_ms,
    }
}

fn mark_point_to_core(p: &digdigdig3_station::data::MarkPricePoint) -> MarkPrice {
    MarkPrice {
        mark_price: p.mark,
        index_price: if p.index.is_nan() { None } else { Some(p.index) },
        funding_rate: None,
        timestamp: p.ts_ms,
    }
}

fn ticker_point_to_core(p: &digdigdig3_station::data::TickerPoint) -> Ticker {
    Ticker {
        last_price: p.last,
        bid_price: if p.bid.is_nan() { None } else { Some(p.bid) },
        ask_price: if p.ask.is_nan() { None } else { Some(p.ask) },
        high_24h: if p.high_24h.is_nan() { None } else { Some(p.high_24h) },
        low_24h: if p.low_24h.is_nan() { None } else { Some(p.low_24h) },
        volume_24h: if p.vol_24h.is_nan() { None } else { Some(p.vol_24h) },
        quote_volume_24h: if p.quote_vol_24h.is_nan() { None } else { Some(p.quote_vol_24h) },
        price_change_24h: None,
        price_change_percent_24h: if p.change_pct_24h.is_nan() { None } else { Some(p.change_pct_24h) },
        timestamp: p.ts_ms,
    }
}

fn trade_point_to_tick(p: &digdigdig3_station::data::TradePoint) -> Tick {
    Tick {
        time: p.ts_ms,
        price: p.price,
        size: p.quantity,
        is_buy: p.side == 0, // 0 = Buy in TradeSide encoding
        bid: None,
        ask: None,
    }
}

fn liq_point_to_core(p: &digdigdig3_station::data::LiquidationPoint) -> Liquidation {
    Liquidation {
        symbol: String::new(),
        side: if p.side == 0 { TradeSide::Buy } else { TradeSide::Sell },
        price: p.price,
        quantity: p.quantity,
        timestamp: p.ts_ms,
        value: if p.value.is_nan() { None } else { Some(p.value) },
    }
}

fn agg_trade_point_to_core(p: &digdigdig3_station::data::AggTradePoint) -> AggTrade {
    AggTrade {
        aggregate_id: p.agg_id as i64,
        price: p.price,
        quantity: p.quantity,
        first_trade_id: 0,
        last_trade_id: 0,
        is_buy: p.side == 0,
        timestamp: p.ts_ms,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Main
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    // Parse --duration-secs N
    let duration_secs = {
        let args: Vec<String> = std::env::args().collect();
        let mut dur = 300u64;
        let mut i = 1usize;
        while i < args.len() {
            if args[i] == "--duration-secs" {
                if let Some(v) = args.get(i + 1) {
                    dur = v.parse().unwrap_or(300);
                }
                i += 2;
            } else {
                i += 1;
            }
        }
        dur
    };

    tracing::info!("Indicator validator starting — duration {}s", duration_secs);

    // ── Build catalog-based routing map ──────────────────────────────────────
    let all_indicator_ids = all_ids();
    let total = all_indicator_ids.len();

    let (stream_kind_map, n_catalogued, n_uncatalogued) =
        build_stream_kind_map(&all_indicator_ids);

    // Per-StreamKind breakdown
    let mut sk_counts: HashMap<String, usize> = HashMap::new();
    for &id in &all_indicator_ids {
        let sk = stream_kind_map[&id];
        *sk_counts.entry(sk.as_str().to_string()).or_default() += 1;
    }

    let n_non_bar: usize = all_indicator_ids
        .iter()
        .filter(|&&id| stream_kind_map[&id] != StreamKind::Bar)
        .count();

    tracing::info!(
        "Catalog: {} signatures with machine_id, {} without (Bar default), {} routed non-Bar",
        n_catalogued, n_uncatalogued, n_non_bar
    );

    // ── Build matched-signature set ──────────────────────────────────────────
    // Second catalog pass: collect the set of BarIndicatorIds that have an
    // explicit catalog entry with machine_id.  Used to populate `matched_signature`
    // per indicator state without a per-indicator catalog construction.
    let catalogued_ids: std::collections::HashSet<BarIndicatorId> = {
        let cat = MasterIndicatorCatalog::new();
        let ids: std::collections::HashSet<BarIndicatorId> = cat
            .iter_signatures()
            .filter_map(|sig| sig.machine_id)
            .collect();
        ids
    };

    // ── Build indicator states ───────────────────────────────────────────────
    tracing::info!("Initializing {} indicators…", total);

    let mut states: Vec<IndicatorState> = all_indicator_ids
        .iter()
        .map(|&id| {
            let sk = stream_kind_map[&id];
            let matched_sig = catalogued_ids.contains(&id);
            IndicatorState::new(id, sk, matched_sig)
        })
        .collect();

    let created_ok = states.iter().filter(|s| s.instance.is_some()).count();
    let create_failed = states.iter().filter(|s| s.create_error.is_some()).count();
    tracing::info!(
        "Created OK: {}  create_failed: {}",
        created_ok, create_failed
    );

    // Build station subscription
    let station = Station::builder()
        .storage_root(std::path::PathBuf::from("./validator_data"))
        .persistence(
            PersistenceConfig::on()
                .trades(false)
                .agg_trades(false)
                .klines(false)
                .tickers(false)
                .orderbook_snapshots(false)
                .mark_price(false)
                .funding_rate(false)
                .open_interest(false)
                .liquidations(false),
        )
        .warm_start(0)
        .build()
        .await?;

    let interval_1m = KlineInterval::new("1m");

    // Per-stream subscribe with best-effort fallback: some (exchange, stream)
    // combos eagerly return NotSupported (e.g. Binance OI WS, BingX OI WS).
    // We try each individually and merge into one handle's stream of events
    // via a single multi-add SubscriptionSet — but if even one fails the
    // whole subscribe() fails. So we do N tiny one-stream subscribes and
    // collect handles.
    let combos: &[(ExchangeId, Stream)] = &[
        (ExchangeId::Binance, Stream::Trade),
        (ExchangeId::Binance, Stream::AggTrade),
        (ExchangeId::Binance, Stream::Kline(interval_1m.clone())),
        (ExchangeId::Binance, Stream::Ticker),
        (ExchangeId::Binance, Stream::Orderbook),
        (ExchangeId::Binance, Stream::MarkPrice),
        (ExchangeId::Binance, Stream::FundingRate),
        (ExchangeId::Binance, Stream::Liquidation),
        // Binance OI: WS NotSupported by exchange, skip.
        (ExchangeId::Bybit, Stream::Trade),
        (ExchangeId::Bybit, Stream::AggTrade),
        (ExchangeId::Bybit, Stream::Kline(interval_1m.clone())),
        (ExchangeId::Bybit, Stream::Ticker),
        (ExchangeId::Bybit, Stream::Orderbook),
        (ExchangeId::Bybit, Stream::MarkPrice),
        (ExchangeId::Bybit, Stream::FundingRate),
        (ExchangeId::Bybit, Stream::Liquidation),
        // Bybit OI: try
        (ExchangeId::Bybit, Stream::OpenInterest),
    ];

    let mut handles: Vec<_> = Vec::new();
    for (exch, stream) in combos {
        let single = SubscriptionSet::new().add(
            *exch,
            "BTCUSDT",
            AccountType::FuturesCross,
            [stream.clone()],
        );
        match station.subscribe(single).await {
            Ok(h) => handles.push(h),
            Err(e) => tracing::warn!(?exch, ?stream, error = %e, "subscribe skipped"),
        }
    }
    if handles.is_empty() {
        anyhow::bail!("all subscriptions failed");
    }
    tracing::info!("Station subscribed: {} streams active", handles.len());

    // Merge all per-stream handles into one mpsc channel.
    let (merged_tx, mut handle) = tokio::sync::mpsc::unbounded_channel::<digdigdig3_station::Event>();
    for mut h in handles {
        let tx = merged_tx.clone();
        tokio::spawn(async move {
            while let Some(ev) = h.recv().await {
                if tx.send(ev).is_err() {
                    break;
                }
            }
        });
    }
    drop(merged_tx);

    let deadline = Instant::now() + Duration::from_secs(duration_secs);
    let mut total_events: u64 = 0;

    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            tracing::info!("Duration elapsed — stopping");
            break;
        }

        let maybe_ev = timeout(remaining, handle.recv()).await;

        match maybe_ev {
            Err(_) => {
                tracing::info!("Timeout reached — stopping");
                break;
            }
            Ok(None) => {
                tracing::info!("Station stream closed");
                break;
            }
            Ok(Some(ev)) => {
                total_events += 1;
                if total_events % 500 == 0 {
                    tracing::info!(total_events, "events processed…");
                }

                use digdigdig3_station::Event;
                match &ev {
                    Event::Bar { point, .. } => {
                        let ts = point.open_time;
                        let (o, h, l, c, v) = (
                            point.open, point.high, point.low, point.close, point.volume,
                        );
                        for s in &mut states {
                            if s.stream_kind == StreamKind::Bar {
                                s.try_update_bar(o, h, l, c, v, ts);
                            }
                        }
                    }
                    Event::Trade { point, .. } => {
                        let tick = trade_point_to_tick(point);
                        for s in &mut states {
                            if s.stream_kind == StreamKind::Tick {
                                s.try_update_tick(&tick);
                            }
                        }
                    }
                    Event::AggTrade { point, .. } => {
                        let agg = agg_trade_point_to_core(point);
                        for s in &mut states {
                            if s.stream_kind == StreamKind::AggTrade {
                                s.try_update_agg_trade(&agg);
                            }
                        }
                    }
                    Event::Ticker { point, .. } => {
                        let ticker = ticker_point_to_core(point);
                        for s in &mut states {
                            if s.stream_kind == StreamKind::Ticker {
                                s.try_update_ticker(&ticker);
                            }
                        }
                    }
                    Event::OrderbookSnapshot { point, .. } => {
                        let book = obs_to_orderbook(point);
                        for s in &mut states {
                            if s.stream_kind == StreamKind::OrderBook {
                                s.try_update_orderbook(&book);
                            }
                        }
                    }
                    Event::MarkPrice { point, .. } => {
                        let mp = mark_point_to_core(point);
                        for s in &mut states {
                            if s.stream_kind == StreamKind::MarkPrice {
                                s.try_update_mark(&mp);
                            }
                        }
                    }
                    Event::FundingRate { point, .. } => {
                        let fr = funding_point_to_core(point);
                        for s in &mut states {
                            if s.stream_kind == StreamKind::Funding {
                                s.try_update_funding(&fr);
                            }
                        }
                    }
                    Event::OpenInterest { point, .. } => {
                        let oi = oi_point_to_core(point);
                        for s in &mut states {
                            if s.stream_kind == StreamKind::OpenInterest {
                                s.try_update_oi(&oi);
                            }
                        }
                    }
                    Event::Liquidation { point, .. } => {
                        let liq = liq_point_to_core(point);
                        for s in &mut states {
                            if s.stream_kind == StreamKind::Liquidation {
                                s.try_update_liquidation(&liq);
                            }
                        }
                    }
                }
            }
        }
    }

    // ─── Build report ────────────────────────────────────────────────────────
    let records: Vec<IndicatorRecord> = states.iter().map(|s| s.finalize_record()).collect();

    let n_created_ok = records.iter().filter(|r| r.status != "create_failed").count();
    let n_create_failed = records.iter().filter(|r| r.status == "create_failed").count();
    let n_panic = records.iter().filter(|r| r.status == "panic").count();
    let n_never_event = records.iter().filter(|r| r.status == "never_received_event").count();
    let n_never_ready = records.iter().filter(|r| r.status == "never_ready").count();
    let n_always_zero = records.iter().filter(|r| r.status == "always_zero").count();
    let n_nan_inf = records.iter().filter(|r| r.status == "always_nan_inf").count();
    let n_pass = records.iter().filter(|r| r.status == "pass").count();
    let n_matched = records.iter().filter(|r| r.matched_signature).count();

    let panic_list: Vec<&str> = records
        .iter()
        .filter(|r| r.status == "panic")
        .take(20)
        .map(|r| r.id.as_str())
        .collect();

    // Per-category breakdown
    let mut cat_map: HashMap<&str, (usize, usize, usize)> = HashMap::new();
    for r in &records {
        let e = cat_map.entry(r.category.as_str()).or_default();
        e.0 += 1;
        if r.status == "pass" { e.1 += 1; } else { e.2 += 1; }
    }

    // First 30 failures
    let failures: Vec<&IndicatorRecord> = records
        .iter()
        .filter(|r| r.status != "pass")
        .take(30)
        .collect();

    println!();
    println!("=== Indicator Validator Summary ===");
    println!("Total indicators tried:          {total}");
    println!("  with catalog signature:        {n_matched}");
    println!("  without signature (Bar dflt):  {}", total - n_matched);
    println!("  routed to non-Bar streams:     {n_non_bar}");
    println!();
    println!("=== Catalog coverage ===");
    println!("  signatures with machine_id:    {n_catalogued}");
    println!("  no catalog entry (fallback):   {n_uncatalogued}");
    println!();
    println!("=== Per-StreamKind routing ===");
    let mut sk_sorted: Vec<_> = sk_counts.into_iter().collect();
    sk_sorted.sort_by_key(|(k, _)| k.clone());
    for (sk, cnt) in &sk_sorted {
        println!("  {sk:<24}  {cnt}");
    }
    println!();
    println!("=== Run results ===");
    println!("  created OK:          {n_created_ok}");
    println!("  create_failed:       {n_create_failed}");
    println!("  panic_during_update: {n_panic}");
    if !panic_list.is_empty() {
        println!("    (first 20: {})", panic_list.join(", "));
    }
    println!("  never_received_event:{n_never_event}");
    println!("  never_ready:         {n_never_ready}");
    println!("  always_zero:         {n_always_zero}");
    println!("  always_nan_inf:      {n_nan_inf}");
    println!("  pass:                {n_pass}");
    println!();
    println!("=== Per-category breakdown ===");
    let mut cats: Vec<(&str, (usize, usize, usize))> = cat_map.into_iter().collect();
    cats.sort_by_key(|(c, _)| *c);
    for (cat, (tried, pass, fail)) in &cats {
        println!("  {cat:<28}  {tried} tried, {pass} pass, {fail} fail");
    }
    println!();
    println!("=== First 30 failures ===");
    println!("{:<40}  {:<24}  {:<16}  matched  last_value", "id", "reason", "stream");
    for r in &failures {
        let val_short = if r.last_value_repr.len() > 50 {
            &r.last_value_repr[..50]
        } else {
            &r.last_value_repr
        };
        println!(
            "{:<40}  {:<24}  {:<16}  {:<7}  {}",
            r.id,
            r.status,
            r.stream_kind,
            if r.matched_signature { "yes" } else { "no" },
            val_short,
        );
    }
    println!();
    println!("Total events processed: {total_events}");

    // Write JSON report
    let report_path = "validator_report.json";
    let json = serde_json::to_string_pretty(&records)?;
    std::fs::write(report_path, &json)?;
    println!("Report written to {report_path}");

    Ok(())
}
