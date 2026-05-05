//! entropy_catalog.rs: Auto-generated indicator catalog for entropy indicators
//!
//! Generated for Universal Indicator System
//! Contains IndicatorSignature definitions for all 13 entropy indicators

use crate::catalog::{
    IndicatorSignature, IndicatorCategory, ParamConstraint, ParamType, ParamValue,
};
use super::super::bar_indicator_id::BarIndicatorId;

use once_cell::sync::Lazy;
use std::collections::HashMap;

/// Category for all indicators in this module
pub const CATEGORY: IndicatorCategory = IndicatorCategory::Entropy;

// ============================================================================
// Individual indicator signatures
// ============================================================================

/// Approximate Entropy
pub fn signature_approximate_entropy() -> IndicatorSignature {
    IndicatorSignature::builder("APEN", CATEGORY)
        .name("Approximate Entropy")
        .description("Measures regularity and predictability in price patterns")
        .add_constraint(ParamConstraint::period(10, 200, 20))
        .add_constraint(
            ParamConstraint::new("pattern_length", ParamType::USize)
                .with_min(ParamValue::USize(1))
                .with_max(ParamValue::USize(5))
                .with_default(ParamValue::USize(2))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("tolerance", ParamType::F64)
                .with_min(ParamValue::F64(0.01))
                .with_max(ParamValue::F64(1.0))
                .with_default(ParamValue::F64(0.15))
                .required()
        )
        .metadata("range", "0.0-2.0+")
        .metadata("interpretation", "0.0 = maximally regular, higher = less regular")
        .machine_id(BarIndicatorId::Apen) // TODO: Add to enum
        // Note: "APEN" is already the main ID, no need for alias
        .alias("Apen")
        .alias("apen")
        .alias("APPROXIMATEENTROPY")
        .alias("ApproximateEntropy")
        .alias("approximateentropy")
        .alias("approximate_entropy")
        .alias("APPROXIMATE_ENTROPY")
        .alias("Approximate_Entropy")
        .build()
}

/// Conditional Entropy
pub fn signature_conditional_entropy() -> IndicatorSignature {
    IndicatorSignature::builder("CONDEN", CATEGORY)
        .name("Conditional Entropy")
        .description("Measures conditional uncertainty in price movements")
        .add_constraint(ParamConstraint::period(10, 200, 20))
        .add_constraint(
            ParamConstraint::new("bins", ParamType::USize)
                .with_min(ParamValue::USize(5))
                .with_max(ParamValue::USize(50))
                .with_default(ParamValue::USize(10))
                .required()
        )
        .machine_id(BarIndicatorId::Conden) // TODO: Add to enum
        // Note: "CONDEN" is already the main ID, no need for alias
        .alias("Conden")
        .alias("conden")
        .alias("CONDITIONALENTROPY")
        .alias("ConditionalEntropy")
        .alias("conditionalentropy")
        .alias("conditional_entropy")
        .alias("CONDITIONAL_ENTROPY")
        .alias("Conditional_Entropy")
        .build()
}

/// Cross Mutual Information with Lags
pub fn signature_cross_mutual_information_lags() -> IndicatorSignature {
    IndicatorSignature::builder("XMIL", CATEGORY)
        .name("Cross Mutual Information with Lags")
        .description("Measures mutual information between price and lagged values")
        .add_constraint(ParamConstraint::period(10, 200, 20))
        .add_constraint(
            ParamConstraint::new("lag", ParamType::USize)
                .with_min(ParamValue::USize(1))
                .with_max(ParamValue::USize(20))
                .with_default(ParamValue::USize(5))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("bins", ParamType::USize)
                .with_min(ParamValue::USize(5))
                .with_max(ParamValue::USize(50))
                .with_default(ParamValue::USize(10))
                .required()
        )
        .machine_id(BarIndicatorId::Xmil) // TODO: Add to enum
        // Note: "XMIL" is already the main ID, no need for alias
        .alias("Xmil")
        .alias("xmil")
        .alias("CROSSMUTUALINFORMATIONWITHLAGS")
        .alias("CrossMutualInformationwithLags")
        .alias("crossmutualinformationwithlags")
        .alias("cross_mutual_information_with_lags")
        .alias("CROSS_MUTUAL_INFORMATION_WITH_LAGS")
        .alias("Cross_Mutual_Information_With_Lags")
        .build()
}

/// Fisher Information
pub fn signature_fisher_information() -> IndicatorSignature {
    IndicatorSignature::builder("FISHER", CATEGORY)
        .name("Fisher Information")
        .description("Measures information content and parameter sensitivity")
        .add_constraint(ParamConstraint::period(10, 200, 20))
        .add_constraint(
            ParamConstraint::new("bins", ParamType::USize)
                .with_min(ParamValue::USize(5))
                .with_max(ParamValue::USize(50))
                .with_default(ParamValue::USize(10))
                .required()
        )
        .machine_id(BarIndicatorId::Fisher) // TODO: Add to enum
        // Note: "FISHER" is already the main ID, no need for alias
        .alias("Fisher")
        .alias("fisher")
        .alias("FISHERINFORMATION")
        .alias("FisherInformation")
        .alias("fisherinformation")
        .alias("fisher_information")
        .alias("FISHER_INFORMATION")
        .alias("Fisher_Information")
        .build()
}

/// Information Gain
pub fn signature_information_gain() -> IndicatorSignature {
    IndicatorSignature::builder("INFOG", CATEGORY)
        .name("Information Gain")
        .description("Measures reduction in uncertainty from price information")
        .add_constraint(ParamConstraint::period(10, 200, 20))
        .add_constraint(
            ParamConstraint::new("bins", ParamType::USize)
                .with_min(ParamValue::USize(5))
                .with_max(ParamValue::USize(50))
                .with_default(ParamValue::USize(10))
                .required()
        )
        .machine_id(BarIndicatorId::Infog) // TODO: Add to enum
        // Note: "INFOG" is already the main ID, no need for alias
        .alias("Infog")
        .alias("infog")
        .alias("INFORMATIONGAIN")
        .alias("InformationGain")
        .alias("informationgain")
        .alias("information_gain")
        .alias("INFORMATION_GAIN")
        .alias("Information_Gain")
        .build()
}

/// Jensen-Shannon Divergence
pub fn signature_js_divergence() -> IndicatorSignature {
    IndicatorSignature::builder("JSD", CATEGORY)
        .name("Jensen-Shannon Divergence")
        .description("Symmetric measure of distribution difference between periods")
        .add_constraint(ParamConstraint::period(10, 200, 20))
        .add_constraint(
            ParamConstraint::new("bins", ParamType::USize)
                .with_min(ParamValue::USize(8))
                .with_max(ParamValue::USize(50))
                .with_default(ParamValue::USize(10))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("clip_abs", ParamType::F64)
                .with_min(ParamValue::F64(0.001))
                .with_max(ParamValue::F64(1.0))
                .with_default(ParamValue::F64(0.1))
                .required()
        )
        .metadata("range", "0.0-1.0")
        .metadata("interpretation", "0.0 = identical distributions, 1.0 = maximally different")
        .machine_id(BarIndicatorId::Jsd) // TODO: Add to enum
        // Note: "JSD" is already the main ID, no need for alias
        .alias("Jsd")
        .alias("jsd")
        .alias("JENSENSHANNONDIVERGENCE")
        .alias("JensenShannonDivergence")
        .alias("jensenshannondivergence")
        .alias("jensen_shannon_divergence")
        .alias("JENSEN_SHANNON_DIVERGENCE")
        .alias("Jensen_Shannon_Divergence")
        .build()
}

/// Kullback-Leibler Divergence
pub fn signature_kl_divergence() -> IndicatorSignature {
    IndicatorSignature::builder("KLD", CATEGORY)
        .name("Kullback-Leibler Divergence")
        .description("Measures distribution difference between adjacent windows")
        .add_constraint(ParamConstraint::period(10, 200, 20))
        .add_constraint(
            ParamConstraint::new("bins", ParamType::USize)
                .with_min(ParamValue::USize(8))
                .with_max(ParamValue::USize(50))
                .with_default(ParamValue::USize(10))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("clip_abs", ParamType::F64)
                .with_min(ParamValue::F64(0.001))
                .with_max(ParamValue::F64(1.0))
                .with_default(ParamValue::F64(0.1))
                .required()
        )
        .metadata("range", "0.0+")
        .metadata("interpretation", "0.0 = identical distributions, higher = more different")
        .machine_id(BarIndicatorId::Kld) // TODO: Add to enum
        // Note: "KLD" is already the main ID, no need for alias
        .alias("Kld")
        .alias("kld")
        .alias("KULLBACKLEIBLERDIVERGENCE")
        .alias("KullbackLeiblerDivergence")
        .alias("kullbackleiblerdivergence")
        .alias("kullback_leibler_divergence")
        .alias("KULLBACK_LEIBLER_DIVERGENCE")
        .alias("Kullback_Leibler_Divergence")
        .build()
}

/// Mutual Information
pub fn signature_mutual_information() -> IndicatorSignature {
    IndicatorSignature::builder("MI", CATEGORY)
        .name("Mutual Information")
        .description("Measures mutual dependence between current and lagged returns")
        .add_constraint(ParamConstraint::period(10, 200, 20))
        .add_constraint(
            ParamConstraint::new("lag", ParamType::USize)
                .with_min(ParamValue::USize(1))
                .with_max(ParamValue::USize(20))
                .with_default(ParamValue::USize(1))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("bins", ParamType::USize)
                .with_min(ParamValue::USize(5))
                .with_max(ParamValue::USize(50))
                .with_default(ParamValue::USize(10))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("clip_abs", ParamType::F64)
                .with_min(ParamValue::F64(0.001))
                .with_max(ParamValue::F64(1.0))
                .with_default(ParamValue::F64(0.1))
                .required()
        )
        .metadata("range", "0.0+")
        .metadata("complexity", "O(1) with rolling histogram")
        .machine_id(BarIndicatorId::Mi) // TODO: Add to enum
        // Note: "MI" is already the main ID, no need for alias
        .alias("Mi")
        .alias("mi")
        .alias("MUTUALINFORMATION")
        .alias("MutualInformation")
        .alias("mutualinformation")
        .alias("mutual_information")
        .alias("MUTUAL_INFORMATION")
        .alias("Mutual_Information")
        .build()
}

/// Permutation Entropy
pub fn signature_permutation_entropy() -> IndicatorSignature {
    IndicatorSignature::builder("PE", CATEGORY)
        .name("Permutation Entropy")
        .description("Measures diversity of ordinal patterns in time series")
        .add_constraint(ParamConstraint::period(10, 200, 20))
        .add_constraint(
            ParamConstraint::new("order", ParamType::USize)
                .with_min(ParamValue::USize(3))
                .with_max(ParamValue::USize(7))
                .with_default(ParamValue::USize(3))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("delay", ParamType::USize)
                .with_min(ParamValue::USize(1))
                .with_max(ParamValue::USize(5))
                .with_default(ParamValue::USize(1))
                .required()
        )
        .metadata("range", "0.0-1.0 (normalized)")
        .metadata("interpretation", "0.0 = one pattern, 1.0 = all patterns equally probable")
        .machine_id(BarIndicatorId::Pe) // TODO: Add to enum
        // Note: "PE" is already the main ID, no need for alias
        .alias("Pe")
        .alias("pe")
        .alias("PERMUTATIONENTROPY")
        .alias("PermutationEntropy")
        .alias("permutationentropy")
        .alias("permutation_entropy")
        .alias("PERMUTATION_ENTROPY")
        .alias("Permutation_Entropy")
        .build()
}

/// Sample Entropy
pub fn signature_sample_entropy() -> IndicatorSignature {
    IndicatorSignature::builder("SAMPEN", CATEGORY)
        .name("Sample Entropy")
        .description("Improved ApEn without self-matches, measures time series complexity")
        .add_constraint(ParamConstraint::period(10, 200, 20))
        .add_constraint(
            ParamConstraint::new("pattern_length", ParamType::USize)
                .with_min(ParamValue::USize(1))
                .with_max(ParamValue::USize(5))
                .with_default(ParamValue::USize(2))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("tolerance", ParamType::F64)
                .with_min(ParamValue::F64(0.01))
                .with_max(ParamValue::F64(1.0))
                .with_default(ParamValue::F64(0.15))
                .required()
        )
        .metadata("range", "0.0-3.0+")
        .metadata("interpretation", "0.0 = maximally predictable, higher = more complex")
        .machine_id(BarIndicatorId::Sampen) // TODO: Add to enum
        // Note: "SAMPEN" is already the main ID, no need for alias
        .alias("Sampen")
        .alias("sampen")
        .alias("SAMPLEENTROPY")
        .alias("SampleEntropy")
        .alias("sampleentropy")
        .alias("sample_entropy")
        .alias("SAMPLE_ENTROPY")
        .alias("Sample_Entropy")
        .build()
}

/// Shannon Entropy
pub fn signature_shannon_entropy() -> IndicatorSignature {
    IndicatorSignature::builder("SHANNON", CATEGORY)
        .name("Shannon Entropy")
        .description("Measures market unpredictability and randomness")
        .add_constraint(ParamConstraint::period(10, 200, 20))
        .add_constraint(
            ParamConstraint::new("bins", ParamType::USize)
                .with_min(ParamValue::USize(5))
                .with_max(ParamValue::USize(50))
                .with_default(ParamValue::USize(20))
                .required()
        )
        .metadata("range", "0.0-1.0 (normalized)")
        .metadata("interpretation", "0.0 = fully predictable, 1.0 = maximally random")
        .machine_id(BarIndicatorId::Shannon) // TODO: Add to enum
        // Note: "SHANNON" is already the main ID, no need for alias
        .alias("Shannon")
        .alias("shannon")
        .alias("SHANNONENTROPY")
        .alias("ShannonEntropy")
        .alias("shannonentropy")
        .alias("shannon_entropy")
        .alias("SHANNON_ENTROPY")
        .alias("Shannon_Entropy")
        .build()
}

/// Transfer Entropy
pub fn signature_transfer_entropy() -> IndicatorSignature {
    IndicatorSignature::builder("TE", CATEGORY)
        .name("Transfer Entropy")
        .description("Measures directional information transfer between time series")
        .add_constraint(ParamConstraint::period(10, 200, 20))
        .add_constraint(
            ParamConstraint::new("lag", ParamType::USize)
                .with_min(ParamValue::USize(1))
                .with_max(ParamValue::USize(20))
                .with_default(ParamValue::USize(1))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("bins", ParamType::USize)
                .with_min(ParamValue::USize(5))
                .with_max(ParamValue::USize(50))
                .with_default(ParamValue::USize(10))
                .required()
        )
        .add_constraint(
            ParamConstraint::new("clip_abs", ParamType::F64)
                .with_min(ParamValue::F64(0.001))
                .with_max(ParamValue::F64(1.0))
                .with_default(ParamValue::F64(0.1))
                .required()
        )
        .metadata("range", "0.0+")
        .metadata("interpretation", "Higher values indicate stronger directional causality")
        .machine_id(BarIndicatorId::Te) // TODO: Add to enum
        // Note: "TE" is already the main ID, no need for alias
        .alias("Te")
        .alias("te")
        .alias("TRANSFERENTROPY")
        .alias("TransferEntropy")
        .alias("transferentropy")
        .alias("transfer_entropy")
        .alias("TRANSFER_ENTROPY")
        .alias("Transfer_Entropy")
        .build()
}

// ============================================================================
// Catalog HashMap
// ============================================================================

/// Static catalog of all entropy indicators
/// Base catalog with main IDs only (used for initialization)
const BASE_CATALOG: &[(&str, fn() -> IndicatorSignature)] = &[
    ("APEN", signature_approximate_entropy as fn() -> IndicatorSignature),
    ("CONDEN", signature_conditional_entropy as fn() -> IndicatorSignature),
    ("XMIL", signature_cross_mutual_information_lags as fn() -> IndicatorSignature),
    ("FISHER", signature_fisher_information as fn() -> IndicatorSignature),
    ("INFOG", signature_information_gain as fn() -> IndicatorSignature),
    ("JSD", signature_js_divergence as fn() -> IndicatorSignature),
    ("KLD", signature_kl_divergence as fn() -> IndicatorSignature),
    ("MI", signature_mutual_information as fn() -> IndicatorSignature),
    ("PE", signature_permutation_entropy as fn() -> IndicatorSignature),
    ("SAMPEN", signature_sample_entropy as fn() -> IndicatorSignature),
    ("SHANNON", signature_shannon_entropy as fn() -> IndicatorSignature),
    ("TE", signature_transfer_entropy as fn() -> IndicatorSignature),
];

/// Expanded catalog with all aliases auto-generated from signatures
/// This allows O(1) lookup by any alias without manual maintenance
pub static ENTROPY_CATALOG: Lazy<HashMap<String, fn() -> IndicatorSignature>> = Lazy::new(|| {
    let mut m = HashMap::new();

    for &(main_id, func) in BASE_CATALOG {
        // Call function once to get signature with aliases
        let sig = func();

        // Insert main ID
        m.insert(main_id.to_string(), func);

        // Auto-insert all aliases from signature
        for alias in &sig.aliases {
            m.insert(alias.clone(), func);
        }
    }

    m
});

// ============================================================================
// Public API
// ============================================================================

/// Get indicator signature by ID
pub fn get_signature(id: &str) -> Option<IndicatorSignature> {
    ENTROPY_CATALOG.get(id).map(|f| f())
}

/// Get all indicator IDs in this category
pub fn all_indicator_ids() -> Vec<&'static str> {
    BASE_CATALOG.iter().map(|(id, _)| *id).collect()
}

/// Get count of indicators
pub fn count() -> usize {
    BASE_CATALOG.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_shannon_signature() {
        let sig = get_signature("SHANNON").unwrap();
        assert_eq!(sig.id, "SHANNON");
        assert_eq!(sig.category, CATEGORY);
    }

    #[test]
    fn test_all_signatures_valid() {
        for id in all_indicator_ids() {
            let sig = get_signature(id).unwrap();
            assert_eq!(sig.id, id);
            assert_eq!(sig.category, CATEGORY);
        }
    }

    #[test]
    fn test_count() {
        assert_eq!(count(), 12);
    }
}
