//! Comonads for context-dependent agents.
//!
//! A comonad is the dual of a monad: an endofunctor W equipped with:
//! - `extract`: W(A) → A
//! - `duplicate`: W(A) → W(W(A))
//! - `extend`: W(A) → (W(A) → B) → W(B)
//!
//! Comonads model computations in context (e.g., zippers, streams, environments).

use std::rc::Rc;

/// Stream comonad: an infinite stream with a focused position.
#[derive(Clone, Debug, PartialEq)]
pub struct StreamComonad<A> {
    pub focus: A,
    pub tail: Vec<A>,
}

impl<A: Clone> StreamComonad<A> {
    /// Create from a focus and a tail (context).
    pub fn new(focus: A, tail: Vec<A>) -> Self {
        Self { focus, tail }
    }

    /// extract: get the focused value.
    pub fn extract(&self) -> A {
        self.focus.clone()
    }

    /// duplicate: wrap the stream in another stream layer.
    pub fn duplicate(&self) -> StreamComonad<StreamComonad<A>> {
        let mut streams = Vec::new();
        let mut current = self.clone();
        for _ in 0..self.tail.len().max(1) {
            streams.push(current.clone());
            if !current.tail.is_empty() {
                let f = current.tail[0].clone();
                let t = current.tail[1..].to_vec();
                current = StreamComonad::new(f, t);
            }
        }
        StreamComonad::new(self.clone(), streams)
    }

    /// extend: apply a context-dependent function.
    pub fn extend<B: Clone, F>(&self, f: F) -> StreamComonad<B>
    where
        F: Fn(&StreamComonad<A>) -> B,
    {
        let b = f(self);
        let tail: Vec<B> = {
            let mut results = Vec::new();
            let mut current = self.clone();
            for _ in 0..self.tail.len() {
                if !current.tail.is_empty() {
                    let f_val = current.tail[0].clone();
                    let t = current.tail[1..].to_vec();
                    current = StreamComonad::new(f_val, t);
                    results.push(f(&current));
                }
            }
            results
        };
        StreamComonad::new(b, tail)
    }

    /// fmap: map a pure function over the stream.
    pub fn fmap<B: Clone, F>(&self, f: F) -> StreamComonad<B>
    where
        F: Fn(A) -> B + Clone,
    {
        StreamComonad::new(
            f.clone()(self.focus.clone()),
            self.tail.iter().cloned().map(f).collect(),
        )
    }
}

/// Environment comonad: a value together with a read-only environment.
#[derive(Clone, Debug, PartialEq)]
pub struct EnvComonad<E, A> {
    pub env: E,
    pub value: A,
}

impl<E: Clone, A: Clone> EnvComonad<E, A> {
    pub fn new(env: E, value: A) -> Self {
        Self { env, value }
    }

    /// extract: get the value.
    pub fn extract(&self) -> A {
        self.value.clone()
    }

    /// extract_env: get the environment.
    pub fn extract_env(&self) -> E {
        self.env.clone()
    }

    /// duplicate: nest the environment.
    pub fn duplicate(&self) -> EnvComonad<E, EnvComonad<E, A>> {
        EnvComonad::new(self.env.clone(), self.clone())
    }

    /// extend: context-dependent computation with access to env.
    pub fn extend<B: Clone, F>(&self, f: F) -> EnvComonad<E, B>
    where
        F: Fn(&EnvComonad<E, A>) -> B,
    {
        EnvComonad::new(self.env.clone(), f(self))
    }

    /// fmap: map over the value, keeping the environment.
    pub fn fmap<B: Clone, F>(&self, f: F) -> EnvComonad<E, B>
    where
        F: Fn(A) -> B,
    {
        EnvComonad::new(self.env.clone(), f(self.value.clone()))
    }

    /// Local: modify the environment for a sub-computation.
    pub fn local<F2>(&self, f: F2) -> EnvComonad<E, A>
    where
        F2: Fn(E) -> E,
    {
        EnvComonad::new(f(self.env.clone()), self.value.clone())
    }
}

/// Store comonad: a value indexed by a position (dual of State monad).
/// Uses Rc<dyn Fn> for cheap cloning.
pub struct StoreComonad<S, A> {
    pub pos: S,
    pub peek: Rc<dyn Fn(S) -> A>,
}

impl<S: Clone, A: Clone> Clone for StoreComonad<S, A> {
    fn clone(&self) -> Self {
        StoreComonad {
            pos: self.pos.clone(),
            peek: self.peek.clone(),
        }
    }
}

impl<S: Clone + 'static, A: Clone + 'static> StoreComonad<S, A> {
    pub fn new(pos: S, peek: impl Fn(S) -> A + 'static) -> Self {
        Self { pos, peek: Rc::new(peek) }
    }

    /// extract: get value at current position.
    pub fn extract(&self) -> A {
        (self.peek)(self.pos.clone())
    }

    /// duplicate: create a store of stores.
    pub fn duplicate(&self) -> StoreComonad<S, StoreComonad<S, A>> {
        let peek = self.peek.clone();
        StoreComonad::new(self.pos.clone(), move |s: S| {
            StoreComonad {
                pos: s,
                peek: peek.clone(),
            }
        })
    }

    /// extend: context-dependent mapping.
    pub fn extend<B: Clone + 'static, F>(&self, f: F) -> StoreComonad<S, B>
    where
        F: Fn(&StoreComonad<S, A>) -> B + 'static,
    {
        let b = f(self);
        StoreComonad::new(self.pos.clone(), move |_s: S| b.clone())
    }

    /// seek: move to a new position.
    pub fn seek(&self, new_pos: S) -> StoreComonad<S, A> {
        StoreComonad {
            pos: new_pos,
            peek: self.peek.clone(),
        }
    }

    /// Peek at a different position without moving.
    pub fn peek_at(&self, s: S) -> A {
        (self.peek)(s)
    }
}

/// Identity comonad: simplest comonad, just wraps a value.
#[derive(Clone, Debug, PartialEq)]
pub struct IdentityComonad<A>(pub A);

impl<A: Clone> IdentityComonad<A> {
    pub fn new(a: A) -> Self {
        IdentityComonad(a)
    }

    pub fn extract(&self) -> A {
        self.0.clone()
    }

    pub fn duplicate(&self) -> IdentityComonad<IdentityComonad<A>> {
        IdentityComonad(IdentityComonad(self.0.clone()))
    }

    pub fn extend<B, F>(&self, f: F) -> IdentityComonad<B>
    where
        F: Fn(&IdentityComonad<A>) -> B,
    {
        IdentityComonad(f(self))
    }

    pub fn fmap<B, F>(&self, f: F) -> IdentityComonad<B>
    where
        F: Fn(A) -> B,
    {
        IdentityComonad(f(self.0.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Stream Comonad tests ---

    #[test]
    fn test_stream_extract() {
        let s = StreamComonad::new(42, vec![43, 44]);
        assert_eq!(s.extract(), 42);
    }

    #[test]
    fn test_stream_fmap() {
        let s = StreamComonad::new(1, vec![2, 3]);
        let doubled = s.fmap(|x| x * 2);
        assert_eq!(doubled.focus, 2);
        assert_eq!(doubled.tail, vec![4, 6]);
    }

    #[test]
    fn test_stream_extend() {
        let s = StreamComonad::new(1, vec![2, 3]);
        let extended = s.extend(|st| st.focus + st.tail.first().unwrap_or(&0));
        assert_eq!(extended.focus, 3); // 1 + 2
    }

    #[test]
    fn test_stream_duplicate_extract_identity() {
        let s = StreamComonad::new(10, vec![20]);
        let dup = s.duplicate();
        assert_eq!(dup.extract().extract(), 10);
    }

    // --- Env Comonad tests ---

    #[test]
    fn test_env_extract() {
        let env = EnvComonad::new("config", 42);
        assert_eq!(env.extract(), 42);
        assert_eq!(env.extract_env(), "config");
    }

    #[test]
    fn test_env_fmap() {
        let env = EnvComonad::new(100, 5);
        let mapped = env.fmap(|x| x * 3);
        assert_eq!(mapped.value, 15);
        assert_eq!(mapped.env, 100);
    }

    #[test]
    fn test_env_extend() {
        let env = EnvComonad::new(10, 5);
        let extended = env.extend(|e| e.value + e.env);
        assert_eq!(extended.value, 15);
        assert_eq!(extended.env, 10);
    }

    #[test]
    fn test_env_duplicate_extract() {
        let env = EnvComonad::new("ctx", 99);
        let dup = env.duplicate();
        assert_eq!(dup.extract().extract(), 99);
    }

    #[test]
    fn test_env_local() {
        let env = EnvComonad::new(10, 5);
        let local = env.local(|e| e + 100);
        assert_eq!(local.env, 110);
        assert_eq!(local.value, 5);
    }

    // --- Store Comonad tests ---

    #[test]
    fn test_store_extract() {
        let data: Vec<i32> = vec![10, 20, 30, 40, 50];
        let store = StoreComonad::new(2usize, move |i: usize| data[i]);
        assert_eq!(store.extract(), 30);
    }

    #[test]
    fn test_store_seek() {
        let data: Vec<i32> = vec![10, 20, 30];
        let store = StoreComonad::new(0usize, move |i: usize| data[i]);
        let moved = store.seek(2);
        assert_eq!(moved.extract(), 30);
    }

    #[test]
    fn test_store_peek_at() {
        let data: Vec<i32> = vec![10, 20, 30];
        let store = StoreComonad::new(0usize, move |i: usize| data[i]);
        assert_eq!(store.peek_at(1), 20);
    }

    #[test]
    fn test_store_duplicate_extract() {
        let data: Vec<i32> = vec![100, 200, 300];
        let store = StoreComonad::new(1usize, move |i: usize| data[i]);
        let dup = store.duplicate();
        assert_eq!(dup.extract().extract(), 200);
    }

    // --- Identity Comonad tests ---

    #[test]
    fn test_identity_extract() {
        let id = IdentityComonad::new(42);
        assert_eq!(id.extract(), 42);
    }

    #[test]
    fn test_identity_fmap() {
        let id = IdentityComonad::new(5);
        let mapped = id.fmap(|x| x * 10);
        assert_eq!(mapped, IdentityComonad(50));
    }

    #[test]
    fn test_identity_extend() {
        let id = IdentityComonad::new(7);
        let extended = id.extend(|x| x.extract() + 3);
        assert_eq!(extended, IdentityComonad(10));
    }

    #[test]
    fn test_identity_duplicate() {
        let id = IdentityComonad::new(99);
        let dup = id.duplicate();
        assert_eq!(dup.extract().extract(), 99);
    }
}
