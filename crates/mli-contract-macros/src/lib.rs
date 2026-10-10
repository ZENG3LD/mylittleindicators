//! `contract_universe!` — the single declarative source the MLI contract engine
//! is generated from.
//!
//! ```ignore
//! contract_universe! {
//!     members {
//!         Sma:  Field    : MovingAverage => crate::indicators::average::sma::Sma,
//!         Kama: Field    : MovingAverage => crate::indicators::average::kaufman_adaptive_ma::KaufmanAdaptiveMA,
//!         Bb:   Field    : Channel       => crate::indicators::channels::bollinger_bands::BollingerBands,
//!         ...
//!     }
//!     slot_runtimes {
//!         MovingAverage => MovingAverageRuntime,
//!         // register a family here when a consumer slots it — its box-free runtime enum appears
//!     }
//! }
//! ```
//!
//! From the ONE `members` list it emits, all in lockstep (no frontier can drift):
//! - `ContractFactory` — the box-free build+run enum (`build`/`feed`/`value`/…);
//! - `CatalogMeta` + the `id -> contract` projections (`family_of` … `needs_volume_of`);
//! - one narrowed box-free **slot-runtime enum** per `slot_runtimes` entry —
//!   exactly the members of that `Family`, the inline non-recursive container an
//!   indicator holds for a configurable inner producer (an MA smoother, …).
//!
//! `flavor` (`Field` / `Fields` / `Time` / `OrderBook` / `Liquidation` /
//! `OpenInterest`) selects HOW the factory feeds the core (or `Multi[A, B, …]` for N streams).
//! `feed` itself takes `(ts, sample)` — `ts` (canonical ms) is the THIRD axis, forwarded to a
//! core ONLY for `Time` / `+time` members (calendar/anchor cores), dropped for the rest. `Field` is the
//! GENERAL pure-core scalar path: the variant carries the input field and the factory
//! resolves it from any `MarketSample` (a kline field today; book / OI / liquidation /
//! delta by-the-fact) and feeds ONE scalar — NOT bar-specific, the core knows no
//! transport. `Fields` is its K>=2 sibling (the core indexes its own declared lanes — the
//! whole candle is just `Fields([O,H,L,C,V])`). The legacy `Bar`/`update_bar` path is GONE.
//! `Multi[A, B, …]` is the N-stream member: the factory routes each declared stream to its
//! `*Consumer::update_*` natively (no frame, no `MultiStreamConsumer`); value via `indicator_*`.
//! `family` is a
//! TOKEN (the macro cannot read the `FAMILY` const); a generated test asserts the
//! token equals the type's `FAMILY[0]`, so it cannot drift.

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{braced, parenthesized, Ident, Path, Token};

/// One contracted indicator: `Variant : Flavor : Family [+flag…] => ConcreteType`.
/// `Family` is a `Family` variant ident — or `_` for an atomic producer that belongs
/// to NO family (consumed via a `Port`, never slotted; e.g. Highest / Lowest). Each
/// `+flag` tags the member into a flagged id-subset (e.g. `+smoother` → it appears in
/// the generated `SmootherId` — the typed field type for a smoother slot, so the
/// compiler admits only flagged members there).
struct Member {
    variant: Ident,
    flavor: Ident,
    /// For a multi-stream member (`flavor == Multi`): the N declared stream flavors,
    /// e.g. `Multi[OpenInterest, MarkPrice]`. Empty for every single-flavor member. The
    /// factory routes EACH stream's sample to its `update_*` consumer method natively —
    /// no synchronized frame, no `MultiStreamConsumer`. `value`/`is_ready`/`reset` come
    /// from the indicator's inherent `indicator_value`/`indicator_is_ready`/`indicator_reset`
    /// (the per-`*Consumer`-trait methods are ambiguous when N traits are impl'd).
    streams: Vec<Ident>,
    family: Option<Ident>,
    flags: Vec<Ident>,
    /// Pascal variant of [`crate::contract::CubeFormula`] when the member is
    /// `+cube(name)`. The name in the manifest is snake_case (`window_mean`).
    cube_formula: Option<Ident>,
    ty: Path,
    outputs: Vec<Ident>,
    /// Brace entries prefixed with `#` — TABLE outputs (read whole as a `MatrixGrid` via the
    /// generated `grid(id)`, not as an f64). They share the ONE `IndicatorOutputId` space with
    /// the scalar outputs; shape is the only difference.
    matrix_outputs: Vec<Ident>,
}

impl Parse for Member {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let variant: Ident = input.parse()?;
        input.parse::<Token![:]>()?;
        let flavor: Ident = input.parse()?;
        // `Multi[A, B, …]` — N stream flavors routed natively, one `update_*` each.
        let mut streams = Vec::new();
        if input.peek(syn::token::Bracket) {
            let content;
            syn::bracketed!(content in input);
            let list: Punctuated<Ident, Token![,]> =
                content.parse_terminated(Ident::parse, Token![,])?;
            streams = list.into_iter().collect();
        }
        input.parse::<Token![:]>()?;
        let family: Option<Ident> = if input.peek(Token![_]) {
            input.parse::<Token![_]>()?;
            None
        } else {
            Some(input.parse()?)
        };
        let mut flags = Vec::new();
        let mut cube_formula = None;
        while input.peek(Token![+]) {
            input.parse::<Token![+]>()?;
            let name: Ident = input.parse()?;
            if name == "cube" {
                if !input.peek(syn::token::Paren) {
                    return Err(syn::Error::new(
                        name.span(),
                        "+cube requires a formula, for example +cube(window_mean)",
                    ));
                }
                let content;
                parenthesized!(content in input);
                let formula: Ident = content.parse()?;
                cube_formula = Some(cube_formula_variant(&formula)?);
            }
            flags.push(name);
        }
        input.parse::<Token![=>]>()?;
        let ty: Path = input.parse()?;
        let mut outputs = Vec::new();
        let mut matrix_outputs = Vec::new();
        if input.peek(syn::token::Brace) {
            let content;
            braced!(content in input);
            while !content.is_empty() {
                // `#name` marks a TABLE output (whole-grid, read via `grid(id)`); a bare ident is
                // a scalar output. Both land in the one `IndicatorOutputId` enum.
                if content.peek(Token![#]) {
                    content.parse::<Token![#]>()?;
                    matrix_outputs.push(content.parse::<Ident>()?);
                } else {
                    outputs.push(content.parse::<Ident>()?);
                }
            }
        }
        // A member with no scalar outputs (only `#tables`, or an absent brace) still has the
        // implicit single `value` scalar.
        if outputs.is_empty() {
            outputs.push(Ident::new("value", Span::call_site()));
        }
        Ok(Member {
            variant,
            flavor,
            streams,
            family,
            flags,
            cube_formula,
            ty,
            outputs,
            matrix_outputs,
        })
    }
}

/// Manifest names the formula in snake_case. The generated match uses the
/// Pascal variant of `CubeFormula`.
fn cube_formula_variant(name: &Ident) -> syn::Result<Ident> {
    let mapped = match name.to_string().as_str() {
        "identity" => "Identity",
        "window_mean" => "WindowMean",
        "window_max" => "WindowMax",
        "window_min" => "WindowMin",
        "window_weighted" => "WindowWeighted",
        "ema" => "Ema",
        "rma" => "Rma",
        "dema" => "Dema",
        "tema" => "Tema",
        "tma" => "Tma",
        "hma" => "Hma",
        "alma" => "Alma",
        "t3" => "T3",
        "mcginley" => "Mcginley",
        "roc" => "Roc",
        "rsi" => "Rsi",
        "cmo" => "Cmo",
        "bias" => "Bias",
        "true_range" => "TrueRange",
        "atr" => "Atr",
        "bop" => "Bop",
        "vwma" => "Vwma",
        "macd" => "Macd",
        "apo" => "Apo",
        "microprice" => "Microprice",
        "book_imbalance" => "BookImbalance",
        "book_pressure" => "BookPressure",
        "endpoint_slope" => "EndpointSlope",
        "pop_zscore" => "PopZScore",
        "scale" => "Scale",
        "pop_std" => "PopStd",
        "percentile_rank" => "PercentileRank",
        "ratio_to_mean" => "RatioToMean",
        "ema_step" => "EmaStep",
        "book_slope" => "BookSlope",
        "williams_r" => "WilliamsR",
        "obv" => "Obv",
        "pvt" => "Pvt",
        "mfi" => "Mfi",
        "ad_line" => "AdLine",
        "demarker" => "Demarker",
        "ulcer" => "Ulcer",
        "realized_vol" => "RealizedVol",
        "efficiency" => "Efficiency",
        "gapo" => "Gapo",
        "wvf" => "Wvf",
        "quarticity" => "Quarticity",
        "hv_c2c" => "HvC2c",
        "psl" => "Psl",
        "imi" => "Imi",
        "pzo" => "Pzo",
        "cog" => "Cog",
        "bipower" => "Bipower",
        "vhf" => "Vhf",
        "pfe" => "Pfe",
        "volume_z" => "VolumeZ",
        "mom_z" => "MomZ",
        "percent_b" => "PercentB",
        "cfo" => "Cfo",
        "rmid" => "Rmid",
        "wad" => "Wad",
        "mad_z" => "MadZ",
        "cmf" => "Cmf",
        "vwap" => "Vwap",
        "rsx" => "Rsx",
        "asi" => "Asi",
        "var" => "Var",
        "chop" => "Chop",
        "ao" => "Ao",
        "dpo_pct" => "DpoPct",
        "envbw" => "Envbw",
        "ac" => "Ac",
        "williams_mfi" => "WilliamsMfi",
        "vfi" => "Vfi",
        "vzo" => "Vzo",
        "intraday_pct" => "IntradayPct",
        "intraday_ratio" => "IntradayRatio",
        "donchian_pos" => "DonchianPos",
        "donchian_width" => "DonchianWidth",
        "price_channel_osc" => "PriceChannelOsc",
        "price_channel_width" => "PriceChannelWidth",
        "er_full" => "ErFull",
        "er_ring" => "ErRing",
        "r_squared" => "RSquared",
        "vwap_distance" => "VwapDistance",
        "cyber_cycle" => "CyberCycle",
        "ama" => "Ama",
        "vol_break" => "VolBreak",
        "autocorr" => "Autocorr",
        "variance_ratio" => "VarianceRatio",
        "donchian_bands" => "DonchianBands",
        "donchian_metrics" => "DonchianMetrics",
        "aroon_cols" => "AroonCols",
        "central_pivot_range" => "CentralPivotRange",
        "heikin_ashi_cols" => "HeikinAshiCols",
        "candle_anatomy_cols" => "CandleAnatomyCols",
        "qstick_smoothed" => "QstickSmoothed",
        "force_index_smoothed" => "ForceIndexSmoothed",
        "coppock_smoothed" => "CoppockSmoothed",
        "volume_osc_smoothed" => "VolumeOscSmoothed",
        "chaikin_osc_smoothed" => "ChaikinOscSmoothed",
        "intraday_intensity_smoothed" => "IntradayIntensitySmoothed",
        "ease_of_movement_smoothed" => "EaseOfMovementSmoothed",
        "natr_smoothed" => "NatrSmoothed",
        "vroc" => "Vroc",
        "donchian_breakout" => "DonchianBreakout",
        "heikin_ashi_trend" => "HeikinAshiTrend",
        "weekday_effect" => "WeekdayEffect",
        "session_effect" => "SessionEffect",
        "month_effect" => "MonthEffect",
        "day_of_month_effect" => "DayOfMonthEffect",
        "hampel" => "Hampel",
        "psar" => "Psar",
        "supertrend" => "Supertrend",
        "adx" => "Adx",
        "adx_slope" => "AdxSlope",
        "cusum" => "Cusum",
        "har" => "Har",
        "rbvj" => "Rbvj",
        "vol_of_vol" => "VolOfVol",
        "ehlers_cc" => "EhlersCc",
        "vortex" => "Vortex",
        "dm" => "Dm",
        "di_plus_minus" => "DiPlusMinus",
        "rwi" => "Rwi",
        "higher_moments" => "HigherMoments",
        "swing_age" => "SwingAge",
        "ewmac" => "Ewmac",
        "gator" => "Gator",
        "ravi" => "Ravi",
        "tmf" => "Tmf",
        "vol_ratio" => "VolRatio",
        "range_atr" => "RangeAtr",
        "kelt_bw" => "KeltBw",
        "kelt_dist" => "KeltDist",
        "kelt_pos" => "KeltPos",
        "atr_sm" => "AtrSm",
        "hl_range" => "HlRange",
        "abs_log_ret" => "AbsLogRet",
        "hl2" => "Hl2",
        "atr_pct" => "AtrPct",
        "atr_pct_trend" => "AtrPctTrend",
        "atr_z" => "AtrZ",
        "vov_pct" => "VovPct",
        "vov_pct_trend" => "VovPctTrend",
        "rsi_pct_rank" => "RsiPctRank",
        "roll_quart" => "RollQuart",
        "comp_hilb_bar" => "HilbBar",
        "comp_hdc_bar" => "HdcBar",
        "comp_mama_bar" => "MamaBar",
        "comp_ehlersz_bar" => "EhlerszBar",
        "comp_uo_bar" => "UoBar",
        "comp_nr_bar" => "NrBar",
        "comp_eit_bar" => "EitBar",
        "comp_pgry_bar" => "PgryBar",
        "comp_rcb_bar" => "RcbBar",
        "comp_pv_coherence_bar" => "PvCoherenceBar",
        "comp_di_bar" => "DiBar",
        "comp_kregime_bar" => "KregimeBar",
        "comp_amat_bar" => "AmatBar",
        "comp_elder_impulse_bar" => "ElderImpulseBar",
        "comp_uo_smooth_bar" => "UoSmoothBar",
        "comp_atr_rsi_bar" => "AtrRsiBar",
        "comp_vwrsi_bar" => "VwrsiBar",
        "comp_qqe_bar" => "QqeBar",
        "comp_sqmom_bar" => "SqmomBar",
        "comp_pivot_bar" => "PivotBar",
        "comp_floorpivot_bar" => "FloorpivotBar",
        "comp_camarilla_bar" => "CamarillaBar",
        "comp_woodie_bar" => "WoodieBar",
        "comp_demark_bar" => "DemarkBar",
        "comp_alligator_bar" => "AlligatorBar",
        "comp_pivotchan_bar" => "PivotchanBar",
        "comp_theilsenchan_bar" => "TheilsenchanBar",
        "comp_avwap_bar" => "AvwapBar",
        "comp_avwap_dist_bar" => "AvwapDistBar",
        "comp_avwap_mrev_bar" => "AvwapMrevBar",
        "comp_avwap_tprob_bar" => "AvwapTprobBar",
        "comp_mo_fisher_bar" => "MoFisherBar",
        "comp_rel_trend_pos_bar" => "RelTrendPosBar",
        "comp_sweep_rev_bar" => "SweepRevBar",
        "comp_bp_cusum_bar" => "BpCusumBar",
        "comp_vr_agg_bar" => "VrAggBar",
        "comp_vr_z_agg_bar" => "VrZAggBar",
        "comp_fvgdur_bar" => "FvgdurBar",
        "comp_fvgalt_bar" => "FvgaltBar",
        "comp_fvgrev_bar" => "FvgrevBar",
        "comp_wickspike_bar" => "WickspikeBar",
        "comp_ts_swings_bar" => "TsSwingsBar",
        "comp_connors_rsi_bar" => "ConnorsRsiBar",
        "comp_adaptive_stoch_bar" => "AdaptiveStochBar",
        "comp_pivavwap_bar" => "PivavwapBar",
        "comp_swingstr_bar" => "SwingstrBar",
        "comp_liqgap_bar" => "LiqgapBar",
        "comp_fft_bar" => "FftBar",
        "comp_wave_bar" => "WaveBar",
        "comp_minfo_bar" => "MinfoBar",
        "comp_te_bar" => "TeBar",
        "comp_fuzzy_bar" => "FuzzyBar",
        "comp_oscvolw_bar" => "OscVolWeightBar",
        "comp_confluence_bar" => "ConfluenceBar",
        "comp_xmil_bar" => "XmilBar",
        "comp_kcomp_bar" => "KcompBar",
        "comp_rc_bar" => "RcBar",
        "comp_chaososc_bar" => "ChaosOscBar",
        "comp_candle_pattern_bar" => "CandlePatternBar",
        "comp_divergence_bar" => "DivergenceBar",
        "comp_ekf_bar" => "EkfBar",
        "comp_ukf_bar" => "UkfBar",
        "comp_vbd_bar" => "VbdBar",
        "comp_mrf_bar" => "MrfBar",
        "comp_avr_bar" => "AvrBar",
        "comp_dvr_bar" => "DvrBar",
        "comp_swing_bar" => "SwingBar",
        "comp_rvp_bar" => "RvpBar",
        "comp_vprofile_bar" => "VprofileBar",
        "comp_poc_bar" => "PocBar",
        "comp_vpc_bar" => "VolprofchanBar",
        "comp_vwapl_bar" => "VwapLevelsBar",
        "comp_esine_bar" => "EsineBar",
        "comp_ess_bar" => "EssBar",
        "comp_ehlersfa_bar" => "EhlersfaBar",
        "comp_rvol_bar" => "RvolBar",
        "comp_session_vwap_bar" => "SessionVwapBar",
        "comp_frama_bar" => "FramaBar",
        "comp_roc_pct_bar" => "RocPctBar",
        "comp_rvz_bar" => "RvzBar",
        "comp_kama_bar" => "KamaBar",
        "comp_kama_slope_bar" => "KamaSlopeBar",
        "comp_jma_bar" => "JmaBar",
        "comp_vidya_bar" => "VidyaBar",
        "comp_ehlers_rocket_bar" => "EhlersRocketBar",
        "comp_supts_bar" => "SuptsBar",
        "comp_kelts_bar" => "KeltsBar",
        "comp_nvi_pvi_bar" => "NviPviBar",
        "comp_gmma_bar" => "GmmaBar",
        "comp_ewmac_robust_bar" => "EwmacRobustBar",
        "comp_tdi_bar" => "TdiBar",
        "comp_dss_bar" => "DssBar",
        "comp_smi_bar" => "SmiBar",
        "comp_stc_bar" => "StcBar",
        "comp_ift_rsi_bar" => "IftRsiBar",
        "comp_rsi_zscore_bar" => "RsiZscoreBar",
        "comp_vhf_ma_bar" => "VhfMaBar",
        "comp_stoch_rsi_bar" => "StochRsiBar",
        "comp_eg_adf_bar" => "EgAdfBar",
        "comp_coint_bar" => "CointBar",
        "comp_eg_coint_bar" => "EgCointBar",
        "comp_adf_kpss_bar" => "AdfKpssBar",
        "comp_kpss_z_bar" => "KpssZBar",
        "comp_arch_lm_bar" => "ArchLmBar",
        "comp_arch_lm_pval_bar" => "ArchLmPvalBar",
        "comp_adf_bar" => "AdfBar",
        "comp_pp_bar" => "PpBar",
        "comp_za_bar" => "ZaBar",
        "comp_eg_trend_bar" => "EgTrendBar",
        "comp_kpss_bar" => "KpssBar",
        "comp_kpss_trend_bar" => "KpssTrendBar",
        "comp_hurst_bar" => "HurstBar",
        "comp_fractal_dim_bar" => "FractalDimBar",
        "comp_dfa_bar" => "DfaBar",
        "comp_hurst_pct_bar" => "HurstPctBar",
        "comp_dfa_pct_bar" => "DfaPctBar",
        "comp_ljung_box_bar" => "LjungBoxBar",
        "comp_pacf_bar" => "PacfBar",
        "comp_half_life_bar" => "HalfLifeBar",
        "comp_resid_stat_bar" => "ResidStatBar",
        "comp_jsd_bar" => "JsdBar",
        "comp_kld_bar" => "KldBar",
        "comp_lz_bar" => "LzBar",
        "comp_apen_bar" => "ApenBar",
        "comp_sampen_bar" => "SampenBar",
        "comp_perm_ent_bar" => "PermEntBar",
        "comp_con_den_bar" => "ConDenBar",
        "comp_shannon_bar" => "ShannonBar",
        "comp_fisher_info_bar" => "FisherInfoBar",
        "comp_info_gain_bar" => "InfoGainBar",
        "comp_dist_levels_bar" => "DistLevelsBar",
        "comp_med_chan_bar" => "MedChanBar",
        "comp_med_chan_pos_bar" => "MedChanPosBar",
        "comp_dpo_bands_bar" => "DpoBandsBar",
        "comp_volts_bar" => "VoltsBar",
        "comp_ichimoku_bar" => "IchimokuBar",
        "comp_ichimoku_pos_bar" => "IchimokuPosBar",
        "comp_ichimoku_thick_bar" => "IchimokuThickBar",
        "comp_vwap_chan_bar" => "VwapChanBar",
        "comp_vwap_chan_width_bar" => "VwapChanWidthBar",
        "comp_vprb_bar" => "VprbBar",
        "comp_price_chan_bar" => "PriceChanBar",
        "comp_darvas_bar" => "DarvasBar",
        "comp_qr_chan_bar" => "QrChanBar",
        "comp_stoch_kd_bar" => "StochKdBar",
        "comp_donchian_stop_bar" => "DonchianStopBar",
        "comp_proj_bands_bar" => "ProjBandsBar",
        "comp_compound_squeeze_mg" => "CompoundSqueezeMg",
        "comp_capitulation_mg" => "CapitulationMg",
        "comp_block_trade_ratio_mg" => "BlockTradeRatioMg",
        "comp_stop_hunt_mg" => "StopHuntMg",
        "comp_ratio_vs_price_mg" => "RatioVsPriceMg",
        "comp_funding_settle_impact_mg" => "FundingSettleImpactMg",
        "comp_risk_off_mg" => "RiskOffMg",
        "comp_market_stress_mg" => "MarketStressMg",
        "comp_sentiment_comp_mg" => "SentimentCompMg",
        "comp_oi_price_corr_mg" => "OiPriceCorrMg",
        "comp_price_vs_index_mg" => "PriceVsIndexMg",
        "comp_vol_regime_entry_mg" => "VolRegimeEntryMg",
        "comp_settle_vs_mark_mg" => "SettleVsMarkMg",
        "comp_squeeze_prob_mg" => "SqueezeProbMg",
        "comp_funding_drift_mg" => "FundingDriftMg",
        "comp_funding_oi_pressure_mg" => "FundingOiPressureMg",
        "comp_funding_price_div_mg" => "FundingPriceDivMg",
        "comp_funding_sentiment_mg" => "FundingSentimentMg",
        "comp_iv_hv_spread_mg" => "IvHvSpreadMg",
        "comp_long_squeeze_mg" => "LongSqueezeMg",
        "comp_mark_vs_last_mg" => "MarkVsLastMg",
        "comp_index_tracking_mg" => "IndexTrackingMg",
        "comp_book_churn_ev" => "BookChurnEv",
        "comp_level_replenish_ev" => "LevelReplenishEv",
        "comp_quote_stuffing_ev" => "QuoteStuffingEv",
        "comp_basis_extreme_ev" => "BasisExtremeEv",
        "comp_liq_cluster_ev" => "LiqClusterEv",
        "comp_large_trade_filter_ev" => "LargeTradeFilterEv",
        "comp_agg_size_dist_ev" => "AggSizeDistEv",
        "comp_spread_distribution_bk" => "SpreadDistributionBk",
        "comp_layer_concentration_bk" => "LayerConcentrationBk",
        "comp_order_book_velocity_bk" => "OrderBookVelocityBk",
        "comp_tick_volume_ev" => "TickVolumeEv",
        "comp_trade_flow_imb_ev" => "TradeFlowImbEv",
        "comp_up_down_tick_vol_ev" => "UpDownTickVolEv",
        "comp_funding_extreme_ev" => "FundingExtremeEv",
        "comp_funding_mom_ev" => "FundingMomEv",
        "comp_index_price_mom_ev" => "IndexPriceMomEv",
        "comp_mark_gap_ev" => "MarkGapEv",
        "comp_adaptive_threshold_ev" => "AdaptiveThresholdEv",
        "comp_bid_ask_bounce_bk" => "BidAskBounceBk",
        "comp_mid_price_vel_bk" => "MidPriceVelBk",
        "comp_book_depth_change_bk" => "BookDepthChangeBk",
        "comp_wall_detector_bk" => "WallDetectorBk",
        "comp_best_level_vol_bk" => "BestLevelVolBk",
        "comp_price_level_density_bk" => "PriceLevelDensityBk",
        "comp_liquidity_sweep_bk" => "LiquiditySweepBk",
        "comp_macd_hist_z_comp" => "MacdHistZComp",
        "comp_l3_cancel_ratio_ev" => "L3CancelRatioEv",
        "comp_auction_price_deviation_ev" => "AuctionPriceDeviationEv",
        "comp_l3_order_rate_ev" => "L3OrderRateEv",
        "comp_l3_spoofer_score_ev" => "L3SpooferScoreEv",
        "comp_l3_large_order_ev" => "L3LargeOrderEv",
        "comp_trade_cluster_ev" => "TradeClusterEv",
        "comp_vol_imb_zone_ev" => "VolImbZoneEv",
        "comp_vwap_dev_ev" => "VwapDevEv",
        "comp_cvd_ev" => "CvdEv",
        "comp_vdelta_ev" => "VdeltaEv",
        "comp_vpin_ev" => "VpinEv",
        "comp_warnfreq_ev" => "WarnFreqEv",
        "comp_gammasq_ev" => "GammaSqEv",
        "comp_particle_bar" => "ParticleBar",
        "comp_polyreg_bar" => "PolyRegBar",
        "comp_egarch_bar" => "EgarchBar",
        "comp_garch_bar" => "GarchBar",
        "comp_arima_bar" => "ArimaBar",
        "comp_ofi_bk" => "OrderFlowImbBk",
        "comp_value_area_tk" => "ValueAreaTrackerTk",
        "comp_tpo_balance_tk" => "TpoSessionBalanceTk",
        "comp_footprint_chart_tk" => "FootprintChartTk",
        "comp_footprint_imb_tk" => "FootprintImbTk",
        "comp_footprint_poc_tk" => "FootprintPocTk",
        "comp_market_micro_bk" => "MarketMicroBk",
        "comp_iceberg_lv" => "IcebergLv",
        "comp_quote_lifecycle_ev" => "QuoteLifecycleEv",
        "comp_index_corr_ky" => "IndexCorrBreakKy",
        "comp_weight_drift_ky" => "WeightDriftKy",
        "comp_hiddenliq_hy" => "HiddenLiqHy",
        "comp_tbabsorb_hy" => "TbAbsorbHy",
        "comp_sweepimpact_hy" => "SweepImpactHy",
        "comp_absorption_ev" => "AbsorptionEv",
        "comp_adaptwin_ev" => "AdaptWinEv",
        "comp_vol_idx_spike_ev" => "VolIdxSpikeEv",
        "comp_chand_comp" => "ChandComp",
        "comp_cks_comp" => "CksComp",
        "comp_atrts_comp" => "AtrtsComp",
        "comp_gann_hilo_cols" => "GannHiloCols",
        "comp_pressure_comp" => "PressureComp",
        "comp_trima_bands_cols" => "TrimaBandsCols",
        "comp_kp_comp" => "KpComp",
        "comp_butter_comp" => "ButterComp",
        "comp_cheby_comp" => "ChebyComp",
        "comp_sg_comp" => "SgComp",
        "comp_roof_comp" => "RoofComp",
        "comp_sflatp_comp" => "SflatpComp",
        "comp_srollp_comp" => "SrollpComp",
        "comp_srollrp_comp" => "SrollrpComp",
        "comp_sslopep_comp" => "SslopepComp",
        "comp_ssloperp_comp" => "SsloperpComp",
        "comp_sslopez_comp" => "SslopezComp",
        "comp_screstp_comp" => "ScrestpComp",
        "comp_sentent_comp" => "SententComp",
        "comp_sentr_comp" => "SentrComp",
        "comp_sflux_comp" => "SfluxComp",
        "comp_stft_comp" => "StftComp",
        "comp_sflat_comp" => "SflatComp",
        "comp_sslope_comp" => "SslopeComp",
        "comp_sbp_cols" => "SbpCols",
        "comp_sbprhl_comp" => "SbprhlComp",
        "comp_sbwf_comp" => "SbwfComp",
        "comp_scf_comp" => "ScfComp",
        "comp_screst_comp" => "ScrestComp",
        "comp_sent_comp" => "SentComp",
        "comp_ser_comp" => "SerComp",
        "comp_shmpr_comp" => "ShmprComp",
        "comp_slmpr_comp" => "SlmprComp",
        "comp_sroll_comp" => "SrollComp",
        "comp_sroll95_comp" => "Sroll95Comp",
        "comp_kalman_comp" => "KalmanComp",
        "comp_rts_comp" => "RtsComp",
        "comp_kslope_cols" => "KslopeCols",
        "comp_kscr_comp" => "KscrComp",
        "comp_kslopez_comp" => "KslopezComp",
        "comp_abg_cols" => "AbgCols",
        "comp_lr_cols" => "LrCols",
        "comp_reg_chan_cols" => "RegChanCols",
        "comp_reg_chan_width_comp" => "RegChanWidthComp",
        "comp_std_dev_chan_cols" => "StdDevChanCols",
        "comp_std_dev_width_comp" => "StdDevWidthComp",
        "comp_didi_cols" => "DidiCols",
        "comp_ssl_cols" => "SslCols",
        "comp_rvgi_cols" => "RvgiCols",
        "comp_ema_slope_comp" => "EmaSlopeComp",
        "comp_tii_comp" => "TiiComp",
        "comp_price_z_comp" => "PriceZComp",
        "comp_vpt_comp" => "VptComp",
        "comp_cci_comp" => "CciComp",
        "comp_cv_comp" => "CvComp",
        "comp_mi_comp" => "MiComp",
        "comp_rmi_comp" => "RmiComp",
        "comp_rvi_comp" => "RviComp",
        "comp_dsp_comp" => "DspComp",
        "comp_kc_cols" => "KcCols",
        "comp_kc_metrics_cols" => "KcMetricsCols",
        "comp_atrc_cols" => "AtrcCols",
        "comp_atr_chan_cols" => "AtrChanCols",
        "comp_starc_cols" => "StarcCols",
        "comp_vo_kc_cols" => "VoKcCols",
        "comp_bb_cols" => "BbCols",
        "comp_bb_period_cols" => "BbPeriodCols",
        "comp_bb_metrics_cols" => "BbMetricsCols",
        "comp_envelope_cols" => "EnvelopeCols",
        "comp_stoch_cols" => "StochCols",
        "comp_kdj_cols" => "KdjCols",
        "comp_ppo_cols" => "PpoCols",
        "comp_pvo_cols" => "PvoCols",
        "comp_trix_cols" => "TrixCols",
        "comp_tsi_cols" => "TsiCols",
        "comp_kst_cols" => "KstCols",
        "comp_pmo_cols" => "PmoCols",
        "comp_kvo_cols" => "KvoCols",
        "comp_rsi_oma_cols" => "RsiOmaCols",
        "comp_dpo_cols" => "DpoCols",
        "comp_elder_ray_cols" => "ElderRayCols",
        "ev_liq_rate" => "LiqRate",
        "ev_liq_cooldown" => "LiqCooldown",
        "ev_liq_vol_velocity" => "LiqVolVelocity",
        "ev_liq_vol_imbalance" => "LiqVolImbalance",
        "ev_liq_cascade" => "LiqCascade",
        "ev_agg_flow_imb" => "AggFlowImb",
        "ev_oi_change_rate_ev" => "OiChangeRateEv",
        "ev_block_flow" => "BlockFlow",
        "ev_block_rate" => "BlockRate",
        "ev_funding_time_decay_ev" => "FundingTimeDecayEv",
        "ev_settle_approach" => "SettleApproach",
        "ev_charm" => "Charm",
        "ev_warn_rate" => "WarnRate",
        "ev_tick_freq_anomaly" => "TickFreqAnomaly",
        "ev_agg_burst" => "AggBurst",
        "ev_large_tick_mom" => "LargeTickMom",
        "ev_size_wt_mom" => "SizeWtMom",
        "ev_funding_dir_shift" => "FundingDirShift",
        "ev_ls_extreme" => "LsExtreme",
        "ev_pred_funding_extreme" => "PredFundingExtreme",
        "ev_leverage_reduction" => "LeverageReduction",
        "ev_hl_range_ratio" => "HlRangeRatio",
        "ev_ticker_spread" => "TickerSpread",
        "ev_iv_skew_ev" => "IvSkewEv",
        "ev_risk_proximity" => "RiskProximity",
        "ev_mmr_track" => "MmrTrack",
        "ev_theta_decay" => "ThetaDecay",
        "ev_pct_change_z" => "PctChangeZ",
        "ev_hv_spike_ev" => "HvSpikeEv",
        "ev_fund_stress" => "FundStress",
        "ev_pin_risk" => "PinRisk",
        "ev_aggressor_imb" => "AggressorImb",
        "ev_auction_liq" => "AuctionLiq",
        "ev_block_size_z" => "BlockSizeZ",
        "ev_trade_run" => "TradeRun",
        "fractals" => "Fractals",
        "fvg_sig" => "FvgSig",
        "nbar_pivot_sig" => "NbarPivotSig",
        "bos_sig" => "BosSig",
        "logic_and" => "LogicAnd",
        "logic_or" => "LogicOr",
        "logic_xor" => "LogicXor",
        "logic_sign" => "LogicSign",
        "vol_regime_sig" => "VolRegimeSig",
        "rel_position_sig" => "RelPositionSig",
        "cusum_filter" => "CusumFilter",
        "smooth_lane" => "SmoothLane",
        "dir_detect" => "DirDetect",
        "regime_gate_sig" => "RegimeGateSig",
        "threshold_edge" => "ThresholdEdge",
        "threshold_gate_sig" => "ThresholdGateSig",
        "hysteresis_gate_sig" => "HysteresisGateSig",
        "vol_event_sig" => "VolEventSig",
        "slope_dir_line" => "SlopeDirLine",
        "hour_of_day" => "HourOfDay",
        "week_in_month" => "WeekInMonth",
        "weekday_occurrence" => "WeekdayOccurrence",
        "month_turn" => "MonthTurn",
        "quarter_turn" => "QuarterTurn",
        "weekend_prox" => "WeekendProx",
        "start_end_month" => "StartEndMonth",
        "start_end_quarter" => "StartEndQuarter",
        "start_end_week" => "StartEndWeek",
        "time_enc" => "TimeEnc",
        "pct_channels" => "PctChannels",
        "rsi_pct_bands" => "RsiPctBands",
        _ => {
            return Err(syn::Error::new(
                name.span(),
                "unknown cube formula",
            ));
        }
    };
    Ok(Ident::new(mapped, name.span()))
}

/// One slot-runtime registration: `Family => EnumName`.
struct SlotRuntime {
    family: Ident,
    enum_name: Ident,
}

impl Parse for SlotRuntime {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let family: Ident = input.parse()?;
        input.parse::<Token![=>]>()?;
        let enum_name: Ident = input.parse()?;
        Ok(SlotRuntime { family, enum_name })
    }
}

struct Universe {
    members: Vec<Member>,
    slot_runtimes: Vec<SlotRuntime>,
}

impl Parse for Universe {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let kw: Ident = input.parse()?;
        if kw != "members" {
            return Err(syn::Error::new(kw.span(), "expected `members { ... }`"));
        }
        let body;
        braced!(body in input);
        let members: Punctuated<Member, Token![,]> =
            body.parse_terminated(Member::parse, Token![,])?;

        let mut slot_runtimes = Vec::new();
        if !input.is_empty() {
            let kw2: Ident = input.parse()?;
            if kw2 != "slot_runtimes" {
                return Err(syn::Error::new(kw2.span(), "expected `slot_runtimes { ... }`"));
            }
            let body2;
            braced!(body2 in input);
            let srs: Punctuated<SlotRuntime, Token![,]> =
                body2.parse_terminated(SlotRuntime::parse, Token![,])?;
            slot_runtimes = srs.into_iter().collect();
        }

        Ok(Universe { members: members.into_iter().collect(), slot_runtimes })
    }
}

/// One stream-routing statement for a `Multi[…]` member: `if the sample is this stream,
/// drive the indicator's matching `*Consumer::update_*`. Side-effect only — a `Multi`
/// member is exactly one kind per `feed`, so only one of these fires; the value is read
/// afterwards from the inherent `indicator_value()`. The (variant, trait, method) table
/// is the SAME mapping the single-stream flavor arms use — one source of truth for routing.
fn stream_route_stmt(flavor: &Ident) -> TokenStream2 {
    let (variant, trait_path, method): (&str, &str, &str) = match flavor.to_string().as_str() {
        "OrderBook" => ("OrderBook", "crate::engine::streams::order_book_consumer::OrderBookConsumer", "update_orderbook"),
        "OrderbookDelta" => ("OrderbookDelta", "crate::engine::streams::orderbook_delta_consumer::OrderbookDeltaConsumer", "update_delta"),
        "OrderbookL3" => ("OrderbookL3", "crate::engine::streams::orderbook_l3_consumer::OrderbookL3Consumer", "update_orderbook_l3"),
        "Liquidation" => ("Liquidation", "crate::engine::streams::liquidation_consumer::LiquidationConsumer", "update_liquidation"),
        "OpenInterest" => ("OpenInterest", "crate::engine::streams::open_interest_consumer::OpenInterestConsumer", "update_oi"),
        "Tick" => ("Tick", "crate::engine::streams::tick_consumer::TickConsumer", "update_tick"),
        "Ticker" => ("Ticker", "crate::engine::streams::ticker_consumer::TickerConsumer", "update_ticker"),
        "AggTrade" => ("AggTrade", "crate::engine::streams::agg_trade_consumer::AggTradeConsumer", "update_agg_trade"),
        "Funding" => ("Funding", "crate::engine::streams::funding_rate_consumer::FundingRateConsumer", "update_funding"),
        "MarkPrice" => ("MarkPrice", "crate::engine::streams::mark_price_consumer::MarkPriceConsumer", "update_mark"),
        "LongShortRatio" => ("LongShortRatio", "crate::engine::streams::long_short_ratio_consumer::LongShortRatioConsumer", "update_long_short_ratio"),
        "BlockTrade" => ("BlockTrade", "crate::engine::streams::block_trade_consumer::BlockTradeConsumer", "update_block_trade"),
        "OptionGreeks" => ("OptionGreeks", "crate::engine::streams::option_greeks_consumer::OptionGreeksConsumer", "update_option_greeks"),
        "RiskLimit" => ("RiskLimit", "crate::engine::streams::risk_limit_consumer::RiskLimitConsumer", "update_risk_limit"),
        "MarketWarning" => ("MarketWarning", "crate::engine::streams::market_warning_consumer::MarketWarningConsumer", "update_market_warning"),
        "Basis" => ("Basis", "crate::engine::streams::basis_consumer::BasisConsumer", "update_basis"),
        "IndexPrice" => ("IndexPrice", "crate::engine::streams::index_price_consumer::IndexPriceConsumer", "update_index_price"),
        "Settlement" => ("Settlement", "crate::engine::streams::settlement_event_consumer::SettlementEventConsumer", "update_settlement"),
        "Auction" => ("Auction", "crate::engine::streams::auction_event_consumer::AuctionEventConsumer", "update_auction"),
        "FundingSettlement" => ("FundingSettlement", "crate::engine::streams::funding_settlement_consumer::FundingSettlementConsumer", "update_funding_settlement"),
        "PredictedFunding" => ("PredictedFunding", "crate::engine::streams::predicted_funding_consumer::PredictedFundingConsumer", "update_predicted_funding"),
        "CompositeIndex" => ("CompositeIndex", "crate::engine::streams::composite_index_consumer::CompositeIndexConsumer", "update_composite_index"),
        "InsuranceFund" => ("InsuranceFund", "crate::engine::streams::insurance_fund_consumer::InsuranceFundConsumer", "update_insurance_fund"),
        "HistoricalVolatility" => ("HistoricalVolatility", "crate::engine::streams::historical_volatility_consumer::HistoricalVolatilityConsumer", "update_historical_volatility"),
        "VolatilityIndex" => ("VolatilityIndex", "crate::engine::streams::volatility_index_consumer::VolatilityIndexConsumer", "update_volatility_index"),
        other => {
            let msg = format!("unknown stream `{other}` in Multi[…] — not a routable stream flavor");
            return quote! { compile_error!(#msg); };
        }
    };
    let var_id = Ident::new(variant, flavor.span());
    let method_id = Ident::new(method, flavor.span());
    let trait_path: TokenStream2 = trait_path.parse().unwrap();
    quote! {
        if let crate::contract::MarketSample::#var_id(__s) = sample {
            let _ = #trait_path::#method_id(m, __s);
        }
    }
}

/// The `feed` body for a `Multi[A, B, …]` member: route EACH declared stream's sample to
/// its `*Consumer::update_*` (the indicator latches each stream internally). Side-effect only —
/// outputs are read afterwards via `read(id)`. NO synchronized frame, NO `pick`, NO
/// `MultiStreamConsumer` — `INPUT = &[A, B, …]` IS the native multi-source declaration.
fn multi_feed_body(streams: &[Ident]) -> TokenStream2 {
    let routes: Vec<TokenStream2> = streams.iter().map(stream_route_stmt).collect();
    quote! {
        #( #routes )*
    }
}

/// The `feed` match-arm body for one member, by flavor. `m` is the bound concrete
/// indicator; `sample` is the `MarketSample`. Mirrors the legacy `@feed` arms:
/// a sample on a foreign stream is a no-op (returns the current value).
fn feed_body(member: &Member) -> TokenStream2 {
    let flavor = &member.flavor;
    // `+time` is a FEED-behavior flag (NOT a slot-family flag): on `Field`/`Fields` it appends
    // the wall-clock coordinate `ts` to the core feed → `feed(ts, value)` / `feed(ts, &lanes)`.
    // Time is the THIRD axis, orthogonal to INPUT (stream) and SOURCE (lanes). It is excluded
    // from the flag-enum generation (no `TimeId`/`TimeSlot`).
    let timed = member.flags.iter().any(|f| f == "time");
    match flavor.to_string().as_str() {
        // The single-lane pure-core path (K=1): the variant carries the resolved input field
        // (lifted from `const SOURCE` + config via `source_fields`). The factory extracts that
        // ONE field from the bar and feeds the core a scalar. A foreign (non-bar) sample is a
        // no-op. The core owns no source field and no transport knowledge — just `feed(f64)`.
        // With `+time`: `feed(ts, value)` — the orthogonal time coordinate rides alongside.
        "Field" if timed => quote! {
            if let crate::contract::MarketSample::Bar { open, high, low, close, volume, .. } = sample {
                if let Some(__f) = lanes.first() {
                    m.feed(ts, __f.extract(open, high, low, close, volume));
                }
            }
        },
        "Field" => quote! {
            if let crate::contract::MarketSample::Bar { open, high, low, close, volume, .. } = sample {
                if let Some(__f) = lanes.first() {
                    m.feed(__f.extract(open, high, low, close, volume));
                }
            }
        },
        // The MULTI-lane pure-core path (K>=2): the variant carries N resolved input fields;
        // the factory extracts EACH from the bar, in declared order, and feeds the core the
        // scalar slice (`feed(&[f64])`). A foreign (non-bar) sample is a no-op. The systemic
        // multi-source resolve — MACD (fast/slow field), CCI/ATR (high/low/close), OBV
        // (price+volume), … The core knows NO transport: it indexes its own declared lanes.
        "Fields" if timed => quote! {
            if let crate::contract::MarketSample::Bar { open, high, low, close, volume, .. } = sample {
                let mut __vals: arrayvec::ArrayVec<f64, 8> = arrayvec::ArrayVec::new();
                for __f in lanes.iter() {
                    let _ = __vals.try_push(__f.extract(open, high, low, close, volume));
                }
                m.feed(ts, __vals.as_slice());
            }
        },
        "Fields" => quote! {
            if let crate::contract::MarketSample::Bar { open, high, low, close, volume, .. } = sample {
                let mut __vals: arrayvec::ArrayVec<f64, 8> = arrayvec::ArrayVec::new();
                for __f in lanes.iter() {
                    let _ = __vals.try_push(__f.extract(open, high, low, close, volume));
                }
                m.feed(__vals.as_slice());
            }
        },
        // Pure-time consumer (the THIRD axis, no SOURCE): the value depends ONLY on the
        // wall-clock coordinate `ts` (canonical ms) — calendar one-hot cores. ts is always
        // supplied by the driver; a pure-time core is sample-agnostic (time is orthogonal to
        // whatever stream routed here). `feed(ts)` — no price, no `_close` lie, no `TimedBar`.
        "Time" => quote! {
            m.feed(ts);
        },
        "OrderBook" => quote! {
            if let crate::contract::MarketSample::OrderBook(b) = sample {
                let _ = crate::engine::streams::order_book_consumer::OrderBookConsumer::update_orderbook(m, b);
            }
        },
        "Liquidation" => quote! {
            if let crate::contract::MarketSample::Liquidation(l) = sample {
                let _ = crate::engine::streams::liquidation_consumer::LiquidationConsumer::update_liquidation(m, l);
            }
        },
        "OpenInterest" => quote! {
            if let crate::contract::MarketSample::OpenInterest(oi) = sample {
                let _ = crate::engine::streams::open_interest_consumer::OpenInterestConsumer::update_oi(m, oi);
            }
        },
        // --- single-stream consumer flavors: each matches its `MarketSample` variant and
        // routes to the matching `*Consumer` trait (UFCS, fully-qualified — no import here).
        // A sample on a foreign stream is a no-op (returns the current value). ---
        "Tick" => quote! {
            if let crate::contract::MarketSample::Tick(t) = sample {
                let _ = crate::engine::streams::tick_consumer::TickConsumer::update_tick(m, t);
            }
        },
        "Ticker" => quote! {
            if let crate::contract::MarketSample::Ticker(t) = sample {
                let _ = crate::engine::streams::ticker_consumer::TickerConsumer::update_ticker(m, t);
            }
        },
        "AggTrade" => quote! {
            if let crate::contract::MarketSample::AggTrade(t) = sample {
                let _ = crate::engine::streams::agg_trade_consumer::AggTradeConsumer::update_agg_trade(m, t);
            }
        },
        "Funding" => quote! {
            if let crate::contract::MarketSample::Funding(fr) = sample {
                let _ = crate::engine::streams::funding_rate_consumer::FundingRateConsumer::update_funding(m, fr);
            }
        },
        "MarkPrice" => quote! {
            if let crate::contract::MarketSample::MarkPrice(mp) = sample {
                let _ = crate::engine::streams::mark_price_consumer::MarkPriceConsumer::update_mark(m, mp);
            }
        },
        "LongShortRatio" => quote! {
            if let crate::contract::MarketSample::LongShortRatio(lsr) = sample {
                let _ = crate::engine::streams::long_short_ratio_consumer::LongShortRatioConsumer::update_long_short_ratio(m, lsr);
            }
        },
        "BlockTrade" => quote! {
            if let crate::contract::MarketSample::BlockTrade(bt) = sample {
                let _ = crate::engine::streams::block_trade_consumer::BlockTradeConsumer::update_block_trade(m, bt);
            }
        },
        "OrderbookL3" => quote! {
            if let crate::contract::MarketSample::OrderbookL3(l3) = sample {
                let _ = crate::engine::streams::orderbook_l3_consumer::OrderbookL3Consumer::update_orderbook_l3(m, l3);
            }
        },
        "OrderbookDelta" => quote! {
            if let crate::contract::MarketSample::OrderbookDelta(d) = sample {
                let _ = crate::engine::streams::orderbook_delta_consumer::OrderbookDeltaConsumer::update_delta(m, d);
            }
        },
        "OptionGreeks" => quote! {
            if let crate::contract::MarketSample::OptionGreeks(g) = sample {
                let _ = crate::engine::streams::option_greeks_consumer::OptionGreeksConsumer::update_option_greeks(m, g);
            }
        },
        "RiskLimit" => quote! {
            if let crate::contract::MarketSample::RiskLimit(r) = sample {
                let _ = crate::engine::streams::risk_limit_consumer::RiskLimitConsumer::update_risk_limit(m, r);
            }
        },
        "MarketWarning" => quote! {
            if let crate::contract::MarketSample::MarketWarning(w) = sample {
                let _ = crate::engine::streams::market_warning_consumer::MarketWarningConsumer::update_market_warning(m, w);
            }
        },
        "Basis" => quote! {
            if let crate::contract::MarketSample::Basis(b) = sample {
                let _ = crate::engine::streams::basis_consumer::BasisConsumer::update_basis(m, b);
            }
        },
        "IndexPrice" => quote! {
            if let crate::contract::MarketSample::IndexPrice(ip) = sample {
                let _ = crate::engine::streams::index_price_consumer::IndexPriceConsumer::update_index_price(m, ip);
            }
        },
        "Settlement" => quote! {
            if let crate::contract::MarketSample::Settlement(s) = sample {
                let _ = crate::engine::streams::settlement_event_consumer::SettlementEventConsumer::update_settlement(m, s);
            }
        },
        "Auction" => quote! {
            if let crate::contract::MarketSample::Auction(a) = sample {
                let _ = crate::engine::streams::auction_event_consumer::AuctionEventConsumer::update_auction(m, a);
            }
        },
        "FundingSettlement" => quote! {
            if let crate::contract::MarketSample::FundingSettlement(fs) = sample {
                let _ = crate::engine::streams::funding_settlement_consumer::FundingSettlementConsumer::update_funding_settlement(m, fs);
            }
        },
        "PredictedFunding" => quote! {
            if let crate::contract::MarketSample::PredictedFunding(pf) = sample {
                let _ = crate::engine::streams::predicted_funding_consumer::PredictedFundingConsumer::update_predicted_funding(m, pf);
            }
        },
        "CompositeIndex" => quote! {
            if let crate::contract::MarketSample::CompositeIndex(ci) = sample {
                let _ = crate::engine::streams::composite_index_consumer::CompositeIndexConsumer::update_composite_index(m, ci);
            }
        },
        "InsuranceFund" => quote! {
            if let crate::contract::MarketSample::InsuranceFund(ins) = sample {
                let _ = crate::engine::streams::insurance_fund_consumer::InsuranceFundConsumer::update_insurance_fund(m, ins);
            }
        },
        "HistoricalVolatility" => quote! {
            if let crate::contract::MarketSample::HistoricalVolatility(hv) = sample {
                let _ = crate::engine::streams::historical_volatility_consumer::HistoricalVolatilityConsumer::update_historical_volatility(m, hv);
            }
        },
        "VolatilityIndex" => quote! {
            if let crate::contract::MarketSample::VolatilityIndex(vi) = sample {
                let _ = crate::engine::streams::volatility_index_consumer::VolatilityIndexConsumer::update_volatility_index(m, vi);
            }
        },
        // Hybrid Tick+OrderBook consumer: a synchronized trade-and-book sample updates from
        // both; a book-only sample updates the book side and returns the current value.
        "HybridTickBook" => quote! {
            match sample {
                crate::contract::MarketSample::TickWithBook(t, b) => {
                    let _ = crate::engine::streams::hybrid_tick_book_consumer::HybridTickBookConsumer::update_tick_with_book(m, t, b);
                }
                crate::contract::MarketSample::OrderBook(b) => {
                    crate::engine::streams::hybrid_tick_book_consumer::HybridTickBookConsumer::update_book_only(m, b);
                }
                _ => {}
            }
        },
        other => {
            let msg = format!(
                "unknown flavor `{other}` (expected Field / Fields / Time / OrderBook / OrderbookL3 / OrderbookDelta / Liquidation / OpenInterest / Tick / Ticker / AggTrade / Funding / PredictedFunding / FundingSettlement / MarkPrice / LongShortRatio / BlockTrade / OptionGreeks / RiskLimit / MarketWarning / Basis / IndexPrice / Settlement / Auction / CompositeIndex / InsuranceFund / HistoricalVolatility / VolatilityIndex / HybridTickBook) or Multi[stream, …]"
            );
            quote! { compile_error!(#msg) }
        }
    }
}

/// Emit a box-free enum (`#name`) over `members`, with `build` (by machine-id),
/// `feed`/`update_bar`/`value`/`is_ready`/`reset`. The shared shape of the global
/// `ContractFactory` and every per-family slot-runtime — they differ only in the
/// member subset.
fn emit_runtime_enum(name: &Ident, members: &[&Member]) -> TokenStream2 {
    let variants: Vec<&Ident> = members.iter().map(|m| &m.variant).collect();
    let tys: Vec<&Path> = members.iter().map(|m| &m.ty).collect();
    let feeds: Vec<TokenStream2> = members
        .iter()
        .map(|m| if m.streams.is_empty() { feed_body(m) } else { multi_feed_body(&m.streams) })
        .collect();
    // is_ready/reset: a Multi member's per-`*Consumer`-trait methods are ambiguous
    // (N traits each define them), so it routes to the inherent canonical `indicator_*`.
    let readys: Vec<TokenStream2> = members
        .iter()
        .map(|m| if m.streams.is_empty() { quote! { m.is_ready() } } else { quote! { m.indicator_is_ready() } })
        .collect();
    let resets: Vec<TokenStream2> = members
        .iter()
        .map(|m| if m.streams.is_empty() { quote! { m.reset() } } else { quote! { m.indicator_reset() } })
        .collect();

    // read(IndicatorOutputId) -> f64: the NAMED, non-squashing scalar surface. One outer arm
    // per member, an inner match over that member's declared outputs dispatching to its
    // brace-named f64 getter (`m.<brace>()`). The single implicit `value` output dispatches to
    // the inherent f64 `value()` (non-stream) / `indicator_value()` (stream); a multi-output
    // member reaches every output through its named getter — no squashing accessor remains.
    let read_arms: Vec<TokenStream2> = members
        .iter()
        .map(|m| {
            let variant = &m.variant;
            let value_method = if m.streams.is_empty() {
                quote! { value }
            } else {
                quote! { indicator_value }
            };
            let inner: Vec<TokenStream2> = m
                .outputs
                .iter()
                .map(|o| {
                    let id_variant = output_id_parts(m.outputs.len(), &m.variant, o).0;
                    let getter = if o.to_string() == "value" {
                        quote! { m.#value_method() }
                    } else {
                        quote! { m.#o() }
                    };
                    quote! { IndicatorOutputId::#id_variant => #getter, }
                })
                .collect();
            quote! {
                Self::#variant(m, _) => match output {
                    #( #inner )*
                    _ => f64::NAN,
                },
            }
        })
        .collect();

    // grid(IndicatorOutputId) -> Option<&MatrixGrid>: the TABLE surface, the sibling of `read`
    // for `#`-marked outputs. One arm per member that declares any table output, dispatching to
    // its brace-named grid getter (`m.<name>()` returning `&MatrixGrid`). A scalar id, or a
    // member with no tables, yields `None`.
    let grid_arms: Vec<TokenStream2> = members
        .iter()
        .filter(|m| !m.matrix_outputs.is_empty())
        .map(|m| {
            let variant = &m.variant;
            let inner: Vec<TokenStream2> = m
                .matrix_outputs
                .iter()
                .map(|o| {
                    let id_variant = format_ident!("{}{}", m.variant, pascal(&o.to_string()));
                    quote! { IndicatorOutputId::#id_variant => Some(m.#o()), }
                })
                .collect();
            quote! {
                Self::#variant(m, _) => match output {
                    #( #inner )*
                    _ => None,
                },
            }
        })
        .collect();

    // primary(): a test/harness convenience that reads the member's FIRST declared output by
    // name. NOT a squashing accessor — it dispatches to a real named `IndicatorOutputId`, the
    // canonical primary. (The contract surface stays `read(id)`; this is just ergonomics.)
    let primary_id_arms: Vec<TokenStream2> = members
        .iter()
        .map(|m| {
            let variant = &m.variant;
            let first_id = output_id_parts(m.outputs.len(), &m.variant, &m.outputs[0]).0;
            quote! { Self::#variant(..) => IndicatorOutputId::#first_id, }
        })
        .collect();

    quote! {
        #[derive(Debug, Clone)]
        pub enum #name {
            // Each variant carries the pure core PLUS the resolved input FIELDS (the ordered
            // `OhlcvField` list lifted from `const SOURCE` + config via `source_fields`).
            // Single-source members carry one field; multi-field members (MACD/CCI/ATR/OBV)
            // carry N; non-scalar stream consumers carry none. Inline (`ArrayVec`, no heap).
            // The variant holding the fields is what lets the core stay a pure `feed` with no
            // transport knowledge — the factory extracts the fields from the bar.
            #( #variants(#tys, arrayvec::ArrayVec<crate::engine::ohlcv_field::OhlcvField, 8>), )*
        }

        impl #name {
            // NO `from_id(id, periods)`. That periods-only build was the defaulting
            // antipattern: it silently filled source / ma-type / nested params from a
            // member's defaults. The COMPUTE order path forbids defaults — it must demand
            // every param via the contract `Output`/`Port`/`Slot` tree and error on missing.
            // The demand-fill build replaces it (pending). Render builds its cold-start
            // instance from `Render::defaults()` (the one legitimate default home).

            /// Feed one sample at wall-clock `ts` (canonical MILLISECONDS). Side-effect only —
            /// the core latches its state; read outputs afterwards via [`Self::read`] by their
            /// named [`IndicatorOutputId`]. `ts` is the THIRD axis — orthogonal to the sample:
            /// forwarded to the core ONLY for time members (`Time` / `+time`), dropped otherwise.
            /// The driver always has it; a timeless caller may pass `0`. A foreign stream is a no-op.
            #[allow(unused_variables)]
            pub fn feed(
                &mut self,
                ts: i64,
                sample: crate::contract::MarketSample<'_>,
            ) {
                match self {
                    #( Self::#variants(m, lanes) => { #feeds } )*
                }
            }

            /// Convenience: feed one (timeless) OHLCV bar to this resolved runtime (delegates
            /// to `feed` with `ts = 0`, flavor-aware). The harness API for driving a boxed
            /// inner factory (a slotted smoother/oscillator — never a time core), distinct
            /// from the retired per-core `update_bar` crutch.
            pub fn update_bar(
                &mut self,
                open: f64,
                high: f64,
                low: f64,
                close: f64,
                volume: f64,
            ) {
                self.feed(0, crate::contract::MarketSample::Bar {
                    open, high, low, close, volume,
                })
            }

            /// Read ONE named output as a raw scalar — the non-squashing surface that replaces
            /// positional `IndicatorValue` consumption (each output keeps its real identity +
            /// domain via [`IndicatorOutputId`]). Total over this factory's outputs; a foreign
            /// id yields `NaN`.
            pub fn read(&self, output: IndicatorOutputId) -> f64 {
                match self {
                    #( #read_arms )*
                }
            }

            /// Read ONE table output as a whole [`MatrixGrid`](crate::engine::MatrixGrid) — the
            /// sibling of [`Self::read`] for `#`-declared outputs. `None` for a scalar id, a
            /// foreign id, or a producer with no tables. The grid is self-describing (its axes
            /// carry labels).
            pub fn grid(&self, output: IndicatorOutputId) -> Option<&crate::engine::MatrixGrid> {
                match self {
                    #( #grid_arms )*
                    _ => None,
                }
            }

            /// Read the member's FIRST declared output (its canonical primary) as a scalar —
            /// a test/harness convenience over `read(id)`. Dispatches to a real named output id;
            /// not a squashing accessor.
            pub fn primary(&self) -> f64 {
                let id = match self { #( #primary_id_arms )* };
                self.read(id)
            }

            /// `true` once warmed up enough to emit valid values.
            pub fn is_ready(&self) -> bool {
                match self { #( Self::#variants(m, _) => #readys, )* }
            }

            /// Reset to the initial (un-warmed) state.
            pub fn reset(&mut self) {
                match self { #( Self::#variants(m, _) => #resets, )* }
            }

            /// The REAL inline size (bytes) of the concrete core this variant holds —
            /// `size_of_val` of the ACTUAL inner type, NOT the box-free enum's max-variant
            /// size (which is identical for every variant). Per-member generated, so memory
            /// profiling sees each indicator's true inline struct footprint. Heap-held buffers
            /// (`Vec`/`VecDeque` of the period window) are NOT counted here — those are measured
            /// separately by a tracking allocator.
            pub fn core_size(&self) -> usize {
                match self { #( Self::#variants(m, _) => core::mem::size_of_val(m), )* }
            }
        }
    }
}

/// Emit a typed id-only enum `#id_name` over `members` — the TYPED config-field type
/// for a slot (the compiler admits ONLY these members, never a foreign id). Used for
/// a whole slot family (`MovingAverageId`) and for a flagged subset (`SmootherId`).
/// Implements [`FamilyId`] (`MEMBERS` = the widened admissible set the slot carries)
/// + widens to / narrows from the global `IndicatorId`.
fn emit_id_enum(id_name: &Ident, members: &[&Member], doc: &str) -> TokenStream2 {
    let variants: Vec<&Ident> = members.iter().map(|m| &m.variant).collect();
    quote! {
        #[doc = #doc]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum #id_name {
            #( #variants, )*
        }
        impl crate::contract::ParamScalar for #id_name {
            fn scalar_hash<H: ::core::hash::Hasher>(&self, state: &mut H) {
                ::core::hash::Hash::hash(self, state);
            }
        }
        impl #id_name {
            /// Every member of this enum (typed), in manifest order.
            pub const ALL: &'static [#id_name] = &[ #( #id_name::#variants, )* ];
            /// Narrow a global id to this set, or `None` if it is not a member.
            pub fn from_bar_id(
                id: crate::engine::indicator_id::IndicatorId,
            ) -> Option<Self> {
                match id {
                    #( crate::engine::indicator_id::IndicatorId::#variants =>
                        Some(Self::#variants), )*
                    _ => None,
                }
            }
        }
        impl crate::contract::FamilyId for #id_name {
            const MEMBERS: &'static [crate::engine::indicator_id::IndicatorId] = &[
                #( crate::engine::indicator_id::IndicatorId::#variants, )*
            ];
            fn to_bar_id(self) -> crate::engine::indicator_id::IndicatorId {
                match self {
                    #( Self::#variants =>
                        crate::engine::indicator_id::IndicatorId::#variants, )*
                }
            }
        }
        impl From<#id_name> for crate::engine::indicator_id::IndicatorId {
            fn from(x: #id_name) -> Self {
                <#id_name as crate::contract::FamilyId>::to_bar_id(x)
            }
        }
    }
}

/// Emit the box-free TYPED ORDER `IndicatorOrder` — the assembly-time dual of
/// `ContractFactory`. One variant per member holding that member's OWN typed `Config`
/// (the demand-fill node). `build` resolves each variant's kline source and `create`s the
/// core into the matching `ContractFactory` variant. Compute forbids defaults: there is no
/// `Default` on the configs, so a partial order does not type-check — structural fill is
/// guaranteed by construction; `FillError` is the seam for the SEMANTIC validation (member
/// ∈ family, ranges) that lands with nested composite configs.
fn emit_order_enum(members: &[&Member]) -> TokenStream2 {
    let variants: Vec<&Ident> = members.iter().map(|m| &m.variant).collect();
    let tys: Vec<&Path> = members.iter().map(|m| &m.ty).collect();
    quote! {
        /// A demand-fill order rejected at `build` time — a SEMANTIC gap (a chosen slot
        /// member is outside its family, a param out of range, …). Structural
        /// incompleteness is impossible: the configs carry no `Default`, so a partial order
        /// does not type-check. `node` names the offending indicator.
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct FillError {
            /// The indicator node the violation is on.
            pub node: crate::engine::indicator_id::IndicatorId,
            /// The demand: what must be fixed (e.g. "smoother HMA not in family").
            pub reason: String,
        }

        /// A cube point REJECTED by [`IndicatorOrder::build_many`]: the resolved (invalid) order
        /// plus the human reason from [`crate::contract::Config::valid_params`]. The "filtered as
        /// invalid" record returned alongside the valid set (also feeds the cost estimator: the
        /// real iteration window is `valid.len()`, not the raw cube size).
        #[derive(Debug, Clone)]
        pub struct RejectedCombo {
            /// The resolved order that failed validation.
            pub order: IndicatorOrder,
            /// Why it was rejected (one-liner from `valid_params`).
            pub reason: String,
        }

        /// The outcome of [`IndicatorOrder::build_many`]: the built VALID instances + the REJECTED
        /// combos (each with its invalidity reason). `valid.len() + rejected.len()` ≤ `cube_size`.
        pub struct SweepOutcome {
            /// Built, validated factory instances.
            pub valid: Vec<ContractFactory>,
            /// Combos filtered out as invalid, with reasons.
            pub rejected: Vec<RejectedCombo>,
        }

        /// The box-free TYPED ORDER — the assembly-time dual of [`ContractFactory`]. One
        /// variant per member holding that member's OWN typed `Config`. A composite's
        /// `Config` nests child `IndicatorOrder`s under its `Port`/`Slot`, so a full order
        /// is the explicit config TREE (Keltner → ATR → smoother). No `Default` on any
        /// config → a partial node cannot be constructed; `build` then validates the
        /// remaining SEMANTIC constraints and errors via [`FillError`].
        #[derive(Debug, Clone)]
        pub enum IndicatorOrder {
            #( #variants(<#tys as crate::contract::Indicator>::Config), )*
        }

        impl IndicatorOrder {
            /// The machine id of the ordered indicator.
            pub fn id(&self) -> crate::engine::indicator_id::IndicatorId {
                match self {
                    #( Self::#variants(_) =>
                        crate::engine::indicator_id::IndicatorId::#variants, )*
                }
            }

            /// A 64-bit fingerprint of this ordered indicator AND its resolved config params:
            /// the warmup column dedup key's param component (typed replacement for the OSS
            /// stringly `param_hash`). Folds the `id` discriminant with the variant config's
            /// [`ConfigAxes`]-generated `config_hash`.
            pub fn config_hash(&self) -> u64 {
                use ::core::hash::{Hash as _, Hasher as _};
                let mut __h = ::std::collections::hash_map::DefaultHasher::new();
                self.id().hash(&mut __h);
                let __cfg = match self {
                    #( Self::#variants(__c) => __c.config_hash(), )*
                };
                __cfg.hash(&mut __h);
                __h.finish()
            }

            /// The standard cold-start order for `id` — each variant filled from its
            /// [`crate::contract::Config::defaults`] (the one legitimate default home; the
            /// compute path forbids defaults). For harness / validation: drive EVERY id with
            /// its standard config without naming each typed config by hand. `None` if `id` is
            /// not contract-backed. Pair with [`Self::build`] to get a [`ContractFactory`].
            pub fn from_defaults(
                id: crate::engine::indicator_id::IndicatorId,
            ) -> Option<IndicatorOrder> {
                Some(match id {
                    #( crate::engine::indicator_id::IndicatorId::#variants =>
                        IndicatorOrder::#variants(
                            <<#tys as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()
                        ), )*
                    _ => return None,
                })
            }

            /// The machine-generator sweep order for `id` — each variant filled from its config's
            /// [`crate::contract::Config::machine_defaults`] (every sweepable axis a `Param::Many`
            /// over its machine-default range+step; do-not-sweep axes pinned `Solo`). Indicators
            /// that have not filled `machine_defaults()` yet fall back to the trait default
            /// (`defaults()`, all-`Solo`) — un-swept, never broken. `None` if `id` is not
            /// contract-backed. Pair with [`Self::iter`] + [`Self::build`] to expand the full
            /// sweep cube.
            pub fn from_machine_defaults(
                id: crate::engine::indicator_id::IndicatorId,
            ) -> Option<IndicatorOrder> {
                Some(match id {
                    #( crate::engine::indicator_id::IndicatorId::#variants =>
                        IndicatorOrder::#variants(
                            <<#tys as crate::contract::Indicator>::Config as crate::contract::Config>::machine_defaults()
                        ), )*
                    _ => return None,
                })
            }

            /// Build the box-free [`ContractFactory`] from this COMPLETE SOLO order — the central
            /// single-instance factory path. Runs the [`crate::contract::Config::valid_params`]
            /// gate FIRST: an invalid combo returns `Err(FillError{reason})` — never panics, never
            /// silently builds garbage. Both the directly-ordered solo AND each point of
            /// [`Self::build_many`] pass through this one gate.
            pub fn build_solo(self) -> Result<ContractFactory, FillError> {
                match self {
                    #(
                        Self::#variants(cfg) => {
                            <<#tys as crate::contract::Indicator>::Config as crate::contract::Config>::valid_params(&cfg)
                                .map_err(|reason| FillError {
                                    node: crate::engine::indicator_id::IndicatorId::#variants,
                                    reason,
                                })?;
                            let __fields = <#tys as crate::contract::Indicator>::source_fields(&cfg);
                            Ok(ContractFactory::#variants(
                                <#tys as crate::contract::Indicator>::create(cfg),
                                __fields,
                            ))
                        }
                    )*
                }
            }

            /// Validate this order's resolved config params — the per-indicator
            /// [`crate::contract::Config::valid_params`] check, UNIVERSAL across solo and sweep.
            /// `Ok(())` if a viable combo, `Err(reason)` otherwise. `build_solo` / `build_many`
            /// both gate on this; callers can also pre-check without building.
            pub fn valid_params(&self) -> Result<(), String> {
                match self {
                    #( Self::#variants(__c) =>
                        <<#tys as crate::contract::Indicator>::Config as crate::contract::Config>::valid_params(__c), )*
                }
            }

            /// The cube size of this (possibly swept) order — the active variant's config-axis
            /// cardinality product ([`crate::contract::Config::cube_size`]). An all-`Solo` order → 1.
            /// The generator's AST leaf calls this to size an indicator's sweep WITHOUT naming the
            /// typed config by hand — the typed surface that replaced the stringly `param_metadata`.
            pub fn cube_size(&self) -> u128 {
                match self {
                    #( Self::#variants(cfg) =>
                        <<#tys as crate::contract::Indicator>::Config as crate::contract::Config>::cube_size(cfg), )*
                }
            }

            /// Expand this swept order into the cartesian product of resolved (all-`Solo`) ORDERS —
            /// each a concrete cube point ready for [`Self::build`]. LAZY: streams a
            /// [`crate::contract::CubeIter`] over the active variant config's mixed-radix
            /// `axes_decode` (O(1) `next`/`nth`, no `Vec`), so the generator iterates typed
            /// `IndicatorOrder`s and `iter().nth(k)` jumps in O(1) — a wide candidate cube no
            /// longer materializes or walks element-by-element.
            pub fn iter(&self) -> Box<dyn Iterator<Item = IndicatorOrder> + '_> {
                let __total = self.cube_size();
                Box::new(crate::contract::CubeIter::new(__total, move |__i| match self {
                    #( Self::#variants(__c) => IndicatorOrder::#variants(__c.axes_decode(__i)), )*
                }))
            }

            /// Build every VALID cube point of this (possibly swept) order — the sweep factory path.
            /// Streams the lazy cube, runs each point through [`Self::build_solo`] (the same
            /// `valid_params` gate), and PARTITIONS into a [`SweepOutcome`]: the built valid
            /// instances + the rejected combos (each with its invalidity reason). A wide sweep with
            /// invalid points yields only the valid subset; nothing panics, nothing is silently lost.
            /// An all-`Solo` order yields exactly one valid (or one rejected).
            pub fn build_many(&self) -> SweepOutcome {
                let mut valid = Vec::new();
                let mut rejected = Vec::new();
                for point in self.iter() {
                    match point.valid_params() {
                        // valid_params passed → build_solo is infallible for this point.
                        Ok(()) => if let Ok(f) = point.build_solo() { valid.push(f); },
                        Err(reason) => rejected.push(RejectedCombo { order: point, reason }),
                    }
                }
                SweepOutcome { valid, rejected }
            }

            /// Overlay a swept PRIMARY period axis on this order — the generator's typed sweep
            /// override (it ALWAYS overrides; `from_defaults` is render-only Solo). Delegates to the
            /// active variant's config `set_primary_period` (method syntax: the `ConfigAxes`
            /// inherent setter for period-bearing configs, the [`crate::contract::Config`] default
            /// no-op for periodless ones). The period stays a `Param` field IN the config.
            pub fn with_period(mut self, period: crate::contract::Param<usize>) -> Self {
                use crate::contract::Config as _;
                match &mut self {
                    #( Self::#variants(cfg) => cfg.set_primary_period(period), )*
                }
                self
            }

            /// The resolved PRIMARY period of the active variant's config (the first
            /// `Param<usize>`), `None` for periodless indicators — the memory-relevant lookback
            /// depth. Method-syntax dispatch: the `ConfigAxes` inherent getter for period-bearing
            /// configs, the [`crate::contract::Config`] default `None` for periodless ones.
            pub fn primary_period(&self) -> Option<usize> {
                use crate::contract::Config as _;
                match self {
                    #( Self::#variants(cfg) => cfg.primary_period(), )*
                }
            }

            /// The RAW primary-period axis of the active variant's config (`Solo` or a swept
            /// `Many`), `None` for periodless indicators — the round-trip primitive a re-emitter
            /// needs to reconstruct `with_period(axis)` byte-for-byte, including a sweep. Method-
            /// syntax dispatch: the `ConfigAxes` inherent getter for period-bearing configs, the
            /// [`crate::contract::Config`] default `None` for periodless ones.
            pub fn primary_period_axis(&self) -> Option<crate::contract::Param<usize>> {
                use crate::contract::Config as _;
                match self {
                    #( Self::#variants(cfg) => cfg.primary_period_axis(), )*
                }
            }
        }
    }
}

/// Emit a NARROW box-free SLOT enum `#slot_name` over a flagged subset (e.g.
/// `SmootherSlot` from `+smoother`) — the typed field a host composite HOLDS for a
/// config-chosen inner core. It is the runtime twin of the flag's id enum
/// (`#id_name`, e.g. `SmootherId`): the id says WHICH, this holds the running core.
///
/// Built straight from the narrow id (`new(SmootherId, period)`) — NO `IndicatorId`
/// widening, NO bag. The members are PURE SCALAR cores, so the enum exposes only a
/// scalar `feed(f64) -> f64` / `value() -> f64`, NOT the whole OHLCV indicator
/// interface. Heavy/adaptive MAs are not in the flagged subset → they physically
/// cannot be a variant here (a smoother slot can never hold a FRAMA). This is the
/// narrow replacement for the whole-family `MovingAverageRuntime`.
fn emit_slot_enum(slot_name: &Ident, id_name: &Ident, members: &[&Member]) -> TokenStream2 {
    let variants: Vec<&Ident> = members.iter().map(|m| &m.variant).collect();
    let tys: Vec<&Path> = members.iter().map(|m| &m.ty).collect();
    quote! {
        #[derive(Debug, Clone)]
        pub enum #slot_name {
            #( #variants(#tys), )*
        }
        impl #slot_name {
            /// Build the chosen member from its typed narrow id + period.
            pub fn new(id: #id_name, period: usize) -> Self {
                match id {
                    #( #id_name::#variants => Self::#variants(#tys::new(period)), )*
                }
            }
            /// Feed ONE pre-extracted scalar — the host composite computed it.
            pub fn feed(&mut self, value: f64) -> f64 {
                match self { #( Self::#variants(m) => m.feed(value), )* }
            }
            /// The current scalar value.
            pub fn value(&self) -> f64 {
                match self { #( Self::#variants(m) => m.value_f64(), )* }
            }
            pub fn is_ready(&self) -> bool {
                match self { #( Self::#variants(m) => m.is_ready(), )* }
            }
            pub fn reset(&mut self) {
                match self { #( Self::#variants(m) => m.reset(), )* }
            }
            /// The period of the held smoother — delegates to the core's inherent getter.
            pub fn period(&self) -> usize {
                match self { #( Self::#variants(m) => m.period(), )* }
            }
        }
    }
}

/// Emit the demand-fill `SmootherSlotOrder` enum for the flagged smoother subset.
/// Each variant carries the smoother's FULL params (period + shape) as its
/// `Smoother::Params` associated type — so ALMA carries (period, offset, sigma)
/// while the 9 period-only smoothers carry `PeriodConfig { period }`.
/// Provides `into_slot` (builds the `SmootherSlot`), `id`, and `period`.
/// Also emits `SlotField for SmootherSlotOrder` so `#[derive(Slots)]` works uniformly.
fn emit_slot_order_enum(
    slot_name: &Ident,
    id_name: &Ident,
    members: &[&Member],
    slot_trait: &TokenStream2,
) -> TokenStream2 {
    let variants: Vec<&Ident> = members.iter().map(|m| &m.variant).collect();
    let tys: Vec<&Path> = members.iter().map(|m| &m.ty).collect();
    // Name mirrors the runtime slot dispatch: `SmootherSlot` -> `SmootherSlotOrder`
    // (convention: order name = `<runtime-dispatch>Order`). Derived from `slot_name`,
    // so it is correct for any future flag, not hardcoded.
    let order_name = format_ident!("{}Order", slot_name);
    quote! {
        /// A demand-fill order for a scalar slot: each variant carries the member's FULL
        /// typed params (`<trait>::Params`) — period + any shape knobs (ALMA: offset/sigma).
        /// The source-less config the order tree uses for a slotted scalar node.
        #[derive(Debug, Clone, Copy, PartialEq)]
        pub enum #order_name {
            #( #variants(<#tys as #slot_trait>::Params), )*
        }
        impl ::core::hash::Hash for #order_name {
            fn hash<H: ::core::hash::Hasher>(&self, state: &mut H) {
                ::core::mem::discriminant(self).hash(state);
                match self {
                    #( Self::#variants(p) => ::core::hash::Hash::hash(p, state), )*
                }
            }
        }
        impl crate::contract::ParamScalar for #order_name {
            fn scalar_hash<H: ::core::hash::Hasher>(&self, state: &mut H) {
                ::core::hash::Hash::hash(self, state);
            }
        }
        impl #order_name {
            /// Build the runtime `#slot_name` from this order.
            pub fn into_slot(self) -> #slot_name {
                match self {
                    #( Self::#variants(p) =>
                        #slot_name::#variants(<#tys as #slot_trait>::from_params(p)), )*
                }
            }
            /// The id of the chosen member.
            pub fn id(&self) -> #id_name {
                match self { #( Self::#variants(_) => #id_name::#variants, )* }
            }
            /// The period the chosen member was configured with.
            pub fn period(&self) -> usize {
                match self {
                    #( Self::#variants(p) =>
                        <#tys as #slot_trait>::params_period(p), )*
                }
            }
            /// The machine-generator sweep over this slot: EVERY family member, each expanded
            /// over its OWN source-less param cube (`SlotParamsSweep::machine_params`) — a
            /// disjoint union, so a shape-bearing member (ALMA: period×offset×sigma) contributes
            /// its extra axes only in its own branch, never multiplying the period-only members.
            /// The host config's `machine_defaults` sets the `#[slot]` field to
            /// `Param::many(Self::machine_sweep())`.
            pub fn machine_sweep() -> ::std::vec::Vec<Self> {
                let mut out = ::std::vec::Vec::new();
                #(
                    out.extend(
                        <<#tys as #slot_trait>::Params as crate::contract::SlotParamsSweep>::machine_params()
                            .into_iter()
                            .map(Self::#variants),
                    );
                )*
                out
            }
        }
        impl crate::contract::SlotField for #order_name {
            const CANDIDATES: &'static [crate::engine::indicator_id::IndicatorId] =
                <#id_name as crate::contract::FamilyId>::MEMBERS;
            fn member_id(&self) -> crate::engine::indicator_id::IndicatorId {
                crate::contract::FamilyId::to_bar_id(self.id())
            }
        }
    }
}

/// Emit the config-time slot CHOICE `<Flag>Choice` — the typed replacement for the
/// period-carrying `<Flag>SlotOrder`. It carries WHICH member (`#id_name`) + WHERE its period
/// comes from (`SlotPeriod`: `Follow` the host indicator's period — the default — or `Own(p)`).
/// `build(host_period)` resolves `Follow` against the host and constructs the runtime `#slot_name`.
/// The `#[slot]` field a host config holds is `Param<#choice_name>` (sweep WHICH × follow/own).
/// Copy (a resolved point); the sweep lives on the OUTER `Param::Many`.
fn emit_slot_choice_enum(choice_name: &Ident, id_name: &Ident, slot_name: &Ident) -> TokenStream2 {
    quote! {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub struct #choice_name {
            /// Which family member (the typed narrow id).
            pub kind: #id_name,
            /// Follow the host's period (default) or run at an own period.
            pub period: crate::contract::SlotPeriod,
        }
        impl crate::contract::ParamScalar for #choice_name {
            fn scalar_hash<H: ::core::hash::Hasher>(&self, state: &mut H) {
                ::core::hash::Hash::hash(self, state);
            }
        }
        impl #choice_name {
            /// Member following the HOST indicator's period (the default mode).
            pub const fn follow(kind: #id_name) -> Self {
                Self { kind, period: crate::contract::SlotPeriod::Follow }
            }
            /// Member at its OWN period, independent of the host.
            pub const fn own(kind: #id_name, period: usize) -> Self {
                Self { kind, period: crate::contract::SlotPeriod::Own(period) }
            }
            /// Build the runtime `#slot_name`, resolving `Follow` to `host_period`.
            pub fn build(self, host_period: usize) -> #slot_name {
                #slot_name::new(self.kind, self.period.resolve(host_period))
            }
            /// The chosen member id.
            pub fn id(&self) -> #id_name {
                self.kind
            }
            /// The machine-generator sweep over this slot: EVERY family member
            /// (`#id_name::ALL`) with a host-FOLLOWING period. Varying WHICH member fills the
            /// slot is the slot's sweep axis; the member's period rides the host's own period
            /// sweep (Follow), so no independent period axis is added here. The host config's
            /// `machine_defaults` sets the `#[slot]` field to `Param::many(Self::machine_sweep())`.
            pub fn machine_sweep() -> ::std::vec::Vec<Self> {
                #id_name::ALL.iter().map(|&k| Self::follow(k)).collect()
            }
        }
        impl crate::contract::SlotField for #choice_name {
            const CANDIDATES: &'static [crate::engine::indicator_id::IndicatorId] =
                <#id_name as crate::contract::FamilyId>::MEMBERS;
            fn member_id(&self) -> crate::engine::indicator_id::IndicatorId {
                crate::contract::FamilyId::to_bar_id(self.kind)
            }
        }
    }
}

/// Convert a snake_case output name to PascalCase (e.g. `std_dev` -> `StdDev`).
fn pascal(s: &str) -> String {
    s.split('_')
        .map(|seg| {
            let mut c = seg.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            }
        })
        .collect()
}

/// Convert a PascalCase variant to snake_case (e.g. `MacdHistZ` -> `macd_hist_z`).
fn to_snake(s: &str) -> String {
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() && i > 0 {
            out.push('_');
        }
        out.extend(ch.to_lowercase());
    }
    out
}

/// The typed-id + display-name parts for one `(member, output)` pair.
///
/// A single-output member whose sole output is the implicit `value` collapses to the BARE
/// variant id `<Var>` (no `Value` suffix), named with the indicator's snake name — `value`
/// is a placeholder, never an output IDENTITY, so a one-output indicator IS its output. Every
/// other output keeps `<Var><Pascal(output)>` named by its manifest brace.
fn output_id_parts(outputs_len: usize, variant: &Ident, o: &Ident) -> (Ident, String) {
    if outputs_len == 1 && o.to_string() == "value" {
        (variant.clone(), to_snake(&variant.to_string()))
    } else {
        (format_ident!("{}{}", variant, pascal(&o.to_string())), o.to_string())
    }
}

/// Emit the global typed `IndicatorOutputId` enum: one variant per (member, output) pair,
/// named `<Variant><PascalCase(output)>` (e.g. `BbStdDev`), collapsing a single-output
/// member's implicit `value` to the bare variant id `<Var>` (e.g. `Sma`, `Rsi`).
/// Also emits `IndicatorOutputId::ALL`, `IndicatorOutputId::indicator(self)`, `IndicatorOutputId::name(self)`,
/// and the free function `output_ids_of(IndicatorId) -> &'static [IndicatorOutputId]`.
fn emit_output_id_enum(members: &[&Member]) -> TokenStream2 {
    // Flat list of all (member, output) pairs.
    let mut all_variant_idents: Vec<proc_macro2::Ident> = Vec::new();
    let mut owning_member_variants: Vec<&Ident> = Vec::new();
    let mut output_name_str_lits: Vec<String> = Vec::new();

    for m in members {
        for o in &m.outputs {
            let (id_ident, name_str) = output_id_parts(m.outputs.len(), &m.variant, o);
            all_variant_idents.push(id_ident);
            owning_member_variants.push(&m.variant);
            output_name_str_lits.push(name_str);
        }
        // TABLE outputs share the ONE id space — always `<Variant><Pascal(name)>` (never the
        // bare-`value` collapse; a table is never the implicit sole output).
        for o in &m.matrix_outputs {
            all_variant_idents.push(format_ident!("{}{}", m.variant, pascal(&o.to_string())));
            owning_member_variants.push(&m.variant);
            output_name_str_lits.push(o.to_string());
        }
    }

    // Loud guard: collapsing a single-output member's implicit `value` to the bare variant
    // id can alias another member's `<Variant><Output>` pair (e.g. the `LrSlope` indicator vs
    // `Lr`'s `slope` output). A clash would otherwise surface as a cryptic E0428 *inside* the
    // derive — name the offender directly so the manifest fix is obvious.
    let mut seen = std::collections::HashSet::new();
    let mut dup_guard = quote! {};
    for id in &all_variant_idents {
        if !seen.insert(id.to_string()) {
            let msg = format!(
                "duplicate IndicatorOutputId `{id}` — a single-output member's bare id collides \
                 with another member's <Variant><Output>; rename one output in the manifest"
            );
            dup_guard = quote! { compile_error!(#msg); };
        }
    }

    // Per-member: slices of that member's output variants for `output_ids_of`.
    let per_member_match_arms: Vec<TokenStream2> = members
        .iter()
        .map(|m| {
            let member_variant = &m.variant;
            let this_members_output_variants: Vec<proc_macro2::Ident> = m
                .outputs
                .iter()
                .map(|o| output_id_parts(m.outputs.len(), &m.variant, o).0)
                .chain(
                    m.matrix_outputs
                        .iter()
                        .map(|o| format_ident!("{}{}", m.variant, pascal(&o.to_string()))),
                )
                .collect();
            quote! {
                crate::engine::indicator_id::IndicatorId::#member_variant =>
                    &[ #( IndicatorOutputId::#this_members_output_variants, )* ],
            }
        })
        .collect();

    quote! {
        #dup_guard

        /// Typed identity of a single named output value an indicator emits.
        /// One variant per `(indicator, output-name)` pair declared in the manifest.
        /// A single-output member (no explicit output block / sole implicit `value`) collapses
        /// to the bare variant id `<Var>` — a one-output indicator IS its output, so `value`
        /// never appears as an identity.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum IndicatorOutputId {
            #( #all_variant_idents, )*
        }

        impl IndicatorOutputId {
            /// Every output identity, in manifest order.
            pub const ALL: &'static [IndicatorOutputId] = &[ #( IndicatorOutputId::#all_variant_idents, )* ];

            /// The indicator this output belongs to.
            pub const fn indicator(
                self,
            ) -> crate::engine::indicator_id::IndicatorId {
                match self {
                    #( IndicatorOutputId::#all_variant_idents =>
                        crate::engine::indicator_id::IndicatorId::#owning_member_variants, )*
                }
            }

            /// The output's declared name. For display/debug only — NOT for logic.
            pub const fn name(self) -> &'static str {
                match self {
                    #( IndicatorOutputId::#all_variant_idents => #output_name_str_lits, )*
                }
            }
        }

        /// The typed output identities a `IndicatorId` emits, in declared order.
        /// Returns `&[]` for ids not backed by the contract.
        pub fn output_ids_of(
            id: crate::engine::indicator_id::IndicatorId,
        ) -> &'static [IndicatorOutputId] {
            match id {
                #( #per_member_match_arms )*
                _ => &[],
            }
        }
    }
}

#[proc_macro]
pub fn contract_universe(input: TokenStream) -> TokenStream {
    let universe = syn::parse_macro_input!(input as Universe);
    let members = &universe.members;

    let all_refs: Vec<&Member> = members.iter().collect();
    let variants: Vec<&Ident> = members.iter().map(|m| &m.variant).collect();
    let tys: Vec<&Path> = members.iter().map(|m| &m.ty).collect();
    // The TIME axis: members that consume the orthogonal wall-clock — a `Time` (pure-time)
    // core or any `+time`-flagged field core. Drives the catalog `needs_time_of` projection.
    let timed_variants: Vec<&Ident> = members
        .iter()
        .filter(|m| m.flavor == "Time" || m.flags.iter().any(|f| f == "time"))
        .map(|m| &m.variant)
        .collect();
    let mut gpu_conflict = None;
    for member in members.iter() {
        let cube = member.flags.iter().any(|f| f == "cube");
        let shader = member.flags.iter().any(|f| f == "shader");
        if cube && shader {
            gpu_conflict = Some(member.variant.clone());
            break;
        }
    }
    if let Some(variant) = gpu_conflict {
        return syn::Error::new(
            variant.span(),
            "an indicator implements at most one of +cube and +shader",
        )
        .to_compile_error()
        .into();
    }
    let mut cube_ids: Vec<&Ident> = Vec::new();
    let mut cube_formulas: Vec<&Ident> = Vec::new();
    for member in members.iter() {
        if let Some(formula) = &member.cube_formula {
            cube_ids.push(&member.variant);
            cube_formulas.push(formula);
        }
    }
    let shader_variants: Vec<&Ident> = members
        .iter()
        .filter(|m| m.flags.iter().any(|f| f == "shader"))
        .map(|m| &m.variant)
        .collect();

    // --- global box-free factory (all members) ---
    let factory_enum = emit_runtime_enum(&Ident::new("ContractFactory", proc_macro2::Span::call_site()), &all_refs);

    // --- factory-only extras: is_migrated (typed membership predicate) ---
    let factory_extras = quote! {
        impl ContractFactory {
            /// `true` if `id` is built by this box-free factory (migrated to the contract).
            pub fn is_migrated(id: crate::engine::indicator_id::IndicatorId) -> bool {
                matches!(id, #( crate::engine::indicator_id::IndicatorId::#variants )|* )
            }
        }
    };

    // --- id -> contract catalog projections (all members) ---
    let catalog = quote! {
        /// Families an id self-declares. `None` if not contract-backed.
        pub fn family_of(
            id: crate::engine::indicator_id::IndicatorId,
        ) -> Option<&'static [crate::contract::Family]> {
            Some(match id {
                #( crate::engine::indicator_id::IndicatorId::#variants =>
                    <#tys as crate::contract::Indicator>::FAMILY, )*
                _ => return None,
            })
        }
        /// GPU dispatch of an id. `+cube(formula)` is one shared CubeCL kernel.
        /// `+shader` is hand-written WGSL, and only when no formula fits.
        /// An unmarked member stays `GpuMode::None`.
        pub fn gpu_of(id: crate::engine::indicator_id::IndicatorId) -> crate::contract::GpuMode {
            match id {
                #( crate::engine::indicator_id::IndicatorId::#cube_ids =>
                    crate::contract::GpuMode::Cube, )*
                #( crate::engine::indicator_id::IndicatorId::#shader_variants =>
                    crate::contract::GpuMode::Shader, )*
                _ => crate::contract::GpuMode::None,
            }
        }
        /// The cube formula behind `+cube(name)`. `None` when the id is not a cube lane.
        pub fn formula_of(
            id: crate::engine::indicator_id::IndicatorId,
        ) -> Option<crate::contract::CubeFormula> {
            match id {
                #( crate::engine::indicator_id::IndicatorId::#cube_ids =>
                    Some(crate::contract::CubeFormula::#cube_formulas), )*
                _ => None,
            }
        }
        /// The dig3 stream(s) an id consumes. `None` if not contract-backed.
        pub fn inputs_of(
            id: crate::engine::indicator_id::IndicatorId,
        ) -> Option<&'static [crate::engine::stream_kind::StreamKind]> {
            Some(match id {
                #( crate::engine::indicator_id::IndicatorId::#variants =>
                    <#tys as crate::contract::Indicator>::INPUT, )*
                _ => return None,
            })
        }
        /// An id's self-declared cost facets. `None` if not contract-backed.
        pub fn cost_of(
            id: crate::engine::indicator_id::IndicatorId,
        ) -> Option<crate::contract::Cost> {
            Some(match id {
                #( crate::engine::indicator_id::IndicatorId::#variants =>
                    <#tys as crate::contract::Indicator>::COST, )*
                _ => return None,
            })
        }
        /// The full named output surface an id emits. `None` if not contract-backed.
        pub fn outputs_of(
            id: crate::engine::indicator_id::IndicatorId,
        ) -> Option<&'static [crate::contract::Output]> {
            Some(match id {
                #( crate::engine::indicator_id::IndicatorId::#variants =>
                    <#tys as crate::contract::Indicator>::OUTPUTS, )*
                _ => return None,
            })
        }
        /// Draw spec for an id. `None` if not contract-backed. The chart reads this
        /// instead of a parallel rendering table.
        pub fn rendering_of(
            id: crate::engine::indicator_id::IndicatorId,
        ) -> Option<crate::contract::RenderSpec> {
            Some(match id {
                #( crate::engine::indicator_id::IndicatorId::#variants =>
                    <#tys as crate::contract::Render>::rendering(), )*
                _ => return None,
            })
        }
        /// The input-side source axis. `None` if non-bar or not contract-backed.
        pub fn source_of(
            id: crate::engine::indicator_id::IndicatorId,
        ) -> Option<crate::contract::SourceAxis> {
            match id {
                #( crate::engine::indicator_id::IndicatorId::#variants =>
                    <#tys as crate::contract::Indicator>::SOURCE, )*
                _ => None,
            }
        }
        /// The config-chosen family slots an id embeds. `&[]` for non-slot / unbacked ids.
        pub fn slots_of(
            id: crate::engine::indicator_id::IndicatorId,
        ) -> &'static [crate::contract::Slot] {
            match id {
                #( crate::engine::indicator_id::IndicatorId::#variants =>
                    <#tys as crate::contract::Indicator>::SLOTS, )*
                _ => &[],
            }
        }
        /// `true` if an id REQUIRES the volume stream (VWMA / VWAP).
        pub fn needs_volume_of(
            id: crate::engine::indicator_id::IndicatorId,
        ) -> bool {
            match id {
                #( crate::engine::indicator_id::IndicatorId::#variants =>
                    <#tys as crate::contract::Indicator>::NEEDS_VOLUME, )*
                _ => false,
            }
        }
        /// `true` if an id consumes the orthogonal wall-clock coordinate (the TIME axis) — a
        /// `Time` (pure-time) core or a `+time`-flagged field core. The factory feeds these the
        /// `ts` (canonical ms) of `feed(ts, sample)`; all other ids ignore it. `false` for
        /// timeless / uncontracted ids. A pure-time core legitimately has `source_of == None`
        /// while still consuming the `Bar` stream — it draws the clock, not a kline field.
        pub fn needs_time_of(
            id: crate::engine::indicator_id::IndicatorId,
        ) -> bool {
            matches!(
                id,
                #( crate::engine::indicator_id::IndicatorId::#timed_variants )|*
            )
        }
        /// The STATIC-weight slot members of `id` — read off its single default config,
        /// which lives on [`Config::defaults`] (compute forbids defaults; the static
        /// cost estimate prices the one cold-start/default config). In `SLOTS` order. `&[]`
        /// for unbacked/slot-free. (Nuance: the cost projection reads the default config —
        /// revisit when the demand-fill order-time cost lands.)
        pub fn slot_default_members_of(
            id: crate::engine::indicator_id::IndicatorId,
        ) -> Vec<crate::engine::indicator_id::IndicatorId> {
            match id {
                #( crate::engine::indicator_id::IndicatorId::#variants =>
                    <#tys as crate::contract::Indicator>::slot_members(
                        &<<#tys as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()
                    ), )*
                _ => Vec::new(),
            }
        }
    };

    // --- per-family slot-runtime enums (box-free runtime) + the family id enum ---
    let family_enums: Vec<TokenStream2> = universe
        .slot_runtimes
        .iter()
        .map(|sr| {
            let subset: Vec<&Member> = members
                .iter()
                .filter(|m| m.family.as_ref() == Some(&sr.family))
                .collect();
            let runtime = emit_runtime_enum(&sr.enum_name, &subset);
            let id_name = quote::format_ident!("{}Id", sr.family);
            let doc = format!("Typed id of a `Family::{}` member.", sr.family);
            let id_enum = emit_id_enum(&id_name, &subset, &doc);
            quote! { #runtime #id_enum }
        })
        .collect();

    // --- flagged id-subset enums (e.g. `+smoother` -> `SmootherId`): the TYPED
    // field type for a constrained slot — the compiler admits only flagged members,
    // so the heavy/adaptive MAs can never be picked as a low-level smoother. ---
    let mut flag_names: Vec<String> = members
        .iter()
        .flat_map(|m| m.flags.iter().map(|f| f.to_string()))
        .collect();
    flag_names.sort();
    flag_names.dedup();
    // `time` is a FEED-behavior flag, not a slot-family flag: it changes the core's feed arm
    // (`+time` → `feed(ts, …)`), it does NOT define a slottable id-subset. Exclude it so no
    // `TimeId`/`TimeSlot`/`TimeSlotOrder` is generated (cf. `smoother`/`oscillator`, which do).
    // `time` changes the feed arm. `cube` and `shader` select `gpu_of`. None of
    // the three is a slot family, so they must not grow a `*Id` / `*Slot`.
    flag_names.retain(|f| f != "time" && f != "cube" && f != "shader");
    let flag_enums: Vec<TokenStream2> = flag_names
        .iter()
        .map(|flag| {
            let cap = {
                let mut c = flag.chars();
                c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
            };
            let id_name = quote::format_ident!("{}Id", cap);
            let slot_name = quote::format_ident!("{}Slot", cap);
            let choice_name = quote::format_ident!("{}Choice", cap);
            let subset: Vec<&Member> = members
                .iter()
                .filter(|m| m.flags.iter().any(|f| f == flag))
                .collect();
            let doc = format!("Typed `+{flag}`-flagged id subset — the constrained slot-field type.");
            let id_enum = emit_id_enum(&id_name, &subset, &doc);
            // The runtime twin: a narrow box-free slot enum (`SmootherSlot`) the host
            // composite holds, built straight from the narrow id. Replaces the
            // whole-family `MovingAverageRuntime` for flagged scalar slots.
            let slot_enum = emit_slot_enum(&slot_name, &id_name, &subset);
            // The config-time CHOICE (`SmootherChoice` = kind + follow/own period) — the typed
            // replacement for the period-carrying `SmootherSlotOrder`. Host config holds a
            // `Param<#choice_name>`. (Additive: `SmootherSlotOrder` stays until the fan-out.)
            let choice_enum = emit_slot_choice_enum(&choice_name, &id_name, &slot_name);
            // Also emit the demand-fill `<Flag>SlotOrder` enum (each variant carries the
            // member's FULL typed params + `SlotField`) for every flag whose members
            // implement a scalar-slot params trait. `smoother` -> `crate::contract::Smoother`;
            // `oscillator` -> `crate::contract::Oscillator` (the same machinery, a parallel
            // trait). A composite's `#[slot]` field is a `<Flag>SlotOrder`.
            let extra = match flag.as_str() {
                "smoother" => {
                    let t = quote! { crate::contract::Smoother };
                    emit_slot_order_enum(&slot_name, &id_name, &subset, &t)
                }
                "oscillator" => {
                    let t = quote! { crate::contract::Oscillator };
                    emit_slot_order_enum(&slot_name, &id_name, &subset, &t)
                }
                _ => quote! {},
            };
            quote! { #id_enum #slot_enum #choice_enum #extra }
        })
        .collect();

    let slot_families: Vec<&Ident> = universe.slot_runtimes.iter().map(|s| &s.family).collect();
    let slot_runtime_registry = quote! {
        /// Families exposed as iterable slot-runtime enums (one `slot_runtimes`
        /// entry each). A test asserts every family used in any `SLOTS` is here.
        pub const SLOT_RUNTIME_FAMILIES: &[crate::contract::Family] = &[
            #( crate::contract::Family::#slot_families, )*
        ];
    };

    // --- guard: the manifest family token must equal the type's FAMILY[0] (or, for
    // a `_` member, its FAMILY must be empty) — the token cannot drift from the const.
    let guard_asserts: Vec<TokenStream2> = members
        .iter()
        .map(|m| {
            let ty = &m.ty;
            let variant = &m.variant;
            match &m.family {
                Some(f) => quote! {
                    assert_eq!(
                        <#ty as crate::contract::Indicator>::FAMILY.first(),
                        Some(&crate::contract::Family::#f),
                        "manifest family token for {} disagrees with its FAMILY[0]",
                        stringify!(#variant),
                    );
                },
                None => quote! {
                    assert!(
                        <#ty as crate::contract::Indicator>::FAMILY.is_empty(),
                        "manifest declares `_` (no family) for {} but its FAMILY is non-empty",
                        stringify!(#variant),
                    );
                },
            }
        })
        .collect();
    // --- global typed IndicatorOutputId enum ---
    let output_id_enum = emit_output_id_enum(&all_refs);

    // --- box-free typed order (`IndicatorOrder`) — assembly-time dual of ContractFactory ---
    let order_enum = emit_order_enum(&all_refs);

    let guards = quote! {
        #[cfg(test)]
        mod __contract_universe_guards {
            use super::*;
            #[test]
            fn manifest_family_tokens_match_const() {
                #( #guard_asserts )*
            }
            #[test]
            fn output_ids_belong_to_their_indicator() {
                for o in IndicatorOutputId::ALL {
                    let ids = output_ids_of(o.indicator());
                    assert!(ids.contains(o), "{:?} maps to indicator {:?} which does not list it", o, o.indicator());
                }
            }
        }
    };

    let expanded = quote! {
        #factory_enum
        #factory_extras
        #catalog
        #( #family_enums )*
        #( #flag_enums )*
        #slot_runtime_registry
        #output_id_enum
        #order_enum
        #guards
    };

    expanded.into()
}

/// `#[derive(Slots)]` — generate a typed config's slot metadata from its
/// `#[slot]`-marked family-id fields. NO hand-written `const SLOTS`, no stringly
/// `config_key`, no `ma_types` bag: the slot's family is read straight off each
/// field's [`crate::contract::FamilyId`] TYPE (`<FieldTy as FamilyId>::FAMILY`),
/// and the resolved members off the field VALUES — they cannot drift, there is one
/// source (the typed field). Emits inherent `Self::SLOTS` (one slot per `#[slot]`
/// field, consumed `["value"]`) + `Self::slot_members(&self)` (the members in field
/// order). The indicator wires them into its contract:
/// `const SLOTS = Cfg::SLOTS; fn slot_members(c) { c.slot_members() }`.
#[proc_macro_derive(Slots, attributes(slot))]
pub fn derive_slots(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    let name = &input.ident;
    let fields = match &input.data {
        syn::Data::Struct(s) => &s.fields,
        _ => {
            return syn::Error::new_spanned(&input, "#[derive(Slots)] is only for structs")
                .to_compile_error()
                .into()
        }
    };
    let slot_fields: Vec<&syn::Field> = fields
        .iter()
        .filter(|f| f.attrs.iter().any(|a| a.path().is_ident("slot")))
        .collect();
    let specs: Vec<TokenStream2> = slot_fields
        .iter()
        .map(|f| {
            let ty = &f.ty;
            quote! { crate::contract::Slot::new(<#ty as crate::contract::SlotField>::CANDIDATES) }
        })
        .collect();
    let members: Vec<TokenStream2> = slot_fields
        .iter()
        .map(|f| {
            let fname = f.ident.as_ref().expect("named struct field");
            quote! { crate::contract::SlotField::member_id(&self.#fname) }
        })
        .collect();
    let (impl_g, ty_g, where_g) = input.generics.split_for_impl();
    quote! {
        impl #impl_g #name #ty_g #where_g {
            /// GENERATED by `#[derive(Slots)]` — one slot per `#[slot]` field, its
            /// admissible set read off the field's `FamilyId::MEMBERS`.
            pub const SLOTS: &'static [crate::contract::Slot] = &[ #( #specs, )* ];
            /// GENERATED by `#[derive(Slots)]` — the resolved slot members in field order.
            pub fn slot_members(
                &self,
            ) -> Vec<crate::engine::indicator_id::IndicatorId> {
                vec![ #( #members, )* ]
            }
        }
    }
    .into()
}

/// `#[derive(ConfigAxes)]` — generate a dual-mode config's cube surface from its fields.
///
/// EVERY field must be a `Param<T>` axis primitive (`Solo` = one value, `Many` = a swept set —
/// uniform across numbers, source fields, and slot members). Emits two inherent methods:
/// - `axes_cube_size()` — the product of every field's axis cardinality (an all-`Solo` config → 1).
/// - `axes_iter()` — the cartesian product of resolved (all-`Solo`) configs, each a concrete cube
///   point ready for `Indicator::create`.
///
/// `#[slot]` fields are INCLUDED (a slot is just a `Param<…SlotOrder>` axis over its members).
/// Materializes the product into a `Vec` — eager is fine: the generator budget-prunes via
/// `axes_cube_size` BEFORE ever calling `axes_iter`, so only small cubes expand (a lazy product
/// is a later optimization).
#[proc_macro_derive(ConfigAxes, attributes(slot))]
pub fn derive_config_axes(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    let name = &input.ident;
    let named: Vec<&syn::Field> = match &input.data {
        syn::Data::Struct(s) => match &s.fields {
            syn::Fields::Named(n) => n.named.iter().collect(),
            _ => Vec::new(),
        },
        _ => {
            return syn::Error::new_spanned(&input, "#[derive(ConfigAxes)] is only for structs")
                .to_compile_error()
                .into()
        }
    };

    // Every config field is a `Param<T>` (Solo = one value, Many = a swept set). A `#[slot]`
    // field is just a `Param<…SlotOrder>` — no special case; the cube is built uniformly.
    let mut fnames: Vec<syn::Ident> = Vec::new();
    for f in &named {
        let fname = f.ident.clone().expect("named struct field");
        let seg = match &f.ty {
            syn::Type::Path(tp) => tp
                .path
                .segments
                .last()
                .map(|s| s.ident.to_string())
                .unwrap_or_default(),
            _ => String::new(),
        };
        if seg != "Param" {
            return syn::Error::new_spanned(
                &f.ty,
                format!(
                    "#[derive(ConfigAxes)] field `{}` must be a `Param<T>`, got `{}`",
                    fname, seg
                ),
            )
            .to_compile_error()
            .into();
        }
        fnames.push(fname);
    }

    let card_terms: Vec<TokenStream2> = fnames
        .iter()
        .map(|fname| quote! { .saturating_mul(self.#fname.cardinality()) })
        .collect();

    // Mixed-radix LAZY decode: `idx` → the `idx`-th cube point WITHOUT materializing the product.
    // Radix order MUST match the former nested loops (FIRST field most-significant, LAST least),
    // so `axes_decode(k)` ≡ the old `axes_iter().nth(k)`. Axis indices are therefore extracted in
    // REVERSE field order (last field = least-significant digit), then the config is rebuilt forward.
    let axis_idx_vars: Vec<syn::Ident> =
        fnames.iter().map(|n| format_ident!("__ai_{}", n)).collect();
    let decode_lets: Vec<TokenStream2> = fnames
        .iter()
        .zip(&axis_idx_vars)
        .rev()
        .map(|(fname, ai)| {
            quote! {
                let #ai = {
                    let __c = self.#fname.cardinality();
                    let __a = (__rem % __c) as usize;
                    __rem /= __c;
                    __a
                };
            }
        })
        .collect();
    // each resolved config sets every field to a `Solo` of its decoded axis value.
    let decode_ctor: Vec<TokenStream2> = fnames
        .iter()
        .zip(&axis_idx_vars)
        .map(|(fname, ai)| quote! { #fname: crate::contract::Param::Solo(self.#fname.value_at(#ai)) })
        .collect();

    // PRIMARY period axis = the FIRST `Param<usize>` field (the universal lookback the generator
    // overlays). `None` for a periodless config → the inherent setter is a no-op (and the trait
    // default no-op also covers field-less configs that don't derive this).
    fn inner_is_usize(ty: &syn::Type) -> bool {
        if let syn::Type::Path(tp) = ty {
            if let Some(seg) = tp.path.segments.last() {
                if seg.ident == "Param" {
                    if let syn::PathArguments::AngleBracketed(ab) = &seg.arguments {
                        if let Some(syn::GenericArgument::Type(syn::Type::Path(itp))) = ab.args.first() {
                            return itp.path.is_ident("usize");
                        }
                    }
                }
            }
        }
        false
    }

    // Returns the terminal ident of the Param<T> inner type, or None if detection fails.
    fn inner_ident(ty: &syn::Type) -> Option<String> {
        if let syn::Type::Path(tp) = ty {
            if let Some(seg) = tp.path.segments.last() {
                if seg.ident == "Param" {
                    if let syn::PathArguments::AngleBracketed(ab) = &seg.arguments {
                        if let Some(syn::GenericArgument::Type(syn::Type::Path(itp))) = ab.args.first() {
                            return itp.path.segments.last().map(|s| s.ident.to_string());
                        }
                    }
                }
            }
        }
        None
    }

    // Machine-default expression per field: wide Many for usize/bool/OhlcvField/smoother-slot;
    // Solo default for everything else. A `#[slot] Param<SmootherChoice>` field sweeps WHICH
    // family member fills the slot (all members, host-following period) via
    // `SmootherChoice::machine_sweep()`; other slot order types (OscillatorSlotOrder /
    // LineProducerOrder / Vec) stay Solo for now (handled in a later pass).
    let machine_fields: Vec<TokenStream2> = named.iter().map(|f| {
        let fname = f.ident.as_ref().expect("named struct field");
        let is_slot = f.attrs.iter().any(|a| a.path().is_ident("slot"));
        if is_slot {
            return match inner_ident(&f.ty).as_deref() {
                // Thin choice (member + Follow/Own period, no shape) — un-migrated smoother slots.
                Some("SmootherChoice") => quote! {
                    #fname: crate::contract::Param::many(
                        crate::engine::contract_engine::SmootherChoice::machine_sweep()
                    )
                },
                // Rich orders (member + full source-less params incl. shape) — the target form.
                Some("SmootherSlotOrder") => quote! {
                    #fname: crate::contract::Param::many(
                        crate::engine::contract_engine::SmootherSlotOrder::machine_sweep()
                    )
                },
                Some("OscillatorSlotOrder") => quote! {
                    #fname: crate::contract::Param::many(
                        crate::engine::contract_engine::OscillatorSlotOrder::machine_sweep()
                    )
                },
                // Vec<…> slot sets (Confluence) / LineProducerOrder (recursive) → Solo for now.
                _ => quote! { #fname: d.#fname },
            };
        }
        match inner_ident(&f.ty).as_deref() {
            Some("usize") => quote! {
                #fname: crate::contract::Param::range(1, 10000, 1)
            },
            Some("bool") => quote! {
                #fname: crate::contract::Param::many(vec![false, true])
            },
            Some("OhlcvField") => quote! {
                #fname: crate::contract::Param::many(vec![
                    crate::engine::ohlcv_field::OhlcvField::Open,
                    crate::engine::ohlcv_field::OhlcvField::High,
                    crate::engine::ohlcv_field::OhlcvField::Low,
                    crate::engine::ohlcv_field::OhlcvField::Close,
                    crate::engine::ohlcv_field::OhlcvField::Volume,
                    crate::engine::ohlcv_field::OhlcvField::HL2,
                    crate::engine::ohlcv_field::OhlcvField::HLC3,
                    crate::engine::ohlcv_field::OhlcvField::OHLC4,
                ])
            },
            // f64, u32, arrays, enums, slot orders — keep Solo default.
            _ => quote! { #fname: d.#fname },
        }
    }).collect();
    let set_period_body = match named.iter().find(|f| inner_is_usize(&f.ty)) {
        Some(f) => {
            let fname = f.ident.as_ref().expect("named struct field");
            quote! { self.#fname = period; }
        }
        None => quote! { let _ = period; },
    };
    let get_period_body = match named.iter().find(|f| inner_is_usize(&f.ty)) {
        Some(f) => {
            let fname = f.ident.as_ref().expect("named struct field");
            quote! { Some(self.#fname.resolved()) }
        }
        None => quote! { None },
    };
    let get_period_axis_body = match named.iter().find(|f| inner_is_usize(&f.ty)) {
        Some(f) => {
            let fname = f.ident.as_ref().expect("named struct field");
            quote! { Some(self.#fname.clone()) }
        }
        None => quote! { None },
    };

    let (impl_g, ty_g, where_g) = input.generics.split_for_impl();
    quote! {
        impl #impl_g #name #ty_g #where_g {
            /// GENERATED by `#[derive(ConfigAxes)]` — the product of every field's axis cardinality.
            pub fn axes_cube_size(&self) -> u128 {
                1u128 #( #card_terms )*
            }
            /// GENERATED by `#[derive(ConfigAxes)]` — the `idx`-th cube point (mixed-radix over the
            /// field axes), O(1) and allocation-free. `axes_decode(k)` ≡ the former
            /// `axes_iter().nth(k)`: FIRST field most-significant, LAST least (matches the old
            /// nested-loop push order). The lazy primitive the cube enumerator + warmup decode on.
            #[allow(unused_mut, unused_variables, unused_assignments)]
            pub fn axes_decode(&self, idx: u128) -> Self {
                let mut __rem = idx;
                #( #decode_lets )*
                let _ = __rem;
                #name { #( #decode_ctor ),* }
            }
            /// GENERATED by `#[derive(ConfigAxes)]` — the cartesian product of resolved configs,
            /// LAZY: a [`crate::contract::CubeIter`] streaming `axes_decode` (O(1) `next`/`nth`,
            /// NEVER materializes the product into a `Vec` — a wide machine-default cube no longer
            /// OOMs on expansion; consumers `.take`/`.nth` only the points they actually visit).
            pub fn axes_iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
                let __total = self.axes_cube_size();
                Box::new(crate::contract::CubeIter::new(__total, move |__i| self.axes_decode(__i)))
            }
            /// GENERATED by `#[derive(ConfigAxes)]` — INHERENT override of the primary period axis:
            /// set the first `Param<usize>` field to `period` (no-op if the config has none). By
            /// Rust's inherent-over-trait resolution this shadows [`crate::contract::Config::
            /// set_primary_period`]'s default for period-bearing configs.
            pub fn set_primary_period(&mut self, period: crate::contract::Param<usize>) {
                #set_period_body
            }
            /// GENERATED by `#[derive(ConfigAxes)]` — the resolved primary period (first
            /// `Param<usize>` field), `None` if periodless. Shadows the `Config` trait default.
            pub fn primary_period(&self) -> Option<usize> {
                #get_period_body
            }
            /// GENERATED by `#[derive(ConfigAxes)]` — the RAW primary-period axis (first
            /// `Param<usize>` field, `Solo` or a swept `Many`), `None` if periodless. Shadows the
            /// `Config` trait default — the round-trip primitive a re-emitter needs to print
            /// `with_period(axis)` including a swept range, not just one resolved scalar.
            pub fn primary_period_axis(&self) -> Option<crate::contract::Param<usize>> {
                #get_period_axis_body
            }
            /// GENERATED by `#[derive(ConfigAxes)]` — a 64-bit fingerprint of this config's
            /// `Param` fields: the TYPED replacement for the OSS stringly `param_hash`. Hashes
            /// every field via [`crate::contract::ParamScalar`] (so `Param<f64>` folds by bits).
            /// The warmup keys an indicator column on `(IndicatorId, config_hash, output)` — exact
            /// structural id/output + this param fingerprint — to dedup identical columns.
            pub fn config_hash(&self) -> u64 {
                use ::core::hash::{Hash as _, Hasher as _};
                let mut __h = ::std::collections::hash_map::DefaultHasher::new();
                #( self.#fnames.hash(&mut __h); )*
                __h.finish()
            }

            /// GENERATED by `#[derive(ConfigAxes)]` — machine-generator sweep defaults.
            ///
            /// Starts from [`crate::contract::Config::defaults`] (the Solo render defaults) and
            /// widens type-driven axes to `Many`:
            /// - `Param<usize>` → `Param::range(1, 10000, 1)` (full numeric sweep; no fixed-buffer
            ///   cap — window storage is a dynamic `Vec`, so period is bounded only by this
            ///   recommended ceiling, not a runtime assert)
            /// - `Param<bool>`  → `Param::many(vec![false, true])`
            /// - `Param<OhlcvField>` → `Param::many([all 8 variants])`
            /// - slots, `Param<f64>`, `Param<u32>`, arrays, enums → unchanged Solo default
            ///
            /// f64 per-axis step overrides are a later pass; this pass is purely type-driven.
            pub fn machine_defaults_auto() -> Self {
                let d = <Self as crate::contract::Config>::defaults();
                Self {
                    #( #machine_fields ),*
                }
            }
        }
    }
    .into()
}

/// `#[derive(ParamScalar)]` — make an enum / order usable inside `Param<T>` for config hashing.
/// Emits `impl ParamScalar` routing through the type's own `Hash` (so the type must also derive
/// `Hash`). `f64` and the numeric primitives get explicit impls in `contract::axis`; this derive
/// covers the categorical axes (`SmootherChoice`, `OhlcvField`, slot orders, …).
#[proc_macro_derive(ParamScalar)]
pub fn derive_param_scalar(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    let name = &input.ident;
    let (impl_g, ty_g, where_g) = input.generics.split_for_impl();
    quote! {
        impl #impl_g crate::contract::ParamScalar for #name #ty_g #where_g {
            fn scalar_hash<H: ::core::hash::Hasher>(&self, state: &mut H) {
                ::core::hash::Hash::hash(self, state);
            }
        }
    }
    .into()
}
