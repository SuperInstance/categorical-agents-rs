//! Monads for agent composition.
//!
//! A monad is an endofunctor T equipped with:
//! - `return` (η): A → T(A)
//! - `bind` (>>=): T(A) → (A → T(B)) → T(B)
//! satisfying left identity, right identity, and associativity.
//!
//! Also includes do-notation simulation and a monad transformer pattern.

use std::rc::Rc;

/// A monadic value wrapping a vector of results (non-determinism monad / list monad).
#[derive(Clone, Debug, PartialEq)]
pub struct ListMonad<A>(pub Vec<A>);

impl<A: Clone> ListMonad<A> {
    /// return (η): wrap a single value.
    pub fn return_(a: A) -> Self {
        ListMonad(vec![a])
    }

    /// bind (>>=): apply a function producing a monad to each element, collect results.
    pub fn bind<B: Clone, F>(self, f: F) -> ListMonad<B>
    where
        F: Fn(A) -> ListMonad<B>,
    {
        let mut results = Vec::new();
        for a in self.0 {
            let ListMonad(bs) = f(a);
            results.extend(bs);
        }
        ListMonad(results)
    }

    /// fmap: map a pure function over the monad.
    pub fn fmap<B, F>(self, f: F) -> ListMonad<B>
    where
        F: Fn(A) -> B,
    {
        ListMonad(self.0.into_iter().map(f).collect())
    }

    /// Extract inner values.
    pub fn run(self) -> Vec<A> {
        self.0
    }

    /// mzero: the empty monadic value (identity for mplus).
    pub fn mzero() -> Self {
        ListMonad(vec![])
    }

    /// mplus: combine two monadic values (union).
    pub fn mplus(self, other: ListMonad<A>) -> ListMonad<A> {
        let mut v = self.0;
        v.extend(other.0);
        ListMonad(v)
    }

    /// Guard: filter by predicate (returns mzero if false).
    pub fn guard(self, pred: impl Fn(&A) -> bool) -> Self {
        ListMonad(self.0.into_iter().filter(pred).collect())
    }
}

/// Do-notation simulation via chained binds.
/// Uses direct iteration instead of bind to avoid closure borrow issues.
pub struct DoNotation;

impl DoNotation {
    /// Simulate do-notation with two bindings:
    /// do { x <- ma; y <- mb(x); return f(x,y) }
    pub fn do2<A, B, C, F, G>(
        ma: ListMonad<A>,
        mb: F,
        result: G,
    ) -> ListMonad<C>
    where
        A: Clone,
        B: Clone,
        C: Clone,
        F: Fn(&A) -> ListMonad<B>,
        G: Fn(&A, &B) -> C,
    {
        let mut out = Vec::new();
        for a in ma.0 {
            for b in mb(&a).0 {
                out.push(result(&a, &b));
            }
        }
        ListMonad(out)
    }

    /// Three-binding do-notation.
    pub fn do3<A, B, C, D, F, G, H>(
        ma: ListMonad<A>,
        mb: F,
        mc: G,
        result: H,
    ) -> ListMonad<D>
    where
        A: Clone,
        B: Clone,
        C: Clone,
        D: Clone,
        F: Fn(&A) -> ListMonad<B>,
        G: Fn(&A, &B) -> ListMonad<C>,
        H: Fn(&A, &B, &C) -> D,
    {
        let mut out = Vec::new();
        for a in ma.0 {
            for b in mb(&a).0 {
                for c in mc(&a, &b).0 {
                    out.push(result(&a, &b, &c));
                }
            }
        }
        ListMonad(out)
    }
}

/// Maybe monad for optional results.
#[derive(Clone, Debug, PartialEq)]
pub enum MaybeMonad<A> {
    Just(A),
    Nothing,
}

impl<A: Clone> MaybeMonad<A> {
    pub fn return_(a: A) -> Self {
        MaybeMonad::Just(a)
    }

    pub fn bind<B, F>(self, f: F) -> MaybeMonad<B>
    where
        F: Fn(A) -> MaybeMonad<B>,
    {
        match self {
            MaybeMonad::Just(a) => f(a),
            MaybeMonad::Nothing => MaybeMonad::Nothing,
        }
    }

    pub fn fmap<B, F>(self, f: F) -> MaybeMonad<B>
    where
        F: Fn(A) -> B,
    {
        match self {
            MaybeMonad::Just(a) => MaybeMonad::Just(f(a)),
            MaybeMonad::Nothing => MaybeMonad::Nothing,
        }
    }
}

/// State monad: a pure functional state transformation S -> (A, S).
/// Uses Rc<dyn Fn> for cheap cloning (like StoreComonad).
pub struct StateMonad<S, A> {
    run_state: Rc<dyn Fn(S) -> (A, S)>,
}

impl<S: Clone, A: Clone> Clone for StateMonad<S, A> {
    fn clone(&self) -> Self {
        StateMonad { run_state: self.run_state.clone() }
    }
}

impl<S: Clone + 'static, A: Clone + 'static> StateMonad<S, A> {
    /// return for State.
    pub fn return_(a: A) -> Self {
        StateMonad {
            run_state: Rc::new(move |s| (a.clone(), s)),
        }
    }

    /// bind for State.
    pub fn bind<B: Clone + 'static, F>(self, f: F) -> StateMonad<S, B>
    where
        F: Fn(A) -> StateMonad<S, B> + 'static,
    {
        let run = self.run_state;
        StateMonad {
            run_state: Rc::new(move |s| {
                let (a, s1) = run(s);
                f(a).eval(s1)
            }),
        }
    }

    /// Run the stateful computation.
    pub fn eval(&self, s: S) -> (A, S) {
        (self.run_state)(s)
    }

    /// Get the current state.
    pub fn get() -> StateMonad<S, S> {
        StateMonad {
            run_state: Rc::new(|s| (s.clone(), s)),
        }
    }

    /// Put a new state.
    pub fn put(s: S) -> StateMonad<S, ()> {
        StateMonad {
            run_state: Rc::new(move |_| ((), s.clone())),
        }
    }

    /// Modify the state with a function.
    pub fn modify(f: impl Fn(S) -> S + 'static) -> StateMonad<S, ()> {
        StateMonad {
            run_state: Rc::new(move |s| ((), f(s))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_monad_left_identity() {
        let a = 5;
        let f = |x: i32| ListMonad(vec![x, x * 10]);
        let lhs = ListMonad::return_(a).bind(&f);
        let rhs = f(a);
        assert_eq!(lhs, rhs);
    }

    #[test]
    fn test_monad_right_identity() {
        let m = ListMonad(vec![1, 2, 3]);
        let lhs = m.clone().bind(ListMonad::return_);
        assert_eq!(lhs, m);
    }

    #[test]
    fn test_monad_associativity() {
        let m = ListMonad(vec![1, 2]);
        let f = |x: i32| ListMonad(vec![x, x + 1]);
        let g = |x: i32| ListMonad(vec![x * 2]);

        let lhs = m.clone().bind(&f).bind(&g);
        let rhs = m.bind(move |x| f(x).bind(&g));
        assert_eq!(lhs, rhs);
    }

    #[test]
    fn test_fmap() {
        let m = ListMonad(vec![1, 2, 3]);
        let result = m.fmap(|x| x * 2);
        assert_eq!(result, ListMonad(vec![2, 4, 6]));
    }

    #[test]
    fn test_guard() {
        let m = ListMonad(vec![1, 2, 3, 4, 5]);
        let filtered = m.guard(|x| x % 2 == 0);
        assert_eq!(filtered, ListMonad(vec![2, 4]));
    }

    #[test]
    fn test_mzero_mplus() {
        let m1 = ListMonad(vec![1, 2]);
        let m2 = ListMonad(vec![3, 4]);
        let combined = m1.mplus(m2);
        assert_eq!(combined, ListMonad(vec![1, 2, 3, 4]));

        let empty: ListMonad<i32> = ListMonad::mzero();
        let with = empty.mplus(ListMonad(vec![5]));
        assert_eq!(with, ListMonad(vec![5]));
    }

    #[test]
    fn test_do_notation_2() {
        let ma = ListMonad(vec![1, 2]);
        let result = DoNotation::do2(
            ma,
            |x| ListMonad(vec![*x, x * 10]),
            |x, y| *x + *y,
        );
        assert_eq!(result.run(), vec![2, 11, 4, 22]);
    }

    #[test]
    fn test_do_notation_3() {
        let ma = ListMonad(vec![1]);
        let result = DoNotation::do3(
            ma,
            |x| ListMonad(vec![*x, x + 1]),
            |x, y| ListMonad(vec![x + y]),
            |x, y, z| x * y * z,
        );
        assert_eq!(result.run(), vec![2, 6]);
    }

    #[test]
    fn test_state_monad_get() {
        let get: StateMonad<i32, i32> = StateMonad::<i32, i32>::get();
        let (val, state) = get.eval(42);
        assert_eq!(val, 42);
        assert_eq!(state, 42);
    }

    #[test]
    fn test_state_monad_put() {
        let put: StateMonad<i32, ()> = StateMonad::<i32, ()>::put(100);
        let (val, state) = put.eval(42);
        assert_eq!(val, ());
        assert_eq!(state, 100);
    }

    #[test]
    fn test_state_monad_modify() {
        let modify: StateMonad<i32, ()> = StateMonad::<i32, ()>::modify(|s| s + 10);
        let (val, state) = modify.eval(5);
        assert_eq!(val, ());
        assert_eq!(state, 15);
    }

    #[test]
    fn test_state_monad_return() {
        let sm: StateMonad<i32, String> = StateMonad::return_(String::from("hello"));
        let (val, state) = sm.eval(99);
        assert_eq!(val, "hello");
        assert_eq!(state, 99);
    }

    #[test]
    fn test_state_monad_bind() {
        let comp = StateMonad::<i32, i32>::get().bind(|x: i32| {
            StateMonad::return_(x * 2)
        });
        let (result, _s) = comp.eval(7);
        assert_eq!(result, 14);
    }

    #[test]
    fn test_maybe_monad_just() {
        let m = MaybeMonad::return_(5);
        let result = m.bind(|x| MaybeMonad::Just(x + 1));
        assert_eq!(result, MaybeMonad::Just(6));
    }

    #[test]
    fn test_maybe_monad_nothing_short_circuit() {
        let m: MaybeMonad<i32> = MaybeMonad::Nothing;
        let result = m.bind(|x| MaybeMonad::Just(x + 1));
        assert_eq!(result, MaybeMonad::Nothing);
    }

    #[test]
    fn test_maybe_fmap() {
        let m = MaybeMonad::Just(10);
        assert_eq!(m.fmap(|x| x * 3), MaybeMonad::Just(30));
        let n: MaybeMonad<i32> = MaybeMonad::Nothing;
        assert_eq!(n.fmap(|x| x * 3), MaybeMonad::Nothing);
    }
}
