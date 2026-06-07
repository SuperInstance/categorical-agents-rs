# Integration Guide: categorical-agents

## What This Crate Provides

- **`Morphism<A, B>`** — A labeled morphism (function) from type A to type B in a category
- **`Adjunction<L, R>`** — Adjunction between categories: left adjoint F, right adjoint G, unit η, counit ε, with hom-set isomorphism `Hom(F(A), B) ≅ Hom(A, G(B))`
- **`ListMonad<A>`** — Non-determinism/list monad with `return_`, `bind`, `fmap`, `mzero`, `mplus`
- **`StateMonad<S, A>`** — Stateful computation monad: `get`, `put`, `modify`, `run`
- **`StreamComonad<A>`** — Infinite stream comonad with `extract`, `duplicate`, `extend` for context-dependent agents
- **`EnvComonad<E, A>`** — Environment comonad: read-only context with `ask`, `local`

This crate provides category-theoretic abstractions for composing agents: adjunctions for free/forgetful relationships between agent types, monads for chaining agent computations with effects, and comonads for context-dependent agent behavior.

## How to Add This Crate

```bash
cargo add categorical-agents
```

```rust
use categorical_agents::monad::ListMonad;

// Non-deterministic agent: try multiple strategies
let strategies = ListMonad(vec![1, 2, 3]);
let results = strategies.bind(|x| ListMonad(vec![x * 10, x * 100]));
println!("{:?}", results.run()); // [10, 100, 20, 200, 30, 300]
```

## Integration Points

### wasserstein-agents

- **Why**: wasserstein-agents moves agent distributions via optimal transport; categorical-agents provides the composition algebra for combining transport plans. The monadic `bind` chains sequential transport steps; the comonadic `extend` applies a transport plan to all future contexts.
- **How**: Wrap `AgentDistribution` in a `StateMonad` to track distribution state through transport steps, or use `ListMonad` to explore multiple transport plans non-deterministically.

```rust
use categorical_agents::monad::{StateMonad, ListMonad};
// Agent distribution is the state, transport plan is the computation
// StateMonad<AgentDistribution, TransportResult>

let transport_plan = StateMonad::new(|dist: Vec<f64>| {
    let result = vec![dist[0] * 0.5, dist[1] * 1.5];
    (result.clone(), result)
});
let (final_dist, result) = transport_plan.run(vec![2.0, 4.0]);
println!("Final distribution: {:?}", final_dist);
```

### agent-manifest

- **Why**: agent-manifest declares agent capabilities; categorical-agents provides the composition language for combining those capabilities. Each capability is a morphism; composing capabilities is categorical composition.
- **How**: Represent each declared capability as a `Morphism<A, B>`, then compose them using adjunctions or monadic bind to build complex agent behaviors from declared primitives.

```rust
use categorical_agents::adjunction::{Morphism, Adjunction};

// Each agent capability is a morphism
let parse = Morphism::new("parse", |s: String| s.len() as f64);
let threshold = Morphism::new("threshold", |x: f64| x > 5.0);

// Compose: parse → threshold
// Adjunction provides the algebraic structure for capability composition
println!("Capability: {} → {}", parse.label, threshold.label);
```

## For AI Agents

- **Context needed**: Input/output types for each agent operation, desired composition pattern (sequential/branching/contextual)
- **Key imports**: `categorical_agents::adjunction::{Morphism, Adjunction}`, `categorical_agents::monad::{ListMonad, StateMonad}`, `categorical_agents::comonad::{StreamComonad, EnvComonad}`
- **Integration pattern**: Define morphisms for primitive operations → compose via monadic `bind` (sequential) or `mplus` (parallel) → extract results with `.run()`
- **Error handling**: No explicit error type — use `ListMonad::mzero()` to represent failure (empty list of results) and `mplus` to combine alternatives

## For Humans

- **Prerequisites**: Basic category theory (functors, natural transformations), understanding of monads (from Haskell or functional programming)
- **Learning path**: Start with `monad.rs` (most familiar pattern), then `comonad.rs` (dual concept), then `adjunction.rs` (connects them both)
- **Common pitfalls**:
  - `ListMonad::bind()` explores ALL branches — exponential blowup is possible with deeply nested binds
  - `StreamComonad::duplicate()` creates nested structures that grow with tail length
  - `Adjunction` requires the triangle identities to hold — verify your unit/counit satisfy them
  - `StateMonad` is lazy — nothing executes until you call `.run()`
