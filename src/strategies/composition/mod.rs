//! Composition — как собирать стратегию из событий.
//!
//! `CompositionSpec` — рекурсивное дерево над `Event`. `Guard` — фильтры поверх
//! композиции. `validation` — статические проверки `Event` и `CompositionSpec`.

pub mod spec;
pub mod guard;
pub mod validation;

pub use spec::CompositionSpec;
pub use guard::{Guard, CmpOp};
