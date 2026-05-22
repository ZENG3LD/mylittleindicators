//! High-performance mapping factory for all bar indicators (NO allocations, NO logic, just static mapping)
//! Usage: BAR_INDICATOR_MAP.get("sma") -> Some(BarIndicatorId::Sma

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BarIndicatorId {

    // Average (23 indicators) - Legacy Ama/Wma/Hma replaced by optimized O(1) versions
    Alma,  // ALMA
    Ama,  // AMA (O(1) ring window ER - was Amaring)
    AvFrama,  // AV_FRAMA
    AvVidya,  // AV_VIDYA
    Dema,  // DEMA
    Ehlersfa,  // EHLERSFA
    Ehlersz,  // EHLERSZ
    Ema,  // EMA
    Framaadv,  // FRAMAADV
    Hma,  // HMA (O(1) composite - was Hmafast)
    Jma,  // JMA
    Lr,  // LR
    Mcginley,  // MCGINLEY
    Rma,  // RMA
    Sma,  // SMA
    T3,  // T3
    Tema,  // TEMA
    Tma,  // TMA
    Trima,  // TRIMA
    Vwap,  // VWAP
    Vwma,  // VWMA
    Wma,  // WMA (O(1) running sums - was Wmafast)

    // Momentum (114 indicators)
    AdaptiveStoch,  // ADAPTIVE_STOCH
    Adx,  // ADX
    DiPlusMinus,  // DI_PLUS_MINUS (+DI/-DI from ADX)
    Amat,  // AMAT
    Apo,  // APO
    Aroon,  // AROON
    AroonDown,  // AROON_DOWN
    AroonOsc,  // AROON_OSC
    AroonUp,  // AROON_UP
    AtrRsi,  // ATR_RSI
    BbPeriod,  // BB_PERIOD
    Bias,  // BIAS
    Bop,  // BOP
    Cci,  // CCI
    Cfo,  // CFO
    Cmo,  // CMO
    Cog,  // COG
    ConnorsRsi,  // CONNORS_RSI
    Coppock,  // COPPOCK
    Demarker,  // DEMARKER
    Dm,  // DM
    Dpo,  // DPO
    DpoPct,  // DPO_PCT
    Dsp,  // DSP
    Dss,  // DSS
    EhlersCc,  // EHLERS_CC
    EhlersRocket,  // EHLERS_ROCKET
    ElderImpulse,  // ELDER_IMPULSE
    ElderRay,  // ELDER_RAY
    EmaSlope,  // EMA_SLOPE
    Ewmac,  // EWMAC
    EwmacRobust,  // EWMAC_ROBUST
    Gapo,  // GAPO
    Gator,  // GATOR
    Highest,  // HIGHEST
    IftRsi,  // IFT_RSI
    Imi,  // IMI
    Kdj,  // KDJ
    Kst,  // KST
    Kvo,  // KVO
    Lowest,  // LOWEST
    Macd,  // MACD
    MacdHist,  // MACD_HIST
    MacdHistZ,  // MACD_HIST_Z
    MacdSignal,  // MACD_SIGNAL
    MarketCipher,  // MARKET_CIPHER
    MoFisher,  // MO_FISHER
    MoObv,  // MO_OBV
    MomZscore,  // MOM_ZSCORE

    NeuralMom,  // NEURAL_MOM
    Pfe,  // PFE
    Pmo,  // PMO
    Ppo,  // PPO
    PpoSignal,  // PPO_SIGNAL
    Pressure,  // PRESSURE
    Psar,  // PSAR
    Psl,  // PSL
    Qqe,  // QQE
    Qstick,  // QSTICK
    Rmi,  // RMI
    Roc,  // ROC
    RocPct,  // ROC_PCT
    Rsi,  // RSI
    RsiPctBands,  // RSI_PCT_BANDS
    RsiPctRank,  // RSI_PCT_RANK
    RsiZscore,  // RSI_ZSCORE
    Rsioma,  // RSIOMA
    Rsx,  // RSX
    Rvgi,  // RVGI
    Rwi,  // RWI
    Smi,  // SMI
    Stc,  // STC
    Stoch,  // STOCH
    StochRsi,  // STOCH_RSI
    Stochkd,  // STOCHKD
    SweepRev,  // SWEEP_REV
    SwingAge,  // SWING_AGE
    Tdi,  // TDI
    Trix,  // TRIX
    Tsi,  // TSI
    Uo,  // UO
    UoSmooth,  // UO_SMOOTH
    Vhf,  // VHF
    VhfMa,  // VHF_MA
    Vortex,  // VORTEX
    Vwrsi,  // VWRSI
    WilliamsR,  // WILLIAMS_R

    // Signal Processing (84 indicators)
    Autocorr,  // AUTOCORR
    Butter,  // BUTTER
    Cheby,  // CHEBY
    Cusum,  // CUSUM
    Cyber,  // CYBER
    Decyc,  // DECYC
    Esine,  // ESINE
    Ess,  // ESS
    Fft,  // FFT
    Hampel,  // HAMPEL
    Hdc,  // HDC
    Hilb,  // HILB
    Hmom,  // HMOM
    Hyst,  // HYST
    Logicand,  // LOGICAND - AND Logic Gate
    Logicor,  // LOGICOR - OR Logic Gate
    Logicxor,  // LOGICXOR - XOR Logic Gate
    Logicsign,  // LOGICSIGN - Sign Combiner
    Lz,  // LZ
    Mrf,  // MRF
    Rc,  // RC
    Roof,  // ROOF
    Sbp,  // SBP
    Sbprhl,  // SBPRHL
    Sbwf,  // SBWF
    Scf,  // SCF
    Screst,  // SCREST
    Screstp,  // SCRESTP
    Sent,  // SENT
    Sentent,  // SENTENT
    Sentr,  // SENTR
    Ser,  // SER
    Sflat,  // SFLAT
    Sflatp,  // SFLATP
    Sflux,  // SFLUX
    Sg,  // SG
    Shmpr,  // SHMPR
    Slmpr,  // SLMPR
    Sroll,  // SROLL
    Sroll95,  // SROLL95
    Srollp,  // SROLLP
    Srollrp,  // SROLLRP
    Sslope,  // SSLOPE
    Sslopep,  // SSLOPEP
    Ssloperp,  // SSLOPERP
    Sslopez,  // SSLOPEZ
    Stft,  // STFT
    Tenc,  // TENC
    Thresh,  // THRESH
    Wave,  // WAVE
    Wcomp,  // WCOMP
    Zmad,  // ZMAD

    // Channels (72 indicators)
    Adaptivebb,  // ADAPTIVEBB
    Adaptivechan,  // ADAPTIVECHAN
    Atrchan,  // ATRCHAN
    Bb,  // BB
    Bbmetrics,  // BBMETRICS
    Darvas,  // DARVAS
    Dc,  // DC
    Dcmetrics,  // DCMETRICS
    Dcpos,  // DCPOS
    Dcwidth,  // DCWIDTH
    Dpobands,  // DPOBANDS
    Envbw,  // ENVBW
    Envelope,  // ENVELOPE
    Ichimoku,  // ICHIMOKU
    Ichimokupos,  // ICHIMOKUPOS
    Ichimokuthick,  // ICHIMOKUTHICK
    Kc,  // KC
    Kcmetrics,  // KCMETRICS
    Keltbw,  // KELTBW
    Keltdist,  // KELTDIST
    Keltpos,  // KELTPOS
    Medchan,  // MEDCHAN
    Medchanpos,  // MEDCHANPOS
    Pchosc,  // PCHOSC
    Pchwidth,  // PCHWIDTH
    Percentb,  // PERCENTB
    Percentilech,  // PERCENTILECH
    Pivotchan,  // PIVOTCHAN
    Pricechan,  // PRICECHAN
    Projbands,  // PROJBANDS
    Qrchan,  // QRCHAN
    Regchan,  // REGCHAN
    Regchanwidth,  // REGCHANWIDTH
    Starc,  // STARC
    Stddevchan,  // STDDEVCHAN
    Stddevwidth,  // STDDEVWIDTH
    Theilsenchan,  // THEILSENCHAN
    Trimabands,  // TRIMABANDS
    Volprofchan,  // VOLPROFCHAN
    Vwapchan,  // VWAPCHAN
    Vwapchanwidth,  // VWAPCHANWIDTH

    // Volatility (52 indicators)
    Abb,  // ABB
    Atr,  // ATR
    Atrbw,  // ATRBW
    Atrc,  // ATRC
    Atrp,  // ATRP
    Atrpt,  // ATRPT
    Atrz,  // ATRZ
    Avr,  // AVR
    Bpv,  // BPV
    C2cvp,  // C2CVP
    Chop,  // CHOP
    Cv,  // CV
    Dvr,  // DVR
    Fuzzy,  // FUZZY
    Har,  // HAR
    Hvc2c,  // HVC2C
    Kp,  // KP
    Natr,  // NATR
    Nr,  // NR
    Pgry,  // PGRY
    Rbvj,  // RBVJ
    Rcb,  // RCB
    Rp,  // RP
    Rq,  // RQ
    Rv,  // RV
    Rvi,  // RVI
    Rvz,  // RVZ
    Sqmom,  // SQMOM
    Tr,  // TR
    Ui,  // UI
    Vbd,  // VBD
    Vbexp,  // VBEXP
    VoDc,  // VO_DC
    VoKc,  // VO_KC
    VoMi,  // VO_MI
    VoVr,  // VO_VR
    Vov,  // VOV
    Vovp,  // VOVP
    Vovpt,  // VOVPT
    Vprb,  // VPRB
    Wvf,  // WVF

    // Levels (27 indicators)
    Avwap,  // AVWAP
    Avwaprev,  // AVWAPREV
    Avwaptouch,  // AVWAPTOUCH
    Bos,  // BOS
    Camarilla,  // CAMARILLA
    Demark,  // DEMARK
    Floorpivot,  // FLOORPIVOT
    Fvg,  // FVG
    Fvgalt,  // FVGALT
    Fvgdur,  // FVGDUR
    Fvgrev,  // FVGREV
    Hlva,  // HLVA
    Liqgap,  // LIQGAP
    Pivavwap,  // PIVAVWAP
    Pivot,  // PIVOT
    Rmid,  // RMID
    Rquart,  // RQUART
    Swingstr,  // SWINGSTR
    Woodie,  // WOODIE

    // Candles (4 indicators)
    Candleanatomy,  // CANDLEANATOMY
    Heikinashi,  // HEIKINASHI
    Wickspike,  // WICKSPIKE (StatisticalWickDetector)

    // Volume (27 indicators)
    Cvd,  // CVD — Cumulative Volume Delta (rolling)
    Mfi,  // MFI
    NviPvi,  // NVI_PVI
    Poc,  // POC
    Pvo,  // PVO
    Pvt,  // PVT
    Pzo,  // PZO
    Rvol,  // RVOL
    Rvp,  // RVP — Rolling Volume Profile (POC/VAH/VAL, Triple)
    SessionVwap,  // SESSION_VWAP
    Vdelta,  // VDELTA
    Vfi,  // VFI
    Vo,  // VO
    Vpin,  // VPIN
    Vprofile,  // VPROFILE
    Vpt,  // VPT
    Vroc,  // VROC
    Vz,  // VZ
    Vzo,  // VZO
    TradeFlowImbalance,  // TRADE_FLOW_IMBALANCE
    UptickDowntickVolume,  // UPTICK_DOWNTICK_VOLUME
    AggressorImbalance,  // AGGRESSOR_IMBALANCE
    LargeTradeFilter,  // LARGE_TRADE_FILTER

    // Trend (19 indicators)
    AdxSlope,  // ADX_SLOPE
    Didi,  // DIDI
    Eit,  // EIT
    GannHilo,  // GANN_HILO
    Gmma,  // GMMA
    HaTrend,  // HA_TREND
    KamaSlope,  // KAMA_SLOPE
    LrSlope,  // LR_SLOPE
    Ravi,  // RAVI
    Sdl,  // SDL
    Ssl,  // SSL
    Supertrend,  // SUPERTREND
    Tii,  // TII
    TrEr,  // TR_ER
    Zlsma,  // ZLSMA

    // Accumulation (20 indicators)
    Ad,  // AD
    Asi,  // ASI
    Cho,  // CHO
    Cmf,  // CMF
    Di,  // DI
    Eom,  // EOM
    Fi,  // FI
    Ii,  // II
    Iip,  // IIP
    Iir,  // IIR
    Obv,  // OBV
    Tmf,  // TMF
    Wad,  // WAD

    // Adaptive (7 indicators)
    Adaptivema,  // ADAPTIVEMA
    Frama,  // FRAMA
    Kama,  // KAMA
    Mama,  // MAMA
    Vidya,  // VIDYA

    // Entropy (20 indicators)
    Apen,  // APEN
    Conden,  // CONDEN
    Fisher,  // FISHER
    Infog,  // INFOG
    Jsd,  // JSD
    Kld,  // KLD
    Mi,  // MI
    Pe,  // PE
    Sampen,  // SAMPEN
    Shannon,  // SHANNON
    Te,  // TE
    Xmil,  // XMIL

    // Kalman (15 indicators)
    Abgfilter,  // ABGFILTER
    Ekf,  // EKF
    Kalman,  // KALMAN
    Kcomp,  // KCOMP
    Kregime,  // KREGIME
    Kscr,  // KSCR
    Kslope,  // KSLOPE
    Kslopez,  // KSLOPEZ
    Particle,  // PARTICLE
    Rts,  // RTS
    Ukf,  // UKF

    // Trend Stop (21 indicators)
    Atrts,  // ATRTS
    Chand,  // CHAND
    Cks,  // CKS
    Donbo,  // DONBO
    Dons,  // DONS
    Kelts,  // KELTS
    Psars,  // PSARS
    Supts,  // SUPTS
    TsSwings,  // TS_SWINGS
    Volts,  // VOLTS
    VoltsAtr,  // VOLTS_ATR

    // Chaos (18 indicators)
    Ac,  // AC
    Alligator,  // ALLIGATOR
    Ao,  // AO
    ChaosOsc,  // CHAOS_OSC
    Dfa,  // DFA
    DfaPct,  // DFA_PCT
    FractalDim,  // FRACTAL_DIM
    Fractals,  // FRACTALS
    Hurst,  // HURST
    HurstPct,  // HURST_PCT
    WilliamsMfi,  // WILLIAMS_MFI

    // Regression (7 indicators)
    Arima,  // ARIMA
    Arimax,  // ARIMAX
    Egarch,  // EGARCH
    Garch,  // GARCH
    PolyReg,  // POLY_REG
    Var,  // VAR

    // Ratio (4 indicators)
    Er,  // ER
    ErRing,  // ER_RING
    RangeAtr,  // RANGE_ATR
    SpreadAnalyzer,  // SPREAD_ANALYZER

    // Tick Advanced (8 indicators)
    VwapDeviation,                   // VWAP_DEVIATION
    TradeRunDetector,                // TRADE_RUN_DETECTOR
    SizeWeightedDirectionalMomentum, // SIZE_WEIGHTED_DIRECTIONAL_MOMENTUM
    TickFrequencyAnomaly,            // TICK_FREQUENCY_ANOMALY
    AggressorBurstDetector,          // AGGRESSOR_BURST_DETECTOR
    LargeTickMomentum,               // LARGE_TICK_MOMENTUM
    ValueAreaTracker,                // VALUE_AREA_TRACKER
    VolumeImbalanceZone,             // VOLUME_IMBALANCE_ZONE

    // Clusters (11 indicators)
    ClQueueImb,  // CL_QUEUE_IMB
    MarketMicro,  // MARKET_MICRO
    OrderBookSlope,  // ORDER_BOOK_SLOPE
    OrderFlowImb,  // ORDER_FLOW_IMB
    TickVolume,  // TICK_VOLUME
    VwapLevels,  // VWAP_LEVELS
    FootprintChart,  // FOOTPRINT_CHART
    FootprintImbalance,  // FOOTPRINT_IMBALANCE
    FootprintPoc,  // FOOTPRINT_POC
    AbsorptionDetector,  // ABSORPTION_DETECTOR
    TradeClusterDetector,  // TRADE_CLUSTER_DETECTOR

    // Position (19 indicators)
    AvwapDist,  // AVWAP_DIST
    Cpr,  // CPR
    DayWeekMonth,  // DAY_WEEK_MONTH
    DistLevels,  // DIST_LEVELS
    DomWoq,  // DOM_WOQ
    HolidayProx,  // HOLIDAY_PROX
    HourDay,  // HOUR_DAY
    MonthQtr,  // MONTH_QTR
    MonthTurn,  // MONTH_TURN
    QtrTurn,  // QTR_TURN
    RelTrendPos,  // REL_TREND_POS
    Session,  // SESSION
    SomEom,  // SOM_EOM
    SoqEoq,  // SOQ_EOQ
    SowEow,  // SOW_EOW
    VwapDist,  // VWAP_DIST
    WeekMonth,  // WEEK_MONTH
    Weekday,  // WEEKDAY

    // Statistics (29 indicators)
    Adf,  // ADF
    AdfKpss,  // ADF_KPSS
    ArchLm,  // ARCH_LM
    ArchLmPval,  // ARCH_LM_PVAL
    BpCusum,  // BP_CUSUM
    Coint,  // COINT
    EgAdf,  // EG_ADF
    EgCoint,  // EG_COINT
    EgTrend,  // EG_TREND
    HalfLifeMr,  // HALF_LIFE_MR
    Kpss,  // KPSS
    KpssTrend,  // KPSS_TREND
    KpssZ,  // KPSS_Z
    LjungBox,  // LJUNG_BOX
    Pacf,  // PACF
    Pp,  // PP
    PriceZscore,  // PRICE_ZSCORE
    PvCoherence,  // PV_COHERENCE
    RSquared,  // R_SQUARED
    ResidStat,  // RESID_STAT
    StCusum,  // ST_CUSUM
    Vr,  // VR
    VrAgg,  // VR_AGG
    VrZAgg,  // VR_Z_AGG
    Za,  // ZA

    // Book (11 indicators)
    BookImb,  // BOOK_IMB
    BookMicroprice,  // BOOK_MICROPRICE
    BookSlope,  // BOOK_SLOPE
    Ofi,  // OFI
    QueueImb,  // QUEUE_IMB
    LiquiditySweep,  // BOOK_LIQUIDITY_SWEEP
    BookPressure,  // BOOK_PRESSURE
    SpreadDistribution,  // BOOK_SPREAD_DIST
    OrderBookVelocity,  // BOOK_OBV
    WallDetector,  // WALL_DETECTOR
    BookDepthChange,  // BOOK_DEPTH_CHANGE

    // Book delta indicators (3 — consume OrderbookDelta)
    IcebergDetector,  // ICEBERG_DETECTOR
    LevelReplenishRate,  // LEVEL_REPLENISH_RATE
    BookChurnRate,  // BOOK_CHURN_RATE

    // Hybrid Tick+Book indicators (3 — consume Tick + OrderBook together)
    HiddenLiquidityDetector,  // HIDDEN_LIQUIDITY_DETECTOR
    TradeBookAbsorption,  // TRADE_BOOK_ABSORPTION
    SweepImpactAnalyzer,  // SWEEP_IMPACT_ANALYZER

    // Book Advanced (6 — consume OrderBook L2 snapshots)
    BidAskAsymmetry,      // BID_ASK_ASYMMETRY
    BidAskBounceRate,     // BID_ASK_BOUNCE_RATE
    MidPriceVelocity,     // MID_PRICE_VELOCITY
    BestLevelVolatility,  // BEST_LEVEL_VOLATILITY
    LayerConcentration,   // LAYER_CONCENTRATION
    PriceLevelDensity,    // PRICE_LEVEL_DENSITY

    // Funding / OI indicators (3 — consume FundingRate / OpenInterest)
    FundingMomentum,  // FUNDING_MOMENTUM
    FundingZScore,  // FUNDING_ZSCORE
    OiChangeRate,  // OI_CHANGE_RATE
    FundingPriceDivergence,  // FUNDING_PRICE_DIVERGENCE

    // Open Interest indicators (5 — consume OpenInterest / OpenInterest+MarkPrice)
    OiZScore,              // OI_Z_SCORE
    OiMomentum,            // OI_MOMENTUM
    OiPercentile,          // OI_PERCENTILE
    LongSqueezeDetector,   // LONG_SQUEEZE_DETECTOR
    OiPriceCorrelation,    // OI_PRICE_CORRELATION

    // MarkPrice indicators (2 — consume MarkPrice)
    MarkPriceVsLast,  // MARK_PRICE_VS_LAST
    IndexPriceMomentum,  // INDEX_PRICE_MOMENTUM

    // MarkPrice advanced (3 — consume MarkPrice)
    MarkPriceMomentum,           // MARK_PRICE_MOMENTUM
    MarkPriceVolatility,         // MARK_PRICE_VOLATILITY
    MarkPriceGapDetector,        // MARK_PRICE_GAP_DETECTOR

    // Ticker indicators (3 — consume Ticker / 24h stats)
    Volume24hMomentum,  // VOLUME_24H_MOMENTUM
    HighLowRangeRatio,  // HIGH_LOW_RANGE_RATIO
    PriceChange24hZScore,  // PRICE_CHANGE_24H_ZSCORE

    // Liquidation indicators (7 — consume Liquidation events)
    LiquidationRate,  // LIQUIDATION_RATE
    LiquidationVolumeImbalance,  // LIQUIDATION_VOLUME_IMBALANCE
    LiquidationCascade,  // LIQUIDATION_CASCADE
    LiquidationVolumeVelocity,  // LIQUIDATION_VOLUME_VELOCITY
    StopHuntDetector,  // STOP_HUNT_DETECTOR
    LiquidationClusterDetector,  // LIQUIDATION_CLUSTER_DETECTOR
    LiquidationCooldown,  // LIQUIDATION_COOLDOWN

    // Ohlcv Average (15 indicators) - REMOVED
    // All OHLCV variants replaced by MovingAverageWithField
    // Use: MovingAverageWithField::new(MovingAverageType, period, OhlcvField)

    // Sentiment indicators (5 — consume LongShortRatio / AggTrade streams)
    LongShortRatioMomentum,     // LONG_SHORT_RATIO_MOMENTUM
    LongShortExtremeDetector,   // LONG_SHORT_EXTREME_DETECTOR
    RatioVsPriceDivergence,     // RATIO_VS_PRICE_DIVERGENCE
    AggTradeFlowImbalance,      // AGG_TRADE_FLOW_IMBALANCE
    AggTradeSizeDistribution,   // AGG_TRADE_SIZE_DISTRIBUTION

    // Index/Basis indicators (6 — consume IndexPrice, CompositeIndex, Basis)
    PriceVsIndexSpread,          // PRICE_VS_INDEX_SPREAD
    IndexComponentDrift,         // INDEX_COMPONENT_DRIFT
    IndexCorrelationBreakdown,   // INDEX_CORRELATION_BREAKDOWN
    BasisMomentum,               // BASIS_MOMENTUM
    BasisExtreme,                // BASIS_EXTREME
    BasisZScore,                 // BASIS_Z_SCORE

    // Volatility advanced indicators (4 — consume HistoricalVolatility, VolatilityIndex)
    HvMomentum,                  // HV_MOMENTUM
    HvSpike,                     // HV_SPIKE
    VolIdxSpike,                 // VOL_IDX_SPIKE
    VolIdxMomentum,              // VOL_IDX_MOMENTUM

    // Greeks indicators (3 — consume OptionGreeks)
    DeltaExposureFlow,           // DELTA_EXPOSURE_FLOW
    GammaSqueezeDetector,        // GAMMA_SQUEEZE_DETECTOR
    IvSkew,                      // IV_SKEW

    // Greeks advanced (4 — consume OptionGreeks)
    CharmTracker,                // CHARM_TRACKER
    VegaExposureFlow,            // VEGA_EXPOSURE_FLOW
    ThetaDecayTracker,           // THETA_DECAY_TRACKER
    PinRiskDetector,             // PIN_RISK_DETECTOR

    // Stress indicators (6 — consume InsuranceFund / SettlementEvent / MarkPrice)
    FundDepletionRate,           // FUND_DEPLETION_RATE
    FundStressDetector,          // FUND_STRESS_DETECTOR
    InsuranceFundMomentum,       // INSURANCE_FUND_MOMENTUM
    SettlementApproachSignal,    // SETTLEMENT_APPROACH_SIGNAL
    SettlementPriceMomentum,     // SETTLEMENT_PRICE_MOMENTUM
    SettlementVsMarkSpread,      // SETTLEMENT_VS_MARK_SPREAD

    // Microstructure indicators (9 — consume BlockTrade / OrderbookL3 / OrderbookDelta)
    BlockTradeFlow,              // BLOCK_TRADE_FLOW
    BlockTradeImpact,            // BLOCK_TRADE_IMPACT
    L3OrderRate,                 // L3_ORDER_RATE
    L3LargeOrderTracker,         // L3_LARGE_ORDER_TRACKER
    L3CancelRatio,               // L3_CANCEL_RATIO
    BlockTradeSizeAnomaly,       // BLOCK_TRADE_SIZE_ANOMALY
    QuoteStuffingDetector,       // QUOTE_STUFFING_DETECTOR
    L3SpooferScore,              // L3_SPOOFER_SCORE
    QuoteLifecycleTracker,       // QUOTE_LIFECYCLE_TRACKER

    // Risk indicators (3 — consume RiskLimit)
    LeverageReductionWarning,    // LEVERAGE_REDUCTION_WARNING
    MmrTracker,                  // MMR_TRACKER
    RiskLimitProximity,          // RISK_LIMIT_PROXIMITY

    // Funding indicators (5 — consume PredictedFunding / FundingRate / FundingSettlement / MarkPrice)
    FundingDrift,                // FUNDING_DRIFT
    FundingTimeDecay,            // FUNDING_TIME_DECAY
    PredictedFundingExtreme,     // PREDICTED_FUNDING_EXTREME
    SettledFundingMomentum,      // SETTLED_FUNDING_MOMENTUM
    FundingSettlementImpact,     // FUNDING_SETTLEMENT_IMPACT

    // Funding advanced (3 — consume FundingRate)
    AnnualizedFundingRate,       // ANNUALIZED_FUNDING_RATE
    FundingDirectionShift,       // FUNDING_DIRECTION_SHIFT
    FundingExtremeAlert,         // FUNDING_EXTREME_ALERT

    // Misc indicators (5 — consume AuctionEvent / MarketWarning)
    AuctionLiquidityScore,       // AUCTION_LIQUIDITY_SCORE
    AuctionPriceDeviation,       // AUCTION_PRICE_DEVIATION
    AuctionImbalance,            // AUCTION_IMBALANCE
    WarningRate,                 // WARNING_RATE
    WarningFrequencyFilter,      // WARNING_FREQUENCY_FILTER

    // Ticker advanced indicators (2 — consume Ticker)
    TickerSpreadRatio,           // TICKER_SPREAD_RATIO
    Volume24hZScore,             // VOLUME_24H_Z_SCORE

    // Cross-stream composite indicators (8 — consume 2-3 streams simultaneously)
    FundingOiPressure,           // FUNDING_OI_PRESSURE (FundingRate + OI)
    IvHvSpread,                  // IV_HV_SPREAD (VolatilityIndex + HistoricalVolatility)
    SqueezeProbability,          // SQUEEZE_PROBABILITY (OI + MarkPrice + Liquidation)
    FundingSentimentAlignment,   // FUNDING_SENTIMENT_ALIGNMENT (FundingRate + LongShortRatio)
    VolRegimeEntry,              // VOL_REGIME_ENTRY (VolatilityIndex + MarkPrice)
    BlockTradeVolumeRatio,       // BLOCK_TRADE_VOLUME_RATIO (BlockTrade + AggTrade)
    CapitulationDetector,        // CAPITULATION_DETECTOR (Liquidation + AggTrade + MarkPrice)
    IndexTrackingError,          // INDEX_TRACKING_ERROR (IndexPrice + CompositeIndex)

    // Category C composite + adaptive + cross-asset indicators (11)
    MarketStressComposite,       // MARKET_STRESS_COMPOSITE (VolIdx + Liq + Funding + InsuranceFund)
    RiskOffDetector,             // RISK_OFF_DETECTOR (VolIdx + Liq + Funding + InsuranceFund)
    SentimentComposite,          // SENTIMENT_COMPOSITE (LongShortRatio + AggTrade + Funding)
    CompoundSqueezeProbability,  // COMPOUND_SQUEEZE_PROBABILITY (OI + Liq + MarkPrice + Funding)
    TpoSessionBalance,           // TPO_SESSION_BALANCE (Tick)
    CompositeWeightDrift,        // COMPOSITE_WEIGHT_DRIFT (CompositeIndex)
    AdaptiveWindowSelector,      // ADAPTIVE_WINDOW_SELECTOR (Tick)
    AdaptiveThreshold,           // ADAPTIVE_THRESHOLD (Tick)
    PairsCointegrationProxy,     // PAIRS_COINTEGRATION_PROXY (Tick + secondary price)
    CrossAssetBeta,              // CROSS_ASSET_BETA (Tick + secondary price)
    RelativeStrengthCross,       // RELATIVE_STRENGTH_CROSS (Tick + secondary price)

}

impl BarIndicatorId {
    /// True if this indicator accepts a `MovingAverageType` parameter
    /// (i.e. its smoother is configurable). Used by the optimizer to decide
    /// whether to expand `ma_type` as an axis in the parameter cube.
    pub fn supports_ma_type(self) -> bool {
        matches!(
            self,
            BarIndicatorId::Atr
                | BarIndicatorId::Atrbw
                | BarIndicatorId::Atrp
                | BarIndicatorId::Atrz
                | BarIndicatorId::AvVidya
                | BarIndicatorId::BbPeriod
                | BarIndicatorId::MacdHistZ
                | BarIndicatorId::Pressure
                | BarIndicatorId::RangeAtr
                | BarIndicatorId::SweepRev
                | BarIndicatorId::Tmf
                | BarIndicatorId::VhfMa
                | BarIndicatorId::Volts
                | BarIndicatorId::VoltsAtr
        )
    }
}


// Original variants: 487
// Added long aliases: 155
// Total: 637
