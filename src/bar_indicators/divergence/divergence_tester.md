// Тестовый модуль для проверки работы DivergenceDetector с ZigZag и индикаторами на случайных OHLCV
// (c) 2024

use super::*;
use crate::indicators::bar_indicators::zigzag::factory::{ZigZagFactory, ZigZagAlgo};
use crate::indicators::bar_indicators::average::rma::Rma;
use rand::Rng;

#[derive(Debug, Clone, Copy)]
pub struct OhlcvBar {
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

/// Генерирует случайный OHLCV-бар на основе предыдущего close
fn random_bar<R: Rng>(prev_close: f64, rng: &mut R) -> OhlcvBar {
    let change = rng.gen_range(-1.0..1.0);
    let open = prev_close + change * 0.2;
    let close = open + rng.gen_range(-0.8..0.8);
    let high = open.max(close) + rng.gen_range(0.0..0.5);
    let low = open.min(close) - rng.gen_range(0.0..0.5);
    let volume = rng.gen_range(1000.0..10000.0);
    OhlcvBar { open, high, low, close, volume }
}

#[test]
fn test_divergence_detector_with_zigzag_and_rma() {
    let mut rng = rand::thread_rng();
    let mut bars = Vec::with_capacity(500);
    let mut prev_close = 100.0;
    for _ in 0..500 {
        let bar = random_bar(prev_close, &mut rng);
        prev_close = bar.close;
        bars.push(bar);
    }

    // ZigZagClassic
    let mut zigzag = ZigZagFactory::create_classic(10, Some(2.0), None);
    // RMA индикатор
    let mut rma = Rma::new(14);
    // Для value_at будем хранить значения rma по индексам
    let mut rma_values = Vec::with_capacity(500);

    // Модуль дивергенций
    let mut div = DivergenceDetector::new();

    for (idx, bar) in bars.iter().enumerate() {
        // Обновляем ZigZag и индикатор
        zigzag.update_bar(bar.open, bar.high, bar.low, bar.close, bar.volume);
        rma.update(bar.close);
        rma_values.push(rma.value());

        // Получаем экстремумы ZigZag
        let swings = zigzag.swings();
        // Функция для доступа к значению индикатора по индексу swing
        let rma_at = |i: usize| if i < rma_values.len() { rma_values[i] } else { 0.0 };
        div.check_and_record(swings, rma_at, idx);
    }

    // Выводим результаты
    println!("Найдено {} сигналов дивергенции", div.signals.len());
    for sig in div.signals.iter() {
        println!("bar={} swing={} price={:.2} ind={:.2} type={:?}", sig.bar_idx, sig.swing_idx, sig.price, sig.indicator, sig.dtype);
    }
}
