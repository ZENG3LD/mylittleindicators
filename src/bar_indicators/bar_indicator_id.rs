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
    AutoFibo,  // AUTO_FIBO
    BbPeriod,  // BB_PERIOD
    Bias,  // BIAS
    Bop,  // BOP
    CandlePatterns,  // CANDLE_PATTERNS
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
    LaguerreRsi,  // LAGUERRE_RSI
    Lowest,  // LOWEST
    MaCross,  // MA_CROSS
    Macd,  // MACD
    MacdHist,  // MACD_HIST
    MacdHistZ,  // MACD_HIST_Z
    MacdSignal,  // MACD_SIGNAL
    MarketCipher,  // MARKET_CIPHER
    MoFisher,  // MO_FISHER
    MoObv,  // MO_OBV
    MomZscore,  // MOM_ZSCORE
    MtfMomDiv,  // MTF_MOM_DIV
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
    Swings,  // SWINGS
    SwingsSoft,  // SWINGS_SOFT
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
    Zigzag,  // ZIGZAG

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
    Rc2,  // RC2
    Rc3,  // RC3
    Rc4,  // RC4
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
    Fibochan,  // FIBOCHAN
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

    // Candles (19 indicators)
    Candleanatomy,  // CANDLEANATOMY
    Darkcloudcover,  // DARKCLOUDCOVER
    Doji,  // DOJI
    Engulfing,  // ENGULFING
    Eveningstar,  // EVENINGSTAR
    Hammer,  // HAMMER
    Harami,  // HARAMI
    Heikinashi,  // HEIKINASHI
    Marubozu,  // MARUBOZU
    Morningstar,  // MORNINGSTAR
    Patternrec,  // PATTERNREC
    Piercingpattern,  // PIERCINGPATTERN
    Sfp,  // SFP
    Shootingstar,  // SHOOTINGSTAR
    Threeblackcrows,  // THREEBLACKCROWS
    Threewhitesoldiers,  // THREEWHITESOLDIERS
    Tweezer,  // TWEEZER
    Wickspike,  // WICKSPIKE

    // Volume (22 indicators)
    Mfi,  // MFI
    NviPvi,  // NVI_PVI
    Poc,  // POC
    Pvo,  // PVO
    Pvt,  // PVT
    Pzo,  // PZO
    Rvol,  // RVOL
    Trin,  // TRIN
    Vdelta,  // VDELTA
    Vfi,  // VFI
    Vo,  // VO
    Vpin,  // VPIN
    Vprofile,  // VPROFILE
    Vpt,  // VPT
    Vroc,  // VROC
    Vz,  // VZ
    Vzo,  // VZO

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

    // Divergence (13 indicators)
    CciDiv,  // CCI_DIV
    ClassicDiv,  // CLASSIC_DIV
    DivStrength,  // DIV_STRENGTH
    HiddenDiv,  // HIDDEN_DIV
    MacdDiv,  // MACD_DIV
    MacdHistDiv,  // MACD_HIST_DIV
    MultiDiv,  // MULTI_DIV
    ObvDiv,  // OBV_DIV
    RsiDiv,  // RSI_DIV
    StochDiv,  // STOCH_DIV
    VolDiv,  // VOL_DIV
    WilliamsDiv,  // WILLIAMS_DIV
    ZigzagDiv,  // ZIGZAG_DIV

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

    // Clusters (6 indicators)
    ClQueueImb,  // CL_QUEUE_IMB
    MarketMicro,  // MARKET_MICRO
    OrderBookSlope,  // ORDER_BOOK_SLOPE
    OrderFlowImb,  // ORDER_FLOW_IMB
    TickVolume,  // TICK_VOLUME
    VwapLevels,  // VWAP_LEVELS

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

    // Book (4 indicators)
    BookImb,  // BOOK_IMB
    BookSlope,  // BOOK_SLOPE
    Ofi,  // OFI
    QueueImb,  // QUEUE_IMB

    // Ohlcv Average (15 indicators) - REMOVED
    // All OHLCV variants replaced by MovingAverageWithField
    // Use: MovingAverageWithField::new(MovingAverageType, period, OhlcvField)

    // Zigzag (5 indicators)
    ZigzagAtr,  // ZIGZAG_ATR
    ZigzagCandle,  // ZIGZAG_CANDLE
    ZigzagClassic,  // ZIGZAG_CLASSIC
    ZigzagLookahead,  // ZIGZAG_LOOKAHEAD
    ZigzagTime,  // ZIGZAG_TIME

}

// Original variants: 482
// Added long aliases: 155
// Total: 637
