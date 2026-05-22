//! EventSignature — complete specification for an event/detector primitive.
//!
//! Mirrors `IndicatorSignature` but typed for the events subsystem.

use std::collections::HashMap;
use crate::catalog::{ConstraintSet, ParamConstraint};
use crate::bar_indicators::indicator_value::IndicatorValueKind;
use crate::catalog::indicator_signature::{IndicatorRoleKind, SourceType};
use crate::data_loader::stream_kind::StreamKind;
use crate::events::event_id::EventId;

/// Complete specification of an event/detector primitive.
///
/// Used by the universal event factory, codegen, and the catalog UI.
#[derive(Debug, Clone)]
pub struct EventSignature {
    /// Unique identifier (snake_case, e.g. "bos_event_detector").
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Short description.
    pub description: String,
    /// Typed factory identifier.
    pub machine_id: Option<EventId>,
    /// Parameter constraints.
    pub constraints: ConstraintSet,
    /// Primary input stream (almost always `Bar`).
    pub input_stream: StreamKind,
    /// Auxiliary input streams (for hybrid events).
    pub aux_streams: &'static [StreamKind],
    /// Semantic role for codegen slot validation.
    pub role_kind: Option<IndicatorRoleKind>,
    /// Output shape discriminant for codegen.
    pub output_kind: Option<IndicatorValueKind>,
    /// Data source type.
    pub source_type: SourceType,
    /// Aliases for user-facing lookup.
    pub aliases: Vec<String>,
    /// Free-form metadata tags.
    pub metadata: HashMap<String, String>,
}

impl EventSignature {
    /// Create a builder.
    pub fn builder(id: impl Into<String>) -> EventSignatureBuilder {
        EventSignatureBuilder::new(id)
    }

    /// Get metadata value.
    pub fn get_metadata(&self, key: &str) -> Option<&str> {
        self.metadata.get(key).map(|s| s.as_str())
    }
}

// ── Builder ────────────────────────────────────────────────────────────────────

/// Fluent builder for `EventSignature`.
pub struct EventSignatureBuilder {
    id: String,
    name: Option<String>,
    description: Option<String>,
    machine_id: Option<EventId>,
    constraints: Vec<ParamConstraint>,
    input_stream: StreamKind,
    aux_streams: &'static [StreamKind],
    role_kind: Option<IndicatorRoleKind>,
    output_kind: Option<IndicatorValueKind>,
    source_type: SourceType,
    aliases: Vec<String>,
    metadata: HashMap<String, String>,
}

impl EventSignatureBuilder {
    /// Create a new builder.
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: None,
            description: None,
            machine_id: None,
            constraints: Vec::new(),
            input_stream: StreamKind::Bar,
            aux_streams: &[],
            role_kind: None,
            output_kind: None,
            source_type: SourceType::PriceAndVolume,
            aliases: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn machine_id(mut self, id: EventId) -> Self {
        self.machine_id = Some(id);
        self
    }

    pub fn add_constraint(mut self, c: ParamConstraint) -> Self {
        self.constraints.push(c);
        self
    }

    pub fn input_stream(mut self, stream: StreamKind) -> Self {
        self.input_stream = stream;
        self
    }

    pub fn aux_streams(mut self, streams: &'static [StreamKind]) -> Self {
        self.aux_streams = streams;
        self
    }

    pub fn role_kind(mut self, kind: IndicatorRoleKind) -> Self {
        self.role_kind = Some(kind);
        self
    }

    pub fn output_kind(mut self, kind: IndicatorValueKind) -> Self {
        self.output_kind = Some(kind);
        self
    }

    pub fn source_type(mut self, st: SourceType) -> Self {
        self.source_type = st;
        self
    }

    pub fn alias(mut self, alias: impl Into<String>) -> Self {
        self.aliases.push(alias.into());
        self
    }

    pub fn metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    pub fn build(self) -> EventSignature {
        let name = self.name.unwrap_or_else(|| self.id.clone());
        let mut constraints = ConstraintSet::new(&self.id);
        constraints.add_all(self.constraints);
        EventSignature {
            id: self.id,
            name,
            description: self.description.unwrap_or_default(),
            machine_id: self.machine_id,
            constraints,
            input_stream: self.input_stream,
            aux_streams: self.aux_streams,
            role_kind: self.role_kind,
            output_kind: self.output_kind,
            source_type: self.source_type,
            aliases: self.aliases,
            metadata: self.metadata,
        }
    }
}
