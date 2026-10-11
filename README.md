# mylittleindicators

Pull requests are not accepted. Please open an Issue.

Multi-stream indicator library. The cargo name is `mylittleindicators`.

This repository is a virtual workspace. The published library is `crates/mli`. `crates/mli-contract-macros` is the `contract_universe!` proc-macro that generates the engine. Strategies, the validator, and the barometer are not in this repository.

Market-data types come from `digdigdig3-core` (structs and enums only, no exchange connectors).

## Layout

- `crates/mli` — indicators, `ContractFactory`, and the chart scanners (`ewa`, `harmonic`, `fib`, `smc`, `orderflow`, `events`). Source lives in `crates/mli/src`.
- `crates/mli-contract-macros` — `contract_universe!`.

There is no library `src/` at the repository root.

## Compute

```rust
use mylittleindicators::engine::contract_engine::IndicatorOrder;
use mylittleindicators::engine::indicator_id::IndicatorId;

let mut ind = IndicatorOrder::from_defaults(IndicatorId::Rsi)
    .expect("rsi")
    .build_solo()
    .expect("build");
```

GPU kernels are the `gpu` feature. The default build is the CPU engine.
