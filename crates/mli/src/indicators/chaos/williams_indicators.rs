//! Williams Indicators - индикаторы Билла Вильямса для анализа хаоса
//! Включает Alligator, Awesome Oscillator, Acceleration/Deceleration и Market Facilitation Index

use crate::engine::contract_engine::SmootherId;
use crate::engine::contract_engine::SmootherSlot;

/// Alligator - индикатор "Аллигатор" Билла Вильямса
/// Состоит из трех сглаженных скользящих средних с разными периодами и смещениями
#[derive(Debug, Clone)]
pub struct Alligator {
    // Три линии аллигатора
    jaw: SmootherSlot,    // Челюсть (синяя линия) - 13-периодная SMA, смещение 8
    teeth: SmootherSlot,  // Зубы (красная линия) - 8-периодная SMA, смещение 5
    lips: SmootherSlot,   // Губы (зеленая линия) - 5-периодная SMA, смещение 3
    
    // Буферы для смещений
    jaw_buffer: Vec<f64>,
    teeth_buffer: Vec<f64>,
    lips_buffer: Vec<f64>,
    
    // Смещения
    jaw_offset: usize,
    teeth_offset: usize,
    lips_offset: usize,
    
    // Состояние
    is_ready: bool,
}

impl Default for Alligator {
    fn default() -> Self {
        Self::new()
    }
}

impl Alligator {
    pub fn new() -> Self {
        Self {
            jaw: SmootherSlot::new(SmootherId::Sma, 13),
            teeth: SmootherSlot::new(SmootherId::Sma, 8),
            lips: SmootherSlot::new(SmootherId::Sma, 5),
            jaw_buffer: Vec::with_capacity(32),
            teeth_buffer: Vec::with_capacity(32),
            lips_buffer: Vec::with_capacity(32),
            jaw_offset: 8,
            teeth_offset: 5,
            lips_offset: 3,
            is_ready: false,
        }
    }
    
    /// Обновить индикатор новой ценой (обычно медианная цена = (H+L)/2)
    pub fn update(&mut self, price: f64) -> (f64, f64, f64) {
        // Обновляем скользящие средние (используем update_bar с ценой как close)
        let jaw_value = self.jaw.feed(price);
        let teeth_value = self.teeth.feed(price);
        let lips_value = self.lips.feed(price);
        
        // Добавляем в буферы для смещения
        if self.jaw_buffer.len() >= self.jaw_offset {
            self.jaw_buffer.remove(0);
        }
        self.jaw_buffer.push(jaw_value);
        
        if self.teeth_buffer.len() >= self.teeth_offset {
            self.teeth_buffer.remove(0);
        }
        self.teeth_buffer.push(teeth_value);
        
        if self.lips_buffer.len() >= self.lips_offset {
            self.lips_buffer.remove(0);
        }
        self.lips_buffer.push(lips_value);
        
        // Проверяем готовность
        if self.jaw_buffer.len() >= self.jaw_offset && 
           self.teeth_buffer.len() >= self.teeth_offset &&
           self.lips_buffer.len() >= self.lips_offset {
            self.is_ready = true;
        }
        
        self.get_values()
    }
    
    /// Получить текущие значения линий (с учетом смещения)
    /// Offset означает "сколько баров назад" - берём последнее значение в буфере
    pub fn get_values(&self) -> (f64, f64, f64) {
        let jaw = if self.jaw_buffer.len() >= self.jaw_offset {
            // Берём значение с offset баров назад (последнее в буфере = текущее смещённое)
            self.jaw_buffer[self.jaw_buffer.len() - self.jaw_offset]
        } else {
            self.jaw.value()
        };

        let teeth = if self.teeth_buffer.len() >= self.teeth_offset {
            self.teeth_buffer[self.teeth_buffer.len() - self.teeth_offset]
        } else {
            self.teeth.value()
        };

        let lips = if self.lips_buffer.len() >= self.lips_offset {
            self.lips_buffer[self.lips_buffer.len() - self.lips_offset]
        } else {
            self.lips.value()
        };

        (jaw, teeth, lips)
    }
    
    /// Определить состояние аллигатора
    pub fn alligator_state(&self) -> &'static str {
        let (jaw, teeth, lips) = self.get_values();
        
        if lips > teeth && teeth > jaw {
            "Hunting (Uptrend)"
        } else if lips < teeth && teeth < jaw {
            "Hunting (Downtrend)"
        } else {
            "Sleeping (Sideways)"
        }
    }
    
    /// Получить торговый сигнал
    pub fn trading_signal(&self, current_price: f64) -> i8 {
        let (jaw, teeth, lips) = self.get_values();
        
        if lips > teeth && teeth > jaw && current_price > lips {
            1 // Покупка
        } else if lips < teeth && teeth < jaw && current_price < lips {
            -1 // Продажа
        } else {
            0 // Нейтрально
        }
    }
    
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }

    /// Feed the resolved `[high, low]` lanes (in `SOURCE` order); uses median price (H+L)/2.
    pub fn feed(&mut self, lanes: &[f64]) {
        let median_price = (lanes[0] + lanes[1]) / 2.0;
        self.update(median_price);
    }


    /// Named getter for the `jaw` brace output (Alligator jaw line).
    pub fn jaw(&self) -> f64 {
        let (jaw, _, _) = self.get_values();
        jaw
    }

    /// Named getter for the `teeth` brace output (Alligator teeth line).
    pub fn teeth(&self) -> f64 {
        let (_, teeth, _) = self.get_values();
        teeth
    }

    /// Named getter for the `lips` brace output (Alligator lips line).
    pub fn lips(&self) -> f64 {
        let (_, _, lips) = self.get_values();
        lips
    }

    pub fn reset(&mut self) {
        self.jaw = SmootherSlot::new(SmootherId::Sma, 13);
        self.teeth = SmootherSlot::new(SmootherId::Sma, 8);
        self.lips = SmootherSlot::new(SmootherId::Sma, 5);
        self.jaw_buffer.clear();
        self.teeth_buffer.clear();
        self.lips_buffer.clear();
        self.is_ready = false;
    }
}

/// Awesome Oscillator - индикатор AO Билла Вильямса
/// Разность между 5-периодной и 34-периодной простыми скользящими средними медианных цен
#[derive(Debug, Clone)]
pub struct AwesomeOscillator {
    sma5: SmootherSlot,
    sma34: SmootherSlot,
    
    // Буфер значений для анализа
    ao_values: Vec<f64>,
    
    // Результат
    ao_value: f64,
    
    is_ready: bool,
}

impl Default for AwesomeOscillator {
    fn default() -> Self {
        Self::new()
    }
}

impl AwesomeOscillator {
    pub fn new() -> Self {
        Self {
            sma5: SmootherSlot::new(SmootherId::Sma, 5),
            sma34: SmootherSlot::new(SmootherId::Sma, 34),
            ao_values: Vec::with_capacity(512),
            ao_value: 0.0,
            is_ready: false,
        }
    }
    
    /// Обновить индикатор медианной ценой (H+L)/2
    pub fn update(&mut self, median_price: f64) -> f64 {
        let sma5_val = self.sma5.feed(median_price);
        let sma34_val = self.sma34.feed(median_price);
        
        self.ao_value = sma5_val - sma34_val;
        
        // Добавляем в буфер для анализа
        if self.ao_values.len() >= 512 {
            self.ao_values.remove(0);
        }
        self.ao_values.push(self.ao_value);
        
        if self.sma34.is_ready() {
            self.is_ready = true;
        }
        
        self.ao_value
    }
    
    /// Получить текущее значение AO
    pub fn value(&self) -> f64 {
        self.ao_value
    }
    
    /// Определить сигнал "блюдце" (saucer)
    pub fn saucer_signal(&self) -> i8 {
        if self.ao_values.len() < 3 {
            return 0;
        }
        
        let len = self.ao_values.len();
        let current = self.ao_values[len - 1];
        let prev1 = self.ao_values[len - 2];
        let prev2 = self.ao_values[len - 3];
        
        // Бычье блюдце: все значения выше нуля, средний бар ниже соседних
        if current > 0.0 && prev1 > 0.0 && prev2 > 0.0 &&
           prev1 < prev2 && current > prev1 {
            return 1;
        }
        
        // Медвежье блюдце: все значения ниже нуля, средний бар выше соседних
        if current < 0.0 && prev1 < 0.0 && prev2 < 0.0 &&
           prev1 > prev2 && current < prev1 {
            return -1;
        }
        
        0
    }
    
    /// Определить сигнал пересечения нулевой линии
    pub fn zero_line_cross(&self) -> i8 {
        if self.ao_values.len() < 2 {
            return 0;
        }
        
        let len = self.ao_values.len();
        let current = self.ao_values[len - 1];
        let prev = self.ao_values[len - 2];
        
        if prev <= 0.0 && current > 0.0 {
            1 // Пересечение снизу вверх
        } else if prev >= 0.0 && current < 0.0 {
            -1 // Пересечение сверху вниз
        } else {
            0
        }
    }
    
    /// Получить общий торговый сигнал
    pub fn trading_signal(&self) -> i8 {
        let saucer = self.saucer_signal();
        let zero_cross = self.zero_line_cross();
        
        if saucer != 0 {
            saucer // Приоритет сигналу блюдца
        } else {
            zero_cross
        }
    }
    
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }

    /// Feed the resolved `[high, low]` lanes (in `SOURCE` order); uses median price (H+L)/2.
    pub fn feed(&mut self, lanes: &[f64]) {
        let median_price = (lanes[0] + lanes[1]) / 2.0;
        self.update(median_price);
    }

    pub fn reset(&mut self) {
        self.sma5 = SmootherSlot::new(SmootherId::Sma, 5);
        self.sma34 = SmootherSlot::new(SmootherId::Sma, 34);
        self.ao_values.clear();
        self.ao_value = 0.0;
        self.is_ready = false;
    }
}

/// Acceleration/Deceleration - индикатор AC Билла Вильямса
/// Разность между AO и его 5-периодной простой скользящей средней
#[derive(Debug, Clone)]
pub struct AccelerationDeceleration {
    awesome_oscillator: AwesomeOscillator,
    ao_sma: SmootherSlot,
    
    ac_value: f64,
    ac_values: Vec<f64>,
    
    is_ready: bool,
}

impl Default for AccelerationDeceleration {
    fn default() -> Self {
        Self::new()
    }
}

impl AccelerationDeceleration {
    pub fn new() -> Self {
        Self {
            awesome_oscillator: AwesomeOscillator::new(),
            ao_sma: SmootherSlot::new(SmootherId::Sma, 5),
            ac_value: 0.0,
            ac_values: Vec::with_capacity(512),
            is_ready: false,
        }
    }
    
    /// Обновить индикатор медианной ценой
    pub fn update(&mut self, median_price: f64) -> f64 {
        let ao_val = self.awesome_oscillator.update(median_price);
        let ao_sma_val = self.ao_sma.feed(ao_val);
        
        self.ac_value = ao_val - ao_sma_val;
        
        if self.ac_values.len() >= 512 {
            self.ac_values.remove(0);
        }
        self.ac_values.push(self.ac_value);
        
        if self.ao_sma.is_ready() {
            self.is_ready = true;
        }
        
        self.ac_value
    }
    
    /// Получить текущее значение AC
    pub fn value(&self) -> f64 {
        self.ac_value
    }
    
    /// Определить сигнал изменения цвета (смена направления)
    pub fn color_change_signal(&self) -> i8 {
        if self.ac_values.len() < 2 {
            return 0;
        }
        
        let len = self.ac_values.len();
        let current = self.ac_values[len - 1];
        let prev = self.ac_values[len - 2];
        
        if prev < 0.0 && current > 0.0 {
            1 // Смена с красного на зеленый
        } else if prev > 0.0 && current < 0.0 {
            -1 // Смена с зеленого на красный
        } else {
            0
        }
    }
    
    /// Получить торговый сигнал
    pub fn trading_signal(&self) -> i8 {
        self.color_change_signal()
    }
    
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }

    /// Feed the resolved `[high, low]` lanes (in `SOURCE` order); uses median price (H+L)/2.
    pub fn feed(&mut self, lanes: &[f64]) {
        let median_price = (lanes[0] + lanes[1]) / 2.0;
        self.update(median_price);
    }

    pub fn reset(&mut self) {
        self.awesome_oscillator.reset();
        self.ao_sma = SmootherSlot::new(SmootherId::Sma, 5);
        self.ac_value = 0.0;
        self.ac_values.clear();
        self.is_ready = false;
    }
}

/// Market Facilitation Index - индикатор MFI Билла Вильямса
/// Измеряет эффективность движения цены на единицу объема
#[derive(Debug, Clone)]
pub struct MarketFacilitationIndex {
    mfi_values: Vec<f64>,
    volume_values: Vec<f64>,
    
    mfi_value: f64,
    
    is_ready: bool,
}

impl Default for MarketFacilitationIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl MarketFacilitationIndex {
    pub fn new() -> Self {
        Self {
            mfi_values: Vec::with_capacity(512),
            volume_values: Vec::with_capacity(512),
            mfi_value: 0.0,
            is_ready: false,
        }
    }
    
    /// Feed resolved input lanes — `[high, low, volume]` (factory resolves fixed H/L/V slice).
    pub fn feed(&mut self, lanes: &[f64]) -> f64 {
        let high = lanes[0];
        let low = lanes[1];
        let volume = lanes[2];
        self.update(high, low, volume)
    }

    /// Обновить индикатор новым баром
    pub fn update(&mut self, high: f64, low: f64, volume: f64) -> f64 {
        // MFI = (High - Low) / Volume
        self.mfi_value = if volume > 0.0 {
            (high - low) / volume
        } else {
            0.0
        };
        
        // Сохраняем для анализа
        if self.mfi_values.len() >= 512 {
            self.mfi_values.remove(0);
        }
        self.mfi_values.push(self.mfi_value);
        
        if self.volume_values.len() >= 512 {
            self.volume_values.remove(0);
        }
        self.volume_values.push(volume);
        
        if self.mfi_values.len() >= 2 {
            self.is_ready = true;
        }
        
        self.mfi_value
    }
    
    /// Получить текущее значение MFI
    pub fn value(&self) -> f64 {
        self.mfi_value
    }
    
    /// Определить тип бара по Вильямсу
    pub fn bar_type(&self) -> &'static str {
        if self.mfi_values.len() < 2 || self.volume_values.len() < 2 {
            return "Insufficient Data";
        }
        
        let len = self.mfi_values.len();
        let current_mfi = self.mfi_values[len - 1];
        let prev_mfi = self.mfi_values[len - 2];
        let current_volume = self.volume_values[len - 1];
        let prev_volume = self.volume_values[len - 2];
        
        let mfi_up = current_mfi > prev_mfi;
        let volume_up = current_volume > prev_volume;
        
        match (mfi_up, volume_up) {
            (true, true) => "Green (Trend Acceleration)",
            (false, false) => "Fade (Trend Deceleration)",
            (true, false) => "Fake (False Breakout)",
            (false, true) => "Squat (Accumulation/Distribution)",
        }
    }
    
    /// Получить торговый сигнал на основе типа бара
    pub fn trading_signal(&self) -> i8 {
        match self.bar_type() {
            "Green (Trend Acceleration)" => 1,
            "Fade (Trend Deceleration)" => 0,
            "Fake (False Breakout)" => -1,
            "Squat (Accumulation/Distribution)" => 0,
            _ => 0,
        }
    }
    
    /// Получить силу сигнала
    pub fn signal_strength(&self) -> f64 {
        match self.bar_type() {
            "Green (Trend Acceleration)" => 1.0,
            "Squat (Accumulation/Distribution)" => 0.7,
            "Fake (False Breakout)" => 0.5,
            "Fade (Trend Deceleration)" => 0.3,
            _ => 0.0,
        }
    }
    
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }
    
    pub fn reset(&mut self) {
        self.mfi_values.clear();
        self.volume_values.clear();
        self.mfi_value = 0.0;
        self.is_ready = false;
    }
}

// ---- Indicator contract for MarketFacilitationIndex ----

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Cost, Family, Indicator, Output, SourceAxis, Store, StoreKind, UpdateComplexity,
};
use crate::contract::Render;
use crate::contract::{Color, RenderOutput, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Unit config — Williams MFI has no user-tunable parameters.
#[derive(Debug, Clone, Copy, PartialEq, mli_contract_macros::ConfigAxes)]
pub struct WilliamsMfiConfig;

impl Indicator for MarketFacilitationIndex {
    const ID: IndicatorId = IndicatorId::WilliamsMfi;
    /// Volume-based efficiency measure — not a pluggable family member.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Fixed H/L/V triple — no configurable source field.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::High,
        OhlcvField::Low,
        OhlcvField::Volume,
    ]));
    /// O(1) per bar — only scalar state + two bounded Vec histories.
    const COST: Cost = Cost::new(
        UpdateComplexity::Constant,
        &[Store::window(StoreKind::Vec), Store::window(StoreKind::Vec)],
    );
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::WilliamsMfi)];

    type Config = WilliamsMfiConfig;
    type Runtime = MarketFacilitationIndex;

    fn create(_cfg: WilliamsMfiConfig) -> MarketFacilitationIndex {
        MarketFacilitationIndex::new()
    }
}

impl crate::contract::Config for WilliamsMfiConfig {
    fn defaults() -> Self {
        WilliamsMfiConfig
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // Unit struct — no axes to sweep.
        Self::machine_defaults_auto()
    }
}


impl Render for MarketFacilitationIndex {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .output(RenderOutput::histogram(
                IndicatorOutputId::WilliamsMfi,
                "Williams MFI",
                Color::hex(0x00BCD4),
            ))
            .histogram_style(crate::contract::HistogramStyle::FromBottom)
            .precision(4)
            .build()
    }
}

// ---- Indicator contract for AwesomeOscillator ----

/// Unit config — Awesome Oscillator has no user-tunable parameters.
#[derive(Debug, Clone, Copy, PartialEq, mli_contract_macros::ConfigAxes)]
pub struct AoConfig;

impl Indicator for AwesomeOscillator {
    const ID: IndicatorId = IndicatorId::Ao;
    /// Chaos-family oscillator — not a pluggable family member.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Fixed `[high, low]` lanes — Bill Williams indicators run on median price (H+L)/2.
    const SOURCE: Option<SourceAxis> =
        Some(SourceAxis::KlineSlice(&[OhlcvField::High, OhlcvField::Low]));
    /// O(1) per bar — running SMA state + bounded history Vec.
    const COST: Cost = Cost::new(
        UpdateComplexity::Constant,
        &[Store::window(StoreKind::Vec)],
    );
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::Ao)];

    type Config = AoConfig;
    type Runtime = AwesomeOscillator;

    fn create(_cfg: AoConfig) -> AwesomeOscillator {
        AwesomeOscillator::new()
    }
}

impl crate::contract::Config for AoConfig {
    fn defaults() -> Self {
        AoConfig
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // Unit struct — no axes to sweep.
        Self::machine_defaults_auto()
    }
}


impl Render for AwesomeOscillator {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .output(RenderOutput::histogram(
                IndicatorOutputId::Ao,
                "Awesome Oscillator",
                Color::hex(0x4CAF50),
            ))
            .histogram_style(crate::contract::HistogramStyle::Centered)
            .zero_baseline()
            .precision(4)
            .build()
    }
}

// ---- Indicator contract for AccelerationDeceleration ----

/// Unit config — Acceleration/Deceleration has no user-tunable parameters.
#[derive(Debug, Clone, Copy, PartialEq, mli_contract_macros::ConfigAxes)]
pub struct AcConfig;

impl Indicator for AccelerationDeceleration {
    const ID: IndicatorId = IndicatorId::Ac;
    /// Chaos-family oscillator — not a pluggable family member.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Fixed `[high, low]` lanes — Bill Williams indicators run on median price (H+L)/2.
    const SOURCE: Option<SourceAxis> =
        Some(SourceAxis::KlineSlice(&[OhlcvField::High, OhlcvField::Low]));
    /// O(1) per bar — running SMA state + bounded history Vec.
    const COST: Cost = Cost::new(
        UpdateComplexity::Constant,
        &[Store::window(StoreKind::Vec)],
    );
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::Ac)];

    type Config = AcConfig;
    type Runtime = AccelerationDeceleration;

    fn create(_cfg: AcConfig) -> AccelerationDeceleration {
        AccelerationDeceleration::new()
    }
}

impl crate::contract::Config for AcConfig {
    fn defaults() -> Self {
        AcConfig
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // Unit struct — no axes to sweep.
        Self::machine_defaults_auto()
    }
}


impl Render for AccelerationDeceleration {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .output(RenderOutput::histogram(
                IndicatorOutputId::Ac,
                "Accelerator",
                Color::hex(0x00BCD4),
            ))
            .histogram_style(crate::contract::HistogramStyle::Centered)
            .zero_baseline()
            .precision(4)
            .build()
    }
}

// ---- Indicator contract for Alligator ----

/// Unit config — Alligator uses fixed periods (13/8/5) and offsets (8/5/3).
#[derive(Debug, Clone, Copy, PartialEq, mli_contract_macros::ConfigAxes)]
pub struct AlligatorConfig;

impl Indicator for Alligator {
    const ID: IndicatorId = IndicatorId::Alligator;
    /// Chaos-family trend overlay — not a pluggable family member.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Fixed `[high, low]` lanes — Bill Williams indicators run on median price (H+L)/2.
    const SOURCE: Option<SourceAxis> =
        Some(SourceAxis::KlineSlice(&[OhlcvField::High, OhlcvField::Low]));
    /// O(1) per bar — three running SMA slots + three bounded offset buffers (Vec).
    const COST: Cost = Cost::new(
        UpdateComplexity::Constant,
        &[
            Store::window(StoreKind::Vec),
            Store::window(StoreKind::Vec),
            Store::window(StoreKind::Vec),
        ],
    );
    const OUTPUTS: &'static [Output] = &[
        Output::price(IndicatorOutputId::AlligatorJaw),
        Output::price(IndicatorOutputId::AlligatorTeeth),
        Output::price(IndicatorOutputId::AlligatorLips),
    ];

    type Config = AlligatorConfig;
    type Runtime = Alligator;

    fn create(_cfg: AlligatorConfig) -> Alligator {
        Alligator::new()
    }
}

impl crate::contract::Config for AlligatorConfig {
    fn defaults() -> Self {
        AlligatorConfig
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // Unit struct — fixed periods (13/8/5) and offsets (8/5/3), no axes to sweep.
        Self::machine_defaults_auto()
    }
}


impl Render for Alligator {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .output(RenderOutput::line(
                IndicatorOutputId::AlligatorJaw,
                "Jaw",
                Color::hex(0x2196F3),
                2.0,
            ))
            .output(RenderOutput::line(
                IndicatorOutputId::AlligatorTeeth,
                "Teeth",
                Color::hex(0xF44336),
                1.5,
            ))
            .output(RenderOutput::line(
                IndicatorOutputId::AlligatorLips,
                "Lips",
                Color::hex(0x4CAF50),
                1.0,
            ))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alligator_creation() {
        let ind = Alligator::new();
        assert!(!ind.is_ready());
    }

    #[test]
    fn test_alligator_warmup() {
        let mut ind = Alligator::new();
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ind.update(price);
        }
        assert!(ind.is_ready());
    }

    #[test]
    fn test_alligator_values_finite() {
        let mut ind = Alligator::new();
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let (jaw, teeth, lips) = ind.update(price);
            assert!(jaw.is_finite());
            assert!(teeth.is_finite());
            assert!(lips.is_finite());
        }
    }

    #[test]
    fn test_alligator_reset() {
        let mut ind = Alligator::new();
        for i in 0..25 {
            ind.update(100.0 + i as f64);
        }
        ind.reset();
        assert!(!ind.is_ready());
    }

    #[test]
    fn test_awesome_oscillator_creation() {
        let ind = AwesomeOscillator::new();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), 0.0);
    }

    #[test]
    fn test_awesome_oscillator_warmup() {
        let mut ind = AwesomeOscillator::new();
        for i in 0..40 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ind.update(price);
        }
        assert!(ind.is_ready());
    }

    #[test]
    fn test_awesome_oscillator_values_finite() {
        let mut ind = AwesomeOscillator::new();
        for i in 0..50 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let ao = ind.update(price);
            assert!(ao.is_finite());
        }
    }

    #[test]
    fn test_awesome_oscillator_reset() {
        let mut ind = AwesomeOscillator::new();
        for i in 0..40 {
            ind.update(100.0 + i as f64);
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), 0.0);
    }

    #[test]
    fn test_acceleration_deceleration_creation() {
        let ind = AccelerationDeceleration::new();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), 0.0);
    }

    #[test]
    fn test_acceleration_deceleration_warmup() {
        let mut ind = AccelerationDeceleration::new();
        for i in 0..45 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ind.update(price);
        }
        assert!(ind.is_ready());
    }

    #[test]
    fn test_acceleration_deceleration_values_finite() {
        let mut ind = AccelerationDeceleration::new();
        for i in 0..50 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let ac = ind.update(price);
            assert!(ac.is_finite());
        }
    }

    #[test]
    fn test_acceleration_deceleration_reset() {
        let mut ind = AccelerationDeceleration::new();
        for i in 0..45 {
            ind.update(100.0 + i as f64);
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), 0.0);
    }

    #[test]
    fn test_market_facilitation_index_creation() {
        let ind = MarketFacilitationIndex::new();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), 0.0);
    }

    #[test]
    fn test_market_facilitation_index_warmup() {
        let mut ind = MarketFacilitationIndex::new();
        for i in 0..5 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ind.update(price + 1.0, price - 1.0, 1000.0);
        }
        assert!(ind.is_ready());
    }

    #[test]
    fn test_market_facilitation_index_values_finite() {
        let mut ind = MarketFacilitationIndex::new();
        for i in 0..20 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let mfi = ind.update(price + 1.0, price - 1.0, 1000.0 + i as f64);
            assert!(mfi.is_finite());
        }
    }

    #[test]
    fn test_market_facilitation_index_reset() {
        let mut ind = MarketFacilitationIndex::new();
        for i in 0..10 {
            ind.update(105.0, 95.0, 1000.0 + i as f64);
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_williams_mfi() {
        use crate::contract::MarketSample;
        use crate::engine::contract_engine::IndicatorOrder;
        let mut f = IndicatorOrder::WilliamsMfi(<<MarketFacilitationIndex as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for i in 0..10 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            f.feed(0, MarketSample::Bar {
                open: 9999.0, // not used — SOURCE is High/Low/Volume
                high: price + 2.0,
                low: price - 2.0,
                close: 9999.0,
                volume: 1000.0 + i as f64,
            });
        }
        assert!(f.read(IndicatorOutputId::WilliamsMfi).is_finite());
    }

    #[test]
    fn factory_feeds_resolved_ao() {
        use crate::contract::MarketSample;
        use crate::engine::contract_engine::IndicatorOrder;
        let mut f = IndicatorOrder::Ao(<<AwesomeOscillator as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for i in 0..50 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            // close=9999.0 is not used — update_bar only reads high and low
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
                high: price + 1.0,
                low: price - 1.0,
                close: 9999.0,
                volume: 9999.0,
            });
        }
        assert!(f.read(IndicatorOutputId::Ao).is_finite());
    }

    #[test]
    fn factory_feeds_resolved_ac() {
        use crate::contract::MarketSample;
        use crate::engine::contract_engine::IndicatorOrder;
        let mut f = IndicatorOrder::Ac(<<AccelerationDeceleration as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for i in 0..50 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
                high: price + 1.0,
                low: price - 1.0,
                close: 9999.0,
                volume: 9999.0,
            });
        }
        assert!(f.read(IndicatorOutputId::Ac).is_finite());
    }

    #[test]
    fn factory_feeds_resolved_alligator() {
        use crate::contract::MarketSample;
        use crate::engine::contract_engine::IndicatorOrder;
        let mut f = IndicatorOrder::Alligator(<<Alligator as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
                high: price + 1.0,
                low: price - 1.0,
                close: 9999.0,
                volume: 9999.0,
            });
        }
        // Triple output: first = jaw
        assert!(f.read(IndicatorOutputId::AlligatorJaw).is_finite());
    }
}






















