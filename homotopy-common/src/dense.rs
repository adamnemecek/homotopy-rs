use std::{collections::VecDeque, fmt, mem};

use serde::{Deserialize, Serialize};

use crate::{
    declare_idx,
    idx::{Idx, IdxVec},
};

declare_idx! {
    #[derive(Serialize, Deserialize)]
    struct AllocId = usize;
}

#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DenseVec<I, T> {
    forward: IdxVec<I, Option<AllocId>>,
    reverse: IdxVec<AllocId, I>,
    raw: IdxVec<AllocId, T>,
    free: VecDeque<I>,
}

impl<I, T> DenseVec<I, T>
where
    I: Idx,
{
    // Moves the last element in the storage-backing `IdxVec` into the slot
    // provided. This is used to ensure that all `AllocId`s point to valid
    // data. (Note this is a no-op if the backing `IdxVec` is empty.)
    fn realloc(&mut self, slot: AllocId) -> Option<T> {
        let mut elem = self.raw.pop()?;
        let idx = self.reverse.pop()?;

        // If we're reallocating the last element of the storage-backing
        // `IdxVec`, that means it's time for it to be deallocated, so just
        // fast resturn.
        if slot.index() == self.raw.len() {
            return Some(elem);
        }

        self.forward[idx] = Some(slot);
        mem::swap(&mut self.raw[slot], &mut elem);
        Some(elem)
    }

    #[inline]
    pub fn new() -> Self {
        Self {
            forward: <_>::default(),
            reverse: <_>::default(),
            raw: <_>::default(),
            free: <_>::default(),
        }
    }

    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            forward: IdxVec::with_capacity(capacity),
            reverse: IdxVec::with_capacity(capacity),
            raw: IdxVec::with_capacity(capacity),
            free: VecDeque::new(),
        }
    }

    #[inline]
    pub fn push(&mut self, elem: T) -> I {
        let idx = self
            .free
            .pop_front()
            .unwrap_or_else(|| self.forward.push(None));
        let slot = self.raw.push(elem);

        self.reverse.push(idx);
        self.forward[idx] = Some(slot);
        idx
    }

    #[inline]
    pub fn remove(&mut self, index: I) -> Option<T> {
        let slot = self.forward[index].take()?;
        self.free.push_back(index);
        self.realloc(slot)
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.raw.is_empty()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.raw.len()
    }

    #[inline]
    pub fn contains_key(&self, index: I) -> bool {
        self.forward[index].is_some()
    }

    #[inline]
    pub fn get(&self, index: I) -> Option<&T> {
        self.raw.get(self.forward[index]?)
    }

    #[inline]
    pub fn get_mut(&mut self, index: I) -> Option<&mut T> {
        self.raw.get_mut(self.forward[index]?)
    }

    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = (I, &T)> {
        self.keys().zip(self.values())
    }

    #[inline]
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (I, &mut T)> + '_ {
        self.reverse.values().copied().zip(self.raw.values_mut())
    }

    #[inline]
    pub fn keys(&self) -> impl Iterator<Item = I> + '_ {
        self.reverse.values().copied()
    }

    #[inline]
    pub fn values(&self) -> impl Iterator<Item = &T> {
        self.raw.values()
    }

    #[inline]
    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.raw.values_mut()
    }

    #[inline]
    pub fn clear(&mut self) {
        self.forward.clear();
        self.reverse.clear();
        self.raw.clear();
        self.free.clear();
    }
}

impl<I, T> fmt::Debug for DenseVec<I, T>
where
    T: fmt::Debug,
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.raw.fmt(f)
    }
}

impl<I, T> Default for DenseVec<I, T>
where
    I: Idx,
{
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}
