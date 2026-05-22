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
    AggTrade, AuctionEvent, Basis, BlockTrade, CompositeIndex, FundingRate,
    FundingSettlement, HistoricalVolatility, IndexPrice, InsuranceFund,
    L3Action, Liquidation, MarkPrice, MarketWarning, OpenInterest, OptionGreeks,
    OrderBook, OrderBookLevel, OrderBookSide, OrderbookL3Event, PredictedFunding,
    RiskLimit, SettlementEvent, Ticker, Tick, TradeSide, VolatilityIndex,
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
) -> (HashMap<BarIndicatorId, (StreamKind, &'static [StreamKind])>, usize, usize) {
    let catalog = MasterIndicatorCatalog::new();

    let mut map: HashMap<BarIndicatorId, (StreamKind, &'static [StreamKind])> = HashMap::new();
    let mut n_catalogued: usize = 0;

    for sig in catalog.iter_signatures() {
        if let Some(machine_id) = sig.machine_id {
            map.insert(machine_id, (sig.input_stream, sig.aux_streams));
            n_catalogued += 1;
        }
    }

    let n_uncatalogued = all_indicator_ids
        .iter()
        .filter(|id| !map.contains_key(id))
        .count();

    for &id in all_indicator_ids {
        map.entry(id).or_insert((StreamKind::Bar, &[]));
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
    aux_streams: &'static [StreamKind],
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
    /// Returns true if the indicator should receive events from this stream
    /// (matches primary input_stream OR any aux_streams). Composites and
    /// hybrid indicators have non-empty aux_streams.
    fn accepts(&self, kind: StreamKind) -> bool {
        self.stream_kind == kind || self.aux_streams.contains(&kind)
    }
}

/// Period fallback ladder for indicators that reject `vec![14]`.
/// Many multi-period indicators (UltimateOscillator, MACD-likes, etc.) require
/// a specific shape — try common ones before giving up.
const PERIOD_LADDER: &[&[usize]] = &[
    &[14],                 // most common single-period default
    &[14, 28],             // dual MA / 2-period
    &[12, 26],             // MACD-like fast/slow
    &[5, 14, 28],          // triple-period (UltimateOscillator)
    &[7, 14, 28],          // alt triple
    &[12, 26, 9],          // MACD + signal
    &[20],                 // BB / Donchian-like
    &[10],
    &[5],
    &[3],
    &[2],
];

impl IndicatorState {
    fn new(id: BarIndicatorId, stream_kind: StreamKind, aux_streams: &'static [StreamKind], matched_signature: bool) -> Self {
        let category = category_of(id);
        let (instance, create_error, periods_used) = Self::try_create_with_ladder(id);
        if let Some(p) = &periods_used {
            tracing::debug!(?id, ?p, "indicator created with non-default periods");
        }
        Self {
            id,
            category,
            stream_kind,
            aux_streams,
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

    /// Per-id param overrides for indicators whose factory defaults don't
    /// trigger on 1m bar data (e.g. Zigzag default 5% swing requires ~$3000
    /// move on BTC — never fires in a 30min window).
    fn tuned_params(id: BarIndicatorId) -> Vec<(&'static str, f64)> {
        match id {
            // SwingDetection::Percent — factory uses additional_params["deviation"] * 100
            // → 0.005 * 100 = 0.5% (was 5%)
            BarIndicatorId::Zigzag | BarIndicatorId::ZigzagDiv => {
                vec![("deviation", 0.005)]
            }
            // CusumFilter threshold (factory default ~0.01 = 1% cumulative return)
            // Lower to 0.003 (0.3%) — common per-bar BTC log-return scale.
            BarIndicatorId::Cusum | BarIndicatorId::StCusum => {
                vec![("threshold", 0.003)]
            }
            // BpCusum — kappa/threshold pair (test fixture: threshold=0.05, kappa=0.95)
            BarIndicatorId::BpCusum => {
                vec![("threshold", 0.05), ("kappa", 0.95)]
            }
            _ => Vec::new(),
        }
    }

    /// Try IndicatorInstance::create with PERIOD_LADDER fallbacks until one
    /// works. Returns (instance, error, periods_used_if_non_default).
    fn try_create_with_ladder(
        id: BarIndicatorId,
    ) -> (Option<IndicatorInstance>, Option<String>, Option<Vec<usize>>) {
        let mut last_err: Option<String> = None;
        let extra_params = Self::tuned_params(id);
        for (idx, periods) in PERIOD_LADDER.iter().enumerate() {
            let p = periods.to_vec();
            let mut cfg = IndicatorConfig::new(id, format!("{id:?}"), p.clone());
            for (k, v) in &extra_params {
                cfg = cfg.with_param(*k, *v);
            }
            let result =
                panic::catch_unwind(AssertUnwindSafe(|| IndicatorInstance::create(&cfg)));
            match result {
                Ok(Ok(inst)) => {
                    let used = if idx == 0 { None } else { Some(p) };
                    return (Some(inst), None, used);
                }
                Ok(Err(e)) => last_err = Some(e),
                Err(_) => last_err = Some("panic during create".to_string()),
            }
        }
        (None, last_err, None)
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

    fn try_update_block_trade(&mut self, bt: &BlockTrade) {
        let inst = match &mut self.instance { Some(i) => i, None => return };
        self.events_received += 1;
        let result = panic::catch_unwind(AssertUnwindSafe(|| inst.update_block_trade(bt)));
        match result {
            Ok(val) => {
                if !self.is_ready {
                    if matches!(panic::catch_unwind(AssertUnwindSafe(|| inst.is_ready())), Ok(true)) {
                        self.is_ready = true;
                        self.ready_at_event = Some(self.events_received);
                    }
                }
                self.record_value(val);
            }
            Err(_) => { self.panic_count += 1; }
        }
    }

    fn try_update_index_price(&mut self, ip: &IndexPrice) {
        let inst = match &mut self.instance { Some(i) => i, None => return };
        self.events_received += 1;
        let result = panic::catch_unwind(AssertUnwindSafe(|| inst.update_index_price(ip)));
        match result {
            Ok(val) => {
                if !self.is_ready {
                    if matches!(panic::catch_unwind(AssertUnwindSafe(|| inst.is_ready())), Ok(true)) {
                        self.is_ready = true;
                        self.ready_at_event = Some(self.events_received);
                    }
                }
                self.record_value(val);
            }
            Err(_) => { self.panic_count += 1; }
        }
    }

    fn try_update_composite_index(&mut self, ci: &CompositeIndex) {
        let inst = match &mut self.instance { Some(i) => i, None => return };
        self.events_received += 1;
        let result = panic::catch_unwind(AssertUnwindSafe(|| inst.update_composite_index(ci)));
        match result {
            Ok(val) => {
                if !self.is_ready {
                    if matches!(panic::catch_unwind(AssertUnwindSafe(|| inst.is_ready())), Ok(true)) {
                        self.is_ready = true;
                        self.ready_at_event = Some(self.events_received);
                    }
                }
                self.record_value(val);
            }
            Err(_) => { self.panic_count += 1; }
        }
    }

    fn try_update_option_greeks(&mut self, g: &OptionGreeks) {
        let inst = match &mut self.instance { Some(i) => i, None => return };
        self.events_received += 1;
        let result = panic::catch_unwind(AssertUnwindSafe(|| inst.update_option_greeks(g)));
        match result {
            Ok(val) => {
                if !self.is_ready {
                    if matches!(panic::catch_unwind(AssertUnwindSafe(|| inst.is_ready())), Ok(true)) {
                        self.is_ready = true;
                        self.ready_at_event = Some(self.events_received);
                    }
                }
                self.record_value(val);
            }
            Err(_) => { self.panic_count += 1; }
        }
    }

    fn try_update_volatility_index(&mut self, vi: &VolatilityIndex) {
        let inst = match &mut self.instance { Some(i) => i, None => return };
        self.events_received += 1;
        let result = panic::catch_unwind(AssertUnwindSafe(|| inst.update_volatility_index(vi)));
        match result {
            Ok(val) => {
                if !self.is_ready {
                    if matches!(panic::catch_unwind(AssertUnwindSafe(|| inst.is_ready())), Ok(true)) {
                        self.is_ready = true;
                        self.ready_at_event = Some(self.events_received);
                    }
                }
                self.record_value(val);
            }
            Err(_) => { self.panic_count += 1; }
        }
    }

    fn try_update_historical_volatility(&mut self, hv: &HistoricalVolatility) {
        let inst = match &mut self.instance { Some(i) => i, None => return };
        self.events_received += 1;
        let result =
            panic::catch_unwind(AssertUnwindSafe(|| inst.update_historical_volatility(hv)));
        match result {
            Ok(val) => {
                if !self.is_ready {
                    if matches!(panic::catch_unwind(AssertUnwindSafe(|| inst.is_ready())), Ok(true)) {
                        self.is_ready = true;
                        self.ready_at_event = Some(self.events_received);
                    }
                }
                self.record_value(val);
            }
            Err(_) => { self.panic_count += 1; }
        }
    }

    fn try_update_basis(&mut self, b: &Basis) {
        let inst = match &mut self.instance { Some(i) => i, None => return };
        self.events_received += 1;
        let result = panic::catch_unwind(AssertUnwindSafe(|| inst.update_basis(b)));
        match result {
            Ok(val) => {
                if !self.is_ready {
                    if matches!(panic::catch_unwind(AssertUnwindSafe(|| inst.is_ready())), Ok(true)) {
                        self.is_ready = true;
                        self.ready_at_event = Some(self.events_received);
                    }
                }
                self.record_value(val);
            }
            Err(_) => { self.panic_count += 1; }
        }
    }

    fn try_update_insurance_fund(&mut self, ins: &InsuranceFund) {
        let inst = match &mut self.instance { Some(i) => i, None => return };
        self.events_received += 1;
        let result = panic::catch_unwind(AssertUnwindSafe(|| inst.update_insurance_fund(ins)));
        match result {
            Ok(val) => {
                if !self.is_ready {
                    if matches!(panic::catch_unwind(AssertUnwindSafe(|| inst.is_ready())), Ok(true)) {
                        self.is_ready = true;
                        self.ready_at_event = Some(self.events_received);
                    }
                }
                self.record_value(val);
            }
            Err(_) => { self.panic_count += 1; }
        }
    }

    fn try_update_orderbook_l3(&mut self, l3: &OrderbookL3Event) {
        let inst = match &mut self.instance { Some(i) => i, None => return };
        self.events_received += 1;
        let result = panic::catch_unwind(AssertUnwindSafe(|| inst.update_orderbook_l3(l3)));
        match result {
            Ok(val) => {
                if !self.is_ready {
                    if matches!(panic::catch_unwind(AssertUnwindSafe(|| inst.is_ready())), Ok(true)) {
                        self.is_ready = true;
                        self.ready_at_event = Some(self.events_received);
                    }
                }
                self.record_value(val);
            }
            Err(_) => { self.panic_count += 1; }
        }
    }

    fn try_update_settlement(&mut self, s: &SettlementEvent) {
        let inst = match &mut self.instance { Some(i) => i, None => return };
        self.events_received += 1;
        let result = panic::catch_unwind(AssertUnwindSafe(|| inst.update_settlement(s)));
        match result {
            Ok(val) => {
                if !self.is_ready {
                    if matches!(panic::catch_unwind(AssertUnwindSafe(|| inst.is_ready())), Ok(true)) {
                        self.is_ready = true;
                        self.ready_at_event = Some(self.events_received);
                    }
                }
                self.record_value(val);
            }
            Err(_) => { self.panic_count += 1; }
        }
    }

    fn try_update_auction(&mut self, a: &AuctionEvent) {
        let inst = match &mut self.instance { Some(i) => i, None => return };
        self.events_received += 1;
        let result = panic::catch_unwind(AssertUnwindSafe(|| inst.update_auction(a)));
        match result {
            Ok(val) => {
                if !self.is_ready {
                    if matches!(panic::catch_unwind(AssertUnwindSafe(|| inst.is_ready())), Ok(true)) {
                        self.is_ready = true;
                        self.ready_at_event = Some(self.events_received);
                    }
                }
                self.record_value(val);
            }
            Err(_) => { self.panic_count += 1; }
        }
    }

    fn try_update_market_warning(&mut self, w: &MarketWarning) {
        let inst = match &mut self.instance { Some(i) => i, None => return };
        self.events_received += 1;
        let result = panic::catch_unwind(AssertUnwindSafe(|| inst.update_market_warning(w)));
        match result {
            Ok(val) => {
                if !self.is_ready {
                    if matches!(panic::catch_unwind(AssertUnwindSafe(|| inst.is_ready())), Ok(true)) {
                        self.is_ready = true;
                        self.ready_at_event = Some(self.events_received);
                    }
                }
                self.record_value(val);
            }
            Err(_) => { self.panic_count += 1; }
        }
    }

    fn try_update_risk_limit(&mut self, r: &RiskLimit) {
        let inst = match &mut self.instance { Some(i) => i, None => return };
        self.events_received += 1;
        let result = panic::catch_unwind(AssertUnwindSafe(|| inst.update_risk_limit(r)));
        match result {
            Ok(val) => {
                if !self.is_ready {
                    if matches!(panic::catch_unwind(AssertUnwindSafe(|| inst.is_ready())), Ok(true)) {
                        self.is_ready = true;
                        self.ready_at_event = Some(self.events_received);
                    }
                }
                self.record_value(val);
            }
            Err(_) => { self.panic_count += 1; }
        }
    }

    fn try_update_predicted_funding(&mut self, pf: &PredictedFunding) {
        let inst = match &mut self.instance { Some(i) => i, None => return };
        self.events_received += 1;
        let result = panic::catch_unwind(AssertUnwindSafe(|| inst.update_predicted_funding(pf)));
        match result {
            Ok(val) => {
                if !self.is_ready {
                    if matches!(panic::catch_unwind(AssertUnwindSafe(|| inst.is_ready())), Ok(true)) {
                        self.is_ready = true;
                        self.ready_at_event = Some(self.events_received);
                    }
                }
                self.record_value(val);
            }
            Err(_) => { self.panic_count += 1; }
        }
    }

    fn try_update_funding_settlement(&mut self, fs: &FundingSettlement) {
        let inst = match &mut self.instance { Some(i) => i, None => return };
        self.events_received += 1;
        let result =
            panic::catch_unwind(AssertUnwindSafe(|| inst.update_funding_settlement(fs)));
        match result {
            Ok(val) => {
                if !self.is_ready {
                    if matches!(panic::catch_unwind(AssertUnwindSafe(|| inst.is_ready())), Ok(true)) {
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
        IndicatorValue::ValueFlag(v, flag) => vec![*v, if *flag { 1.0 } else { 0.0 }],
        IndicatorValue::DoubleFlag(a, b) => vec![if *a { 1.0 } else { 0.0 }, if *b { 1.0 } else { 0.0 }],
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

fn block_trade_point_to_core(p: &digdigdig3_station::data::BlockTradePoint) -> BlockTrade {
    BlockTrade {
        block_id: p.block_id.clone(),
        price: p.price,
        quantity: p.quantity,
        is_buy: p.side == digdigdig3::core::types::TradeSide::Buy,
        timestamp: p.ts_ms,
        is_iv: p.is_iv,
    }
}

fn index_price_point_to_core(p: &digdigdig3_station::data::IndexPricePoint) -> IndexPrice {
    IndexPrice {
        price: p.price,
        timestamp: p.ts_ms,
    }
}

fn composite_index_point_to_core(
    p: &digdigdig3_station::data::CompositeIndexPoint,
) -> CompositeIndex {
    CompositeIndex {
        price: p.price,
        components: Vec::new(),
        timestamp: p.ts_ms,
    }
}

fn option_greeks_point_to_core(p: &digdigdig3_station::data::OptionGreeksPoint) -> OptionGreeks {
    OptionGreeks {
        delta: p.delta,
        gamma: p.gamma,
        vega: p.vega,
        theta: p.theta,
        rho: p.rho,
        mark_iv: p.mark_iv,
        bid_iv: if p.bid_iv.is_nan() { None } else { Some(p.bid_iv) },
        ask_iv: if p.ask_iv.is_nan() { None } else { Some(p.ask_iv) },
        timestamp: p.ts_ms,
    }
}

fn volatility_index_point_to_core(
    p: &digdigdig3_station::data::VolatilityIndexPoint,
) -> VolatilityIndex {
    VolatilityIndex {
        value: p.value,
        timestamp: p.ts_ms,
    }
}

fn historical_volatility_point_to_core(
    p: &digdigdig3_station::data::HistoricalVolatilityPoint,
) -> HistoricalVolatility {
    HistoricalVolatility {
        volatility: p.volatility,
        timestamp: p.ts_ms,
    }
}

fn basis_point_to_core(p: &digdigdig3_station::data::BasisPoint) -> Basis {
    Basis {
        basis: p.basis,
        timestamp: p.ts_ms,
    }
}

fn insurance_fund_point_to_core(p: &digdigdig3_station::data::InsuranceFundPoint) -> InsuranceFund {
    InsuranceFund {
        balance: p.balance,
        timestamp: p.ts_ms,
    }
}

fn orderbook_l3_point_to_core(p: &digdigdig3_station::data::OrderbookL3Point) -> OrderbookL3Event {
    OrderbookL3Event {
        side: match p.side {
            digdigdig3::core::types::OrderSide::Buy => OrderBookSide::Bid,
            digdigdig3::core::types::OrderSide::Sell => OrderBookSide::Ask,
        },
        order_id: p.order_id.clone(),
        price: p.price,
        quantity: p.quantity,
        action: L3Action::Add,
        timestamp: p.ts_ms,
    }
}

fn settlement_event_point_to_core(
    p: &digdigdig3_station::data::SettlementEventPoint,
) -> SettlementEvent {
    SettlementEvent {
        settlement_price: p.settlement_price,
        settlement_time: p.settlement_time,
        timestamp: p.ts_ms,
    }
}

fn auction_event_point_to_core(p: &digdigdig3_station::data::AuctionEventPoint) -> AuctionEvent {
    AuctionEvent {
        auction_id: p.auction_id.clone(),
        indicative_price: p.indicative_price,
        indicative_qty: p.indicative_qty,
        state: p.state.clone(),
        timestamp: p.ts_ms,
    }
}

fn market_warning_point_to_core(p: &digdigdig3_station::data::MarketWarningPoint) -> MarketWarning {
    MarketWarning {
        symbol: String::new(),
        warning_kind: p.warning_kind.clone(),
        message: p.message.clone(),
        timestamp: p.ts_ms,
    }
}

fn risk_limit_point_to_core(p: &digdigdig3_station::data::RiskLimitPoint) -> RiskLimit {
    RiskLimit {
        tier: p.tier,
        max_leverage: p.max_leverage,
        max_position_value: p.max_position_value,
        mmr: p.maintenance_margin_rate,
        imr: p.initial_margin_rate,
        timestamp: p.ts_ms,
    }
}

fn predicted_funding_point_to_core(
    p: &digdigdig3_station::data::PredictedFundingPoint,
) -> PredictedFunding {
    PredictedFunding {
        predicted_rate: p.predicted_rate,
        next_funding_time: p.next_funding_time,
        timestamp: p.ts_ms,
    }
}

fn funding_settlement_point_to_core(
    p: &digdigdig3_station::data::FundingSettlementPoint,
) -> FundingSettlement {
    FundingSettlement {
        settled_rate: p.settled_rate,
        settlement_time: p.settlement_time,
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
        let (sk, _aux) = stream_kind_map[&id];
        *sk_counts.entry(sk.as_str().to_string()).or_default() += 1;
    }

    let n_non_bar: usize = all_indicator_ids
        .iter()
        .filter(|&&id| stream_kind_map[&id].0 != StreamKind::Bar)
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
            let (sk, aux) = stream_kind_map[&id];
            let matched_sig = catalogued_ids.contains(&id);
            IndicatorState::new(id, sk, aux, matched_sig)
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
    let combos: &[(ExchangeId, AccountType, &str, Stream)] = &[
        // Core 9 streams on Binance + Bybit FuturesCross — well-covered indicators
        (ExchangeId::Binance, AccountType::FuturesCross, "BTCUSDT", Stream::Trade),
        (ExchangeId::Binance, AccountType::FuturesCross, "BTCUSDT", Stream::AggTrade),
        (ExchangeId::Binance, AccountType::FuturesCross, "BTCUSDT", Stream::Kline(interval_1m.clone())),
        (ExchangeId::Binance, AccountType::FuturesCross, "BTCUSDT", Stream::Ticker),
        (ExchangeId::Binance, AccountType::FuturesCross, "BTCUSDT", Stream::Orderbook),
        (ExchangeId::Binance, AccountType::FuturesCross, "BTCUSDT", Stream::MarkPrice),
        (ExchangeId::Binance, AccountType::FuturesCross, "BTCUSDT", Stream::FundingRate),
        (ExchangeId::Binance, AccountType::FuturesCross, "BTCUSDT", Stream::Liquidation),
        (ExchangeId::Bybit, AccountType::FuturesCross, "BTCUSDT", Stream::Trade),
        (ExchangeId::Bybit, AccountType::FuturesCross, "BTCUSDT", Stream::AggTrade),
        (ExchangeId::Bybit, AccountType::FuturesCross, "BTCUSDT", Stream::Kline(interval_1m.clone())),
        (ExchangeId::Bybit, AccountType::FuturesCross, "BTCUSDT", Stream::Ticker),
        (ExchangeId::Bybit, AccountType::FuturesCross, "BTCUSDT", Stream::Orderbook),
        (ExchangeId::Bybit, AccountType::FuturesCross, "BTCUSDT", Stream::MarkPrice),
        (ExchangeId::Bybit, AccountType::FuturesCross, "BTCUSDT", Stream::FundingRate),
        (ExchangeId::Bybit, AccountType::FuturesCross, "BTCUSDT", Stream::Liquidation),
        (ExchangeId::Bybit, AccountType::FuturesCross, "BTCUSDT", Stream::OpenInterest),
        // Extended streams — targeted at exchanges whose dig3 protocol.rs
        // actually declares topic-registry entries for that StreamKind.
        //
        // Deribit (Options + Futures perpetual)
        (ExchangeId::Deribit, AccountType::Options, "BTC-23MAY26-86000-C", Stream::OptionGreeks),
        (ExchangeId::Deribit, AccountType::Options, "BTC-23MAY26-86000-C", Stream::VolatilityIndex),
        (ExchangeId::Deribit, AccountType::Options, "BTC-23MAY26-86000-C", Stream::BlockTrade),
        (ExchangeId::Deribit, AccountType::Options, "BTC-23MAY26-86000-C", Stream::IndexPrice),
        // OKX FuturesCross
        (ExchangeId::OKX, AccountType::FuturesCross, "BTCUSDT", Stream::OptionGreeks),
        (ExchangeId::OKX, AccountType::FuturesCross, "BTCUSDT", Stream::BlockTrade),
        (ExchangeId::OKX, AccountType::FuturesCross, "BTCUSDT", Stream::IndexPrice),
        (ExchangeId::OKX, AccountType::FuturesCross, "BTCUSDT", Stream::IndexPriceKline(interval_1m.clone())),
        (ExchangeId::OKX, AccountType::FuturesCross, "BTCUSDT", Stream::MarkPriceKline(interval_1m.clone())),
        (ExchangeId::OKX, AccountType::FuturesCross, "BTCUSDT", Stream::SettlementEvent),
        // Binance FuturesCross — index/composite kline family
        (ExchangeId::Binance, AccountType::FuturesCross, "BTCUSDT", Stream::IndexPrice),
        (ExchangeId::Binance, AccountType::FuturesCross, "BTCUSDT", Stream::CompositeIndex),
        (ExchangeId::Binance, AccountType::FuturesCross, "BTCUSDT", Stream::IndexPriceKline(interval_1m.clone())),
        (ExchangeId::Binance, AccountType::FuturesCross, "BTCUSDT", Stream::MarkPriceKline(interval_1m.clone())),
        (ExchangeId::Binance, AccountType::FuturesCross, "BTCUSDT", Stream::PremiumIndexKline(interval_1m.clone())),
        // Bybit — risk + insurance
        (ExchangeId::Bybit, AccountType::FuturesCross, "BTCUSDT", Stream::InsuranceFund),
        (ExchangeId::Bybit, AccountType::FuturesCross, "BTCUSDT", Stream::RiskLimit),
        // GateIO
        (ExchangeId::GateIO, AccountType::FuturesCross, "BTCUSDT", Stream::MarkPriceKline(interval_1m.clone())),
        // Hyperliquid
        (ExchangeId::HyperLiquid, AccountType::FuturesCross, "BTC", Stream::IndexPrice),
        (ExchangeId::HyperLiquid, AccountType::FuturesCross, "BTC", Stream::MarketWarning),
        // HTX
        (ExchangeId::HTX, AccountType::FuturesCross, "BTCUSDT", Stream::IndexPrice),
        (ExchangeId::HTX, AccountType::FuturesCross, "BTCUSDT", Stream::IndexPriceKline(interval_1m.clone())),
        // Index-price only
        (ExchangeId::Bitget, AccountType::FuturesCross, "BTCUSDT", Stream::IndexPrice),
        (ExchangeId::KuCoin, AccountType::FuturesCross, "BTCUSDT", Stream::IndexPrice),
        (ExchangeId::MEXC, AccountType::FuturesCross, "BTCUSDT", Stream::IndexPrice),
    ];

    let mut handles: Vec<_> = Vec::new();
    let mut total_failed = 0usize;
    for (exch, acct, sym, stream) in combos {
        // Use add_raw for exchange-native instrument IDs that don't fit
        // canonical BASE-QUOTE shape (e.g. Deribit options).
        let single = if matches!(exch, ExchangeId::Deribit) {
            SubscriptionSet::new().add_raw(*exch, *sym, *acct, [stream.clone()])
        } else {
            SubscriptionSet::new().add(*exch, *sym, *acct, [stream.clone()])
        };
        match station.subscribe(single).await {
            Ok(report) => {
                for f in &report.failed {
                    total_failed += 1;
                    tracing::warn!(?exch, ?acct, sym = sym, ?stream, reason = ?f, "per-stream subscribe failed");
                }
                if !report.ok.is_empty() {
                    handles.push(report.handle);
                }
            }
            Err(e) => tracing::warn!(?exch, ?acct, sym = sym, ?stream, error = %e, "subscribe call errored"),
        }
    }
    tracing::info!("Subscribe pass complete: {} live handles, {} per-stream failures", handles.len(), total_failed);
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
                            if s.accepts(StreamKind::Bar) {
                                s.try_update_bar(o, h, l, c, v, ts);
                            }
                        }
                    }
                    Event::Trade { point, .. } => {
                        let tick = trade_point_to_tick(point);
                        for s in &mut states {
                            if s.accepts(StreamKind::Tick) {
                                s.try_update_tick(&tick);
                            }
                        }
                    }
                    Event::AggTrade { point, .. } => {
                        let agg = agg_trade_point_to_core(point);
                        for s in &mut states {
                            if s.accepts(StreamKind::AggTrade) {
                                s.try_update_agg_trade(&agg);
                            }
                        }
                    }
                    Event::Ticker { point, .. } => {
                        let ticker = ticker_point_to_core(point);
                        for s in &mut states {
                            if s.accepts(StreamKind::Ticker) {
                                s.try_update_ticker(&ticker);
                            }
                        }
                    }
                    Event::OrderbookSnapshot { point, .. } => {
                        let book = obs_to_orderbook(point);
                        for s in &mut states {
                            if s.accepts(StreamKind::OrderBook) {
                                s.try_update_orderbook(&book);
                            }
                        }
                    }
                    Event::MarkPrice { point, .. } => {
                        let mp = mark_point_to_core(point);
                        for s in &mut states {
                            if s.accepts(StreamKind::MarkPrice) {
                                s.try_update_mark(&mp);
                            }
                        }
                    }
                    Event::FundingRate { point, .. } => {
                        let fr = funding_point_to_core(point);
                        for s in &mut states {
                            if s.accepts(StreamKind::Funding) {
                                s.try_update_funding(&fr);
                            }
                        }
                    }
                    Event::OpenInterest { point, .. } => {
                        let oi = oi_point_to_core(point);
                        for s in &mut states {
                            if s.accepts(StreamKind::OpenInterest) {
                                s.try_update_oi(&oi);
                            }
                        }
                    }
                    Event::Liquidation { point, .. } => {
                        let liq = liq_point_to_core(point);
                        for s in &mut states {
                            if s.accepts(StreamKind::Liquidation) {
                                s.try_update_liquidation(&liq);
                            }
                        }
                    }
                    Event::BlockTrade { point, .. } => {
                        let bt = block_trade_point_to_core(point);
                        for s in &mut states {
                            if s.accepts(StreamKind::BlockTrade) {
                                s.try_update_block_trade(&bt);
                            }
                        }
                    }
                    Event::IndexPrice { point, .. } => {
                        let ip = index_price_point_to_core(point);
                        for s in &mut states {
                            if s.accepts(StreamKind::IndexPrice) {
                                s.try_update_index_price(&ip);
                            }
                        }
                    }
                    Event::CompositeIndex { point, .. } => {
                        let ci = composite_index_point_to_core(point);
                        for s in &mut states {
                            if s.accepts(StreamKind::CompositeIndex) {
                                s.try_update_composite_index(&ci);
                            }
                        }
                    }
                    Event::OptionGreeks { point, .. } => {
                        let g = option_greeks_point_to_core(point);
                        for s in &mut states {
                            if s.accepts(StreamKind::OptionGreeks) {
                                s.try_update_option_greeks(&g);
                            }
                        }
                    }
                    Event::VolatilityIndex { point, .. } => {
                        let vi = volatility_index_point_to_core(point);
                        for s in &mut states {
                            if s.accepts(StreamKind::VolatilityIndex) {
                                s.try_update_volatility_index(&vi);
                            }
                        }
                    }
                    Event::HistoricalVolatility { point, .. } => {
                        let hv = historical_volatility_point_to_core(point);
                        for s in &mut states {
                            if s.accepts(StreamKind::HistoricalVolatility) {
                                s.try_update_historical_volatility(&hv);
                            }
                        }
                    }
                    Event::Basis { point, .. } => {
                        let b = basis_point_to_core(point);
                        for s in &mut states {
                            if s.accepts(StreamKind::Basis) {
                                s.try_update_basis(&b);
                            }
                        }
                    }
                    Event::InsuranceFund { point, .. } => {
                        let ins = insurance_fund_point_to_core(point);
                        for s in &mut states {
                            if s.accepts(StreamKind::InsuranceFund) {
                                s.try_update_insurance_fund(&ins);
                            }
                        }
                    }
                    Event::OrderbookL3 { point, .. } => {
                        let l3 = orderbook_l3_point_to_core(point);
                        for s in &mut states {
                            if s.accepts(StreamKind::OrderbookL3) {
                                s.try_update_orderbook_l3(&l3);
                            }
                        }
                    }
                    Event::SettlementEvent { point, .. } => {
                        let se = settlement_event_point_to_core(point);
                        for s in &mut states {
                            if s.accepts(StreamKind::Settlement) {
                                s.try_update_settlement(&se);
                            }
                        }
                    }
                    Event::AuctionEvent { point, .. } => {
                        let a = auction_event_point_to_core(point);
                        for s in &mut states {
                            if s.accepts(StreamKind::Auction) {
                                s.try_update_auction(&a);
                            }
                        }
                    }
                    Event::MarketWarning { point, .. } => {
                        let w = market_warning_point_to_core(point);
                        for s in &mut states {
                            if s.accepts(StreamKind::MarketWarning) {
                                s.try_update_market_warning(&w);
                            }
                        }
                    }
                    Event::RiskLimit { point, .. } => {
                        let r = risk_limit_point_to_core(point);
                        for s in &mut states {
                            if s.accepts(StreamKind::RiskLimit) {
                                s.try_update_risk_limit(&r);
                            }
                        }
                    }
                    Event::PredictedFunding { point, .. } => {
                        let pf = predicted_funding_point_to_core(point);
                        for s in &mut states {
                            if s.accepts(StreamKind::PredictedFunding) {
                                s.try_update_predicted_funding(&pf);
                            }
                        }
                    }
                    Event::FundingSettlement { point, .. } => {
                        let fs = funding_settlement_point_to_core(point);
                        for s in &mut states {
                            if s.accepts(StreamKind::FundingSettlement) {
                                s.try_update_funding_settlement(&fs);
                            }
                        }
                    }
                    // Kline variants for mark/index/premium — routed as Bar
                    Event::MarkPriceKline { point, .. } => {
                        let ts = point.open_time;
                        let (o, h, l, c, v) = (
                            point.open, point.high, point.low, point.close, point.volume,
                        );
                        for s in &mut states {
                            if s.accepts(StreamKind::Bar) {
                                s.try_update_bar(o, h, l, c, v, ts);
                            }
                        }
                    }
                    Event::IndexPriceKline { point, .. } => {
                        let ts = point.open_time;
                        let (o, h, l, c, v) = (
                            point.open, point.high, point.low, point.close, point.volume,
                        );
                        for s in &mut states {
                            if s.accepts(StreamKind::Bar) {
                                s.try_update_bar(o, h, l, c, v, ts);
                            }
                        }
                    }
                    Event::PremiumIndexKline { point, .. } => {
                        let ts = point.open_time;
                        let (o, h, l, c, v) = (
                            point.open, point.high, point.low, point.close, point.volume,
                        );
                        for s in &mut states {
                            if s.accepts(StreamKind::Bar) {
                                s.try_update_bar(o, h, l, c, v, ts);
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
