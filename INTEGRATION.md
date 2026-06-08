# INTEGRATION.md — categorical-agents-rs × wasserstein-agents-rs × conservation-law-rs

**Categorical agents** provide monads, comonads, and adjunctions for
composing agent computations. They connect to optimal transport for
monadic distribution composition and to conservation-law for adjunctions
between state spaces.

## Synergy Map

```
wasserstein-agents-rs          categorical-agents-rs          conservation-law-rs
┌──────────────────┐           ┌──────────────────────┐       ┌─────────────────────┐
│ AgentDistribution │◄─────────►│ StateMonad           │◄─────►│ AgentState          │
│ SinkhornSolver    │           │ MaybeMonad           │       │ total_energy        │
│ OptimalTransport  │           │ ListMonad            │       │ verify_noether      │
│ JKOScheme         │           │ Adjunction           │       │ SymplecticIntegr    │
└──────────────────┘           │ StreamComonad        │       └─────────────────────┘
                               │ StoreComonad         │
                               └──────────────────────┘
```

## Key Insight

Agent computations can be chained like monadic binds: read state,
compute action, update state. Wasserstein agents lift this to
probability distributions (transport plans as monadic values).
Conservation-law provides the adjunction between position and momentum
spaces, exactly the structure underlying Hamiltonian mechanics.

## Example 1: State Monad for Agent Decision Sequences

Chain agent decisions using the state monad from categorical-agents.

```rust
use categorical_agents::monad::StateMonad;

fn agent_decision_chain(initial_state: i32) -> (i32, i32) {
    let step1 = StateMonad::new(|s: i32| (s + 1, s + 1));
    let step2 = step1.bind(|a: i32| {
        StateMonad::new(move |s: i32| (a * 2, s + a * 2))
    });
    step2.eval(initial_state)
}
```

## Example 2: Monadic Composition of Transport Plans

Use ListMonad to represent non-deterministic agent moves, then compute
the Wasserstein barycenter of resulting distributions.

```rust
use categorical_agents::monad::ListMonad;
use wasserstein_agents::transport::OptimalTransport;

fn nondeterministic_fleet_moves(positions: &[Vec<f64>]) -> Vec<f64> {
    let moves = ListMonad::return_(vec![0.0, 1.0, -1.0]);
    let all_positions: Vec<Vec<f64>> = moves.bind(|dx: f64| {
        ListMonad::return_(positions.iter().map(|p| {
            vec![p[0] + dx]
        }).collect())
    }).run();

    let weights = vec![1.0 / all_positions.len() as f64; all_positions.len()];
    let costs: Vec<Vec<f64>> = all_positions.iter()
        .map(|_| vec![1.0; all_positions.len()])
        .collect();

    OptimalTransport::barycenter(
        &all_positions.iter().zip(weights.iter()).map(|(p, w)| (p.as_slice(), &costs)).collect::<Vec<_>>(),
        &weights,
        10,
    )
}
```

## Example 3: Adjunction Between Position and Momentum Spaces

The conservation-law crate's Lagrangian mechanics forms a natural
adjunction between configuration space and momentum space.

```rust
use categorical_agents::adjunction::Adjunction;
use conservation_law::lagrangian::{AgentState, MechanicalLagrangian};

fn position_momentum_adjunction() {
    // left: position -> momentum (via gradient of potential)
    let left = |q: f64| -q; // simplified: force = -grad V = -q
    // right: momentum -> position (via integration)
    let right = |p: f64| -p;
    // unit: position -> position
    let unit = |q: f64| q;
    // counit: momentum -> momentum
    let counit = |p: f64| p;

    let adj = Adjunction::new(left, right, unit, counit);
    let q = 2.0_f64;
    let p = adj.fmap(q);
    let q_recovered = adj.gmap(p);
    println!("Triangle identity 1 holds: {}", adj.triangle_identity_1(q));
    println!("Recovered position: {}", q_recovered);
}
```

## Cargo.toml Wiring

```toml
[dependencies]
categorical-agents = { git = "https://github.com/SuperInstance/categorical-agents-rs" }
wasserstein-agents = { git = "https://github.com/SuperInstance/wasserstein-agents-rs" }
conservation-law = { git = "https://github.com/SuperInstance/conservation-law-rs" }
```

## Design Patterns

### Pattern: Comonadic Context-Aware Sensing

Use EnvComonad to model agents that sense their environment and extend
local observations into fleet-wide context:

```rust
use categorical_agents::comonad::EnvComonad;

fn context_aware_reading(local_temp: f64, fleet_avg: f64) -> EnvComonad<f64, f64> {
    let sensor = EnvComonad::new(fleet_avg, local_temp);
    sensor.extend(|ctx| ctx.extract() - ctx.extract_env())
}
```

### Pattern: Monadic Error Recovery

Chain fleet operations with MaybeMonad to handle partial failures:

```rust
use categorical_agents::monad::MaybeMonad;

fn resilient_dispatch<A, F>(jobs: Vec<A>, f: F) -> Vec<A>
where F: Fn(&A) -> MaybeMonad<A> {
    jobs.into_iter()
        .filter_map(|j| match f(&j).run() {
            Some(v) => Some(v),
            None => { eprintln!("Job failed, skipping"); None }
        })
        .collect()
}
```
