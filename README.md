# categorical-agents

> Category-theoretic abstractions for composing agents — adjunctions, monads, and comonads.

## What This Does

This crate provides the algebraic vocabulary for agent composition. It implements adjunctions (free/forgetful functors with unit and counit), monads (List, Maybe, and State monads with bind, return, and do-notation simulation), and comonads (Stream, Environment, Store, and Identity comonads with extract, extend, and duplicate). These are not academic curiosities — they are design patterns for building agent pipelines where context flows in predictable, refactorable, and testable ways.

## Why It Matters

AGI will not be a single model. It will be a constellation of models, tools, and agents that must compose without cascading failures. Category theory is the mathematics of composition. When your agent coordination layer speaks the language of functors, natural transformations, and adjunctions, you gain structural guarantees that no ad-hoc message bus can provide. This crate makes those guarantees executable.

## Quick Start

```bash
cargo add categorical-agents
```

```rust
use categorical_agents::monad::{ListMonad, DoNotation};
use categorical_agents::comonad::{EnvComonad, StreamComonad};
use categorical_agents::adjunction::{FreeForgetful, CurryingAdjunction};

fn main() {
    // List monad: non-deterministic agent outcomes
    let outcomes = ListMonad(vec![1, 2, 3])
        .bind(|x| ListMonad(vec![x * 2, x * 10]));
    println!("{:?}", outcomes.run()); // [2, 10, 4, 20, 6, 30]

    // Do-notation: cartesian product
    let pairs = DoNotation::do2(
        ListMonad(vec![1, 2]),
        |x| ListMonad(vec![*x, x * 10]),
        |x, y| (*x, *y),
    );
    println!("{:?}", pairs.run());

    // Environment comonad: agent with read-only config context
    let agent = EnvComonad::new("production", 42);
    let result = agent.extend(|e| e.value + e.env.len());
    assert_eq!(result.extract(), 52);
}
```

## Architecture

| Module | Purpose |
|--------|---------|
| `adjunction` | Functors, natural transformations, triangle identities, free/forgetful and currying examples |
| `monad` | List, Maybe, and State monads with bind, fmap, guard, mplus, and do-notation |
| `comonad` | Stream, Environment, Store, and Identity comonads with extract, extend, duplicate, and local context |

## API Tour

### `Adjunction<L, R>`

A pair of functors with unit η and counit ε satisfying the triangle identities.

```rust
pub struct Adjunction<L, R> {
    pub left: fn(L) -> R,
    pub right: fn(R) -> L,
    pub unit: fn(L) -> L,
    pub counit: fn(R) -> R,
}

impl<L: Clone, R: Clone> Adjunction<L, R> {
    pub fn new(left, right, unit, counit) -> Self;
    pub fn fmap(&self, a: L) -> R;
    pub fn gmap(&self, b: R) -> L;
}
```

### `ListMonad<A>`

The non-determinism monad. `return` wraps a value; `bind` maps and flattens.

```rust
impl<A: Clone> ListMonad<A> {
    pub fn return_(a: A) -> Self;
    pub fn bind<B: Clone, F>(self, f: F) -> ListMonad<B>;
    pub fn fmap<B, F>(self, f: F) -> ListMonad<B>;
    pub fn guard(self, pred: impl Fn(&A) -> bool) -> Self;
    pub fn mplus(self, other: ListMonad<A>) -> ListMonad<A>;
    pub fn run(self) -> Vec<A>;
}
```

### `StateMonad<S, A>`

Pure functional state thread. Uses `Rc<dyn Fn>` for cheap cloning.

```rust
impl<S: Clone + 'static, A: Clone + 'static> StateMonad<S, A> {
    pub fn return_(a: A) -> Self;
    pub fn bind<B: Clone + 'static, F>(self, f: F) -> StateMonad<S, B>;
    pub fn eval(&self, s: S) -> (A, S);
    pub fn get() -> StateMonad<S, S>;
    pub fn put(s: S) -> StateMonad<S, ()>;
    pub fn modify(f: impl Fn(S) -> S + 'static) -> StateMonad<S, ()>;
}
```

### `EnvComonad<E, A>`

A value in a read-only environment — the comonadic dual of Reader.

```rust
impl<E: Clone, A: Clone> EnvComonad<E, A> {
    pub fn new(env: E, value: A) -> Self;
    pub fn extract(&self) -> A;
    pub fn extend<B: Clone, F>(&self, f: F) -> EnvComonad<E, B>;
    pub fn local<F2>(&self, f: F2) -> EnvComonad<E, A>;
}
```

### `StoreComonad<S, A>`

A position-indexed value — the comonadic dual of State. Navigate with `seek` and `peek_at`.

```rust
impl<S: Clone + 'static, A: Clone + 'static> StoreComonad<S, A> {
    pub fn new(pos: S, peek: impl Fn(S) -> A + 'static) -> Self;
    pub fn extract(&self) -> A;
    pub fn seek(&self, new_pos: S) -> StoreComonad<S, A>;
    pub fn peek_at(&self, s: S) -> A;
}
```

### `CurryingAdjunction`

The classic adjunction between product and exponential: `Hom(A × B, C) ≅ Hom(A, C^B)`.

```rust
impl CurryingAdjunction {
    pub fn curry<A, B, C>(f: impl Fn(A, B) -> C + Clone + 'static)
        -> Box<dyn Fn(A) -> Box<dyn Fn(B) -> C>>;
    pub fn uncurry<A, B, C>(f: Box<dyn Fn(A) -> Box<dyn Fn(B) -> C>>)
        -> Box<dyn Fn(A, B) -> C>;
}
```

## Performance

| Operation | Complexity | Notes |
|-----------|-----------|-------|
| Monad bind (List) | O(n × m) | n = input length, m = average output length per element |
| Monad bind (State) | O(1) closure creation | Actual work deferred until `eval` |
| Comonad extend (Stream) | O(k) | k = tail length |
| Comonad extend (Store) | O(1) | Closes over function pointer |
| Adjunction compose | O(1) | Structural composition of function pointers |

StateMonad uses `Rc<dyn Fn>` to make cloning cheap — each bind creates a new closure wrapper, not a deep copy.

## Ecosystem

- **[conservation-law](https://github.com/SuperInstance/conservation-law-rs)** — Model agent trajectories as State monad computations over phase space
- **[spectral-fleet](https://github.com/SuperInstance/spectral-fleet-rs)** — Chain spectral clustering stages via monadic bind for reusable ML pipelines
- **[wasserstein-agents](https://github.com/SuperInstance/wasserstein-agents-rs)** — Lift optimal transport into monadic context for probabilistic agent planning
- **[t-minus](https://github.com/SuperInstance/t-minus-rs)** — Compose scheduling and deadline logic with comonadic environment contexts

## Ideas for Improvement

1. **Async monads** — Implement `Future`-aware monad instances for composable async agent pipelines.
2. **Lens integration** — Add lawful optics (Lens, Prism, Traversal) on top of the Store comonad for deeply nested agent state.
3. **Free monad** — A `FreeMonad<F, A>` interpreter pattern for defining agent DSLs with custom effect handlers.
4. **Comonad transformers** — Stack Stream, Env, and Store comonads for multi-layered agent contexts.
5. **Property-based tests** — Verify monad laws (left identity, right identity, associativity) and comonad laws with `proptest`.

## License

MIT OR Apache-2.0
