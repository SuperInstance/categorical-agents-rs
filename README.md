# categorical-agents-rs

Category-theoretic abstractions for composing agents.

Agents are morphisms. Agent pipelines are functors. Agent context is a comonad. Agent effects are a monad. Adjunctions bridge the gap between what an agent *is* and what an agent *does*.

This is not abstraction for its own sake. Category theory gives us laws — left identity, right identity, associativity, triangle identities — that compose correctly by construction. When you chain 50 agents in a pipeline, you want proofs, not prayers.

Part of the **sunset-ecosystem**: categorical composition provides the algebraic backbone for agent orchestration. `conservation-law` uses adjunctions to enforce resource invariants, and `si-fleet-api` dispatches composed agent programs across the fleet.

## The Math

### Categories, Objects, Morphisms

A **category** $\mathcal{C}$ consists of:
- A collection of **objects** $\text{Ob}(\mathcal{C})$
- For each pair $(A, B)$, a set of **morphisms** $\text{Hom}(A, B)$
- Composition $\circ: \text{Hom}(B, C) \times \text{Hom}(A, B) \to \text{Hom}(A, C)$
- Identity morphisms $\text{id}_A \in \text{Hom}(A, A)$

satisfying associativity and identity laws.

For agents: objects are agent types, morphisms are transformations between agent states.

### Functors

A **functor** $F: \mathcal{C} \to \mathcal{D}$ maps objects to objects and morphisms to morphisms, preserving composition and identity:

$$F(g \circ f) = F(g) \circ F(f)$$
$$F(\text{id}_A) = \text{id}_{F(A)}$$

`fmap` is functor application on morphisms.

### Monads

A **monad** on a category $\mathcal{C}$ is an endofunctor $T: \mathcal{C} \to \mathcal{C}$ equipped with:
- **return** ($\eta$): $A \to T(A)$
- **bind** ($\gg=$): $T(A) \to (A \to T(B)) \to T(B)$

satisfying three laws:
1. **Left identity**: $\text{return}(a) \gg= f \equiv f(a)$
2. **Right identity**: $m \gg= \text{return} \equiv m$
3. **Associativity**: $(m \gg= f) \gg= g \equiv m \gg= (\lambda x. f(x) \gg= g)$

### Comonads

A **comonad** is the dual: $W: \mathcal{C} \to \mathcal{C}$ with:
- **extract**: $W(A) \to A$
- **duplicate**: $W(A) \to W(W(A))$
- **extend**: $W(A) \to (W(A) \to B) \to W(B)$

Comonads model agents in context — an agent that can see its neighborhood, its history, its environment.

### Adjunctions

An **adjunction** $F \dashv G$ between functors $F: \mathcal{C} \to \mathcal{D}$ and $G: \mathcal{D} \to \mathcal{C}$ provides a natural isomorphism:

$$\text{Hom}_{\mathcal{D}}(F(A), B) \cong \text{Hom}_{\mathcal{C}}(A, G(B))$$

with unit $\eta: \text{Id} \to GF$ and counit $\varepsilon: FG \to \text{Id}$ satisfying the triangle identities.

### Distributional Functor

The `DistributionalFunctor` maps agent categories to performance distributions. Drift from a category is measured via the 1-Wasserstein distance between an agent's performance vector and the category's reference distribution.

## Installation

```toml
[dependencies]
categorical-agents-rs = { git = "https://github.com/SuperInstance/categorical-agents-rs" }
```

## Usage

### List Monad — Non-Deterministic Agent Computation

```rust
use categorical_agents_rs::monad::ListMonad;

// Non-deterministic choice: agent tries multiple strategies
let strategies = ListMonad(vec![1.0_f64, 2.0, 3.0]);

// fmap: apply a pure function to each strategy
let doubled = strategies.fmap(|x| x * 2.0);
println!("Doubled strategies: {:?}", doubled.run()); // [2.0, 4.0, 6.0]

// bind: each strategy branches into variants
let expanded = doubled.bind(|x| ListMonad(vec![x, x + 0.5]));
println!("Expanded: {:?}", expanded.run()); // [2.0, 2.5, 4.0, 4.5, 6.0, 6.5]

// guard: filter by predicate
let valid = expanded.guard(|x| *x > 3.0);
println!("Valid strategies (>3.0): {:?}", valid.run());

// Monad laws are verified by construction:
// Left identity: return(a).bind(f) ≡ f(a)
let a = 5.0_f64;
let f = |x: f64| ListMonad(vec![x, x * 10.0]);
assert_eq!(ListMonad::return_(a).bind(&f), f(a));

// Right identity: m.bind(return) ≡ m
let m = ListMonad(vec![1.0, 2.0, 3.0]);
assert_eq!(m.clone().bind(ListMonad::return_), m);

// Associativity: (m.bind(f)).bind(g) ≡ m.bind(|x| f(x).bind(g))
let m = ListMonad(vec![1.0, 2.0]);
let f = |x: f64| ListMonad(vec![x, x + 1.0]);
let g = |x: f64| ListMonad(vec![x * 2.0]);
let lhs = m.clone().bind(&f).bind(&g);
let rhs = m.bind(move |x| f(x).bind(&g));
assert_eq!(lhs, rhs);
```

### Maybe Monad — Optional Results

```rust
use categorical_agents_rs::monad::MaybeMonad;

// Agent lookup that might fail
fn lookup_agent(id: u64) -> MaybeMonad<String> {
    if id == 1 {
        MaybeMonad::Just("agent-alpha".to_string())
    } else {
        MaybeMonad::Nothing
    }
}

// Chain lookups — Nothing short-circuits
let result = lookup_agent(1).bind(|name| {
    MaybeMonad::Just(format!("Found: {}", name))
});
assert_eq!(result, MaybeMonad::Just("Found: agent-alpha".to_string()));

let missing = lookup_agent(99).bind(|name| {
    MaybeMonad::Just(format!("Found: {}", name))
});
assert_eq!(missing, MaybeMonad::Nothing);

// fmap propagates Nothing
let mapped = MaybeMonad::<i32>::Nothing.fmap(|x| x * 2);
assert_eq!(mapped, MaybeMonad::Nothing);
```

### State Monad — Agent State Transitions

```rust
use categorical_agents_rs::monad::StateMonad;

// Agent state: a counter
type AgentState = i32;

// Get current state
let get: StateMonad<AgentState, AgentState> = StateMonad::get();
let (val, state) = get.eval(42);
println!("Current state: {} (was {})", val, state);

// Put new state
let put = StateMonad::<AgentState, ()>::put(100);
let (val, state) = put.eval(42);
println!("After put: val={:?}, state={}", val, state);

// Modify state
let inc = StateMonad::<AgentState, ()>::modify(|s| s + 1);
let (_, state) = inc.eval(5);
println!("After increment: {}", state); // 6

// Chain stateful operations with bind
let program = StateMonad::<AgentState, AgentState>::get().bind(|x: i32| {
    StateMonad::return_(x * 2)
});
let (result, _) = program.eval(7);
println!("Program result: {}", result); // 14
```

### Do-Notation — Chained Monadic Computation

```rust
use categorical_agents_rs::monad::{ListMonad, DoNotation};

// Do-notation with two bindings:
// do { x <- [1, 2]; y <- [x, x*10]; return x + y }
let result = DoNotation::do2(
    ListMonad(vec![1, 2]),
    |x| ListMonad(vec![*x, x * 10]),
    |x, y| *x + *y,
);
println!("do2 result: {:?}", result.run()); // [2, 11, 4, 22]

// Three bindings:
// do { x <- [1]; y <- [x, x+1]; z <- [x+y]; return x*y*z }
let result3 = DoNotation::do3(
    ListMonad(vec![1]),
    |x| ListMonad(vec![*x, x + 1]),
    |x, y| ListMonad(vec![x + y]),
    |x, y, z| x * y * z,
);
println!("do3 result: {:?}", result3.run()); // [2, 6]
```

### Stream Comonad — Agent Context Window

```rust
use categorical_agents_rs::comonad::StreamComonad;

// Agent sees a window of recent events
let stream = StreamComonad::new(
    "current_event",       // focused value
    vec!["next_1", "next_2", "next_3"], // future context
);

// extract: get the focused value
println!("Focused: {}", stream.extract()); // "current_event"

// fmap: transform all values
let upper = stream.fmap(|s| s.to_uppercase());
println!("Upper focus: {}", upper.focus);
println!("Upper tail: {:?}", upper.tail);

// extend: context-dependent computation
// Count how many events are visible
let counts = stream.extend(|st| {
    1 + st.tail.len() // current + remaining
});
println!("Visible events: {}", counts.focus); // 4
```

### Environment Comonad — Agent with Config

```rust
use categorical_agents_rs::comonad::EnvComonad;

// Agent with read-only configuration
let agent = EnvComonad::new(
    "production_config",  // environment (read-only)
    "agent_body",         // the agent itself
);

// extract: get the agent
println!("Agent: {}", agent.extract());

// extract_env: get the configuration
println!("Config: {}", agent.extract_env());

// extend: computation with access to both agent and config
let summary = agent.extend(|e| {
    format!("{} running with {}", e.value, e.env)
});
println!("Summary: {}", summary.value);

// local: modify environment for sub-computation
let dev_agent = agent.local(|_| "dev_config");
println!("Dev config: {}", dev_agent.env);
println!("Agent unchanged: {}", dev_agent.value);
```

### Store Comonad — Agent Indexed by Position

```rust
use categorical_agents_rs::comonad::StoreComonad;

// Agent that can "see" values at different positions
let data = vec![10, 20, 30, 40, 50];
let store = StoreComonad::new(2usize, move |i: usize| data[i]);

// extract: value at current position
println!("At position 2: {}", store.extract()); // 30

// peek_at: look at another position without moving
println!("At position 0: {}", store.peek_at(0)); // 10

// seek: move to a new position
let moved = store.seek(4);
println!("Moved to position 4: {}", moved.extract()); // 50

// duplicate: wrap in another store layer
let dup = store.duplicate();
println!("Duplicated extract: {}", dup.extract().extract()); // 30
```

### Adjunctions — Free/Forgetful and Currying

```rust
use categorical_agents_rs::adjunction::{Adjunction, FreeForgetful, CurryingAdjunction};

// Free/Forgetful adjunction: T ↔ Vec<T>
// Left adjoint (free): T → Vec<T>
let free_vec = FreeForgetful::free(42);
println!("Free: {:?}", free_vec); // vec![42]

// Right adjoint (forgetful): Vec<T> → T
let forgotten = FreeForgetful::forget(vec![1, 2, 3]);
println!("Forgotten: {}", forgotten); // 1

// Roundtrip: forget(free(x)) ≡ x
let original = 99;
let recovered = FreeForgetful::forget(FreeForgetful::free(original));
assert_eq!(original, recovered);

// Currying adjunction: Hom(A×B, C) ≅ Hom(A, C^B)
let add = |a: i32, b: i32| a + b;
let curried = CurryingAdjunction::curry(add);
let add_5 = curried(5);
assert_eq!(add_5(3), 8);

// Uncurry: back to two-argument form
let uncurried = CurryingAdjunction::uncurry(curried);
assert_eq!(uncurried(4, 5), 9);

// General adjunction with custom functors
let adj = Adjunction::new(
    |x: f64| x * 2.0,  // left adjoint: double
    |x: f64| x / 2.0,  // right adjoint: halve
    |x: f64| x,         // unit: identity
    |x: f64| x,         // counit: identity
);
assert!((adj.fmap(7.5) - 15.0).abs() < 1e-10);
assert!((adj.gmap(15.0) - 7.5).abs() < 1e-10);
```

### Distributional Functor — Drift Detection

```rust
use categorical_agents_rs::distributional::DistributionalFunctor;

// Define category distributions: 3 categories, 2 dimensions each
// Category 0: high performance
// Category 1: moderate
// Category 2: low
let distributions = vec![
    vec![vec![8.0, 9.0, 8.5, 9.5], vec![7.0, 8.0, 7.5, 8.5]],  // cat 0
    vec![vec![5.0, 6.0, 5.5, 6.5], vec![4.0, 5.0, 4.5, 5.5]],  // cat 1
    vec![vec![2.0, 3.0, 2.5, 3.5], vec![1.0, 2.0, 1.5, 2.5]],  // cat 2
];

let functor = DistributionalFunctor::new(distributions, 0.5);

// Assign an agent to the nearest category
let agent_perf = vec![7.5, 8.5];
let category = functor.assign_category(&agent_perf);
println!("Agent assigned to category: {}", category);

// Check if agent has drifted from its category
let report = functor.detect_drift(&agent_perf, category);
println!("Wasserstein distance: {:.3}", report.wasserstein_distance);
println!("Drifted? {}", report.is_drifted);
```

### Identity Comonad

```rust
use categorical_agents_rs::comonad::IdentityComonad;

// Simplest comonad: just wraps a value
let id = IdentityComonad::new(42);
assert_eq!(id.extract(), 42);

let mapped = id.fmap(|x| x * 10);
assert_eq!(mapped, IdentityComonad(420));

let extended = IdentityComonad::new(7).extend(|x| x.extract() + 3);
assert_eq!(extended, IdentityComonad(10));
```

## API Reference

### Monads

| Type | Description |
|------|-------------|
| `ListMonad<A>` | Non-determinism monad (list of results) |
| `MaybeMonad<A>` | Optional result (`Just` / `Nothing`) |
| `StateMonad<S, A>` | Stateful computation `S → (A, S)` |

| Method | Laws |
|--------|------|
| `return_(a)` | Left/right identity |
| `bind(f)` | Associativity |
| `fmap(f)` | Functor law: `fmap(f).fmap(g) ≡ fmap(f∘g)` |

### Comonads

| Type | Description |
|------|-------------|
| `StreamComonad<A>` | Focused position in a sequence |
| `EnvComonad<E, A>` | Value with read-only environment |
| `StoreComonad<S, A>` | Value indexed by position |
| `IdentityComonad<A>` | Trivial wrapper |

| Method | Description |
|--------|-------------|
| `extract()` | Get the focused value |
| `duplicate()` | Wrap in comonad of comonads |
| `extend(f)` | Context-dependent computation |
| `fmap(f)` | Map over values |

### Adjunctions

| Type | Description |
|------|-------------|
| `Adjunction<L, R>` | General adjunction with unit/counit |
| `FreeForgetful` | Free/Forgetful adjunction `T ↔ Vec<T>` |
| `CurryingAdjunction` | `Hom(A×B, C) ≅ Hom(A, C^B)` |

### Distributional

| Type | Description |
|------|-------------|
| `DistributionalFunctor` | Maps categories to distributions |
| `DriftReport` | Wasserstein distance + drift status |

## Why This Matters for Agent Systems

1. **Composition by construction**: Monad laws guarantee that `bind` chains compose associatively — no surprise ordering bugs.
2. **Context propagation**: Comonads give agents access to their environment, history, or neighborhood without global state.
3. **Type-level guarantees**: `MaybeMonad` forces you to handle the "nothing" case. `StateMonad` makes state transitions explicit.
4. **Adjunctions as bridges**: The free/forgetful adjunction models the gap between "what agents are" and "what we observe about them."
5. **Distributional drift**: The functor from categories to distributions detects when an agent has drifted outside its assigned class — this feeds into `conservation-law` for invariant checking.

## Integration

```rust
// categorical-agents-rs provides the algebra;
// conservation-law enforces resource invariants as monadic state;
// si-fleet-api dispatches composed programs across the fleet;
// wasserstein-agents-rs measures distributional drift between agent categories.
```

## License

MIT
