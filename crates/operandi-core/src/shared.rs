//! Copy-on-write sharing for large payloads (mesh data and mesh identities)
//! that history checkpoints, live-tool bases, staged editor copies and stage
//! snapshots would otherwise deep-copy on every command.
//!
//! `Shared<T>` behaves like an owned `T`: reads deref to the shared value, and
//! any `&mut` access first clones the value if another snapshot still holds it
//! (`Arc::make_mut`). Cloning a handle is a reference-count increment.
//! Equality checks pointer identity first and falls back to full value
//! equality. That matches `T`'s equality except that a value is always equal to
//! itself even if it holds a NaN; mesh and identity validation reject
//! non-finite data anyway. Serialization and `Debug` are transparent, so
//! persisted files and diagnostics are unchanged.
//!
//! Validation memo: an allocation can remember that a pure validation already
//! passed for its exact contents. Mutation through `DerefMut` either edits a
//! uniquely held allocation (the memo is cleared first) or clones into a new
//! allocation (the clone starts with an empty memo), so a recorded pass never
//! outlives the contents it checked. Only passes are recorded; a failing check
//! reruns every time. The memo holds `Weak` handles to partners, which keep the
//! partner's allocation address reserved, so a recorded pointer can never be
//! reused by an unrelated value.
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::any::Any;
use std::sync::{Arc, Mutex, Weak};

type AnyPartner = Weak<dyn Any + Send + Sync>;

struct Payload<T> {
    value: T,
    /// A pass of this value's own validation, for its current contents.
    self_checked: Mutex<bool>,
    /// The partner allocation this value last validated against (for
    /// example the mesh an identity was checked against).
    partner_checked: Mutex<Option<AnyPartner>>,
    /// Results derived from the value (`derived`), one per result type,
    /// for its current contents.
    derived: Mutex<Vec<Arc<dyn Any + Send + Sync>>>,
}
impl<T> Payload<T> {
    fn new(value: T) -> Self {
        Self {
            value,
            self_checked: Mutex::new(false),
            partner_checked: Mutex::new(None),
            derived: Mutex::new(Vec::new()),
        }
    }
    fn forget(&mut self) {
        *self
            .self_checked
            .get_mut()
            .unwrap_or_else(|e| e.into_inner()) = false;
        *self
            .partner_checked
            .get_mut()
            .unwrap_or_else(|e| e.into_inner()) = None;
        self.derived
            .get_mut()
            .unwrap_or_else(|e| e.into_inner())
            .clear();
    }
}
impl<T: Clone> Clone for Payload<T> {
    /// A copy is new contents as far as the memo is concerned.
    fn clone(&self) -> Self {
        Self::new(self.value.clone())
    }
}

pub struct Shared<T>(Arc<Payload<T>>);

impl<T> Shared<T> {
    pub fn new(value: T) -> Self {
        Self(Arc::new(Payload::new(value)))
    }
    /// True when both handles refer to the same allocation, which implies
    /// equal content. False does not imply different content.
    pub fn same(a: &Self, b: &Self) -> bool {
        Arc::ptr_eq(&a.0, &b.0)
    }
    /// The allocation's address: an identity while the handle is alive
    /// (undo_memory.rs counts each allocation once).
    pub(crate) fn addr(&self) -> usize {
        Arc::as_ptr(&self.0) as *const () as usize
    }
    /// Runs `check` unless it already passed for this exact allocation.
    /// Callers must always pass the same pure check for a given `T`.
    pub(crate) fn checked(&self, check: impl FnOnce(&T) -> crate::Result<()>) -> crate::Result<()> {
        if *self
            .0
            .self_checked
            .lock()
            .unwrap_or_else(|e| e.into_inner())
        {
            return Ok(());
        }
        check(&self.0.value)?;
        *self
            .0
            .self_checked
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = true;
        Ok(())
    }
}
impl<T> Shared<T> {
    /// `compute(value)`, remembered for this allocation's current contents.
    /// Callers must always pass the same pure function for a given `T` and
    /// result type; each result type has its own slot (the point extent and
    /// the live patch surface of one mesh are both kept).
    pub(crate) fn derived<R: Send + Sync + 'static>(
        &self,
        compute: impl FnOnce(&T) -> R,
    ) -> Arc<R> {
        let mut memo = self.0.derived.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(hit) = memo.iter().find_map(|m| m.clone().downcast::<R>().ok()) {
            return hit;
        }
        let value = Arc::new(compute(&self.0.value));
        memo.push(value.clone());
        value
    }
    /// Whether this allocation's own validation has passed (`checked`).
    pub(crate) fn passed(&self) -> bool {
        *self
            .0
            .self_checked
            .lock()
            .unwrap_or_else(|e| e.into_inner())
    }
    /// Records that this allocation passed its own validation without running
    /// it. Only for callers holding a proof that is equivalent to that full
    /// check for these exact contents (see `live_extrude.rs` fast path).
    pub(crate) fn record_checked(&self) {
        *self
            .0
            .self_checked
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = true;
    }
}
impl<T: Send + Sync + 'static> Shared<T> {
    /// Like `record_checked`, for the check against `partner`.
    pub(crate) fn record_checked_against<U: Send + Sync + 'static>(&self, partner: &Shared<U>) {
        let weak: Weak<Payload<U>> = Arc::downgrade(&partner.0);
        *self
            .0
            .partner_checked
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(weak as AnyPartner);
    }
    /// The partner this value last passed its check against, if that
    /// allocation is still alive and of type `U`.
    pub(crate) fn checked_partner<U: Send + Sync + 'static>(&self) -> Option<Shared<U>> {
        let weak = self
            .0
            .partner_checked
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()?;
        let any: Arc<dyn Any + Send + Sync> = weak.upgrade()?;
        any.downcast::<Payload<U>>().ok().map(Shared)
    }
    /// Runs `check(self, partner)` unless it already passed for these exact
    /// two allocations. Callers must always pass the same pure check for a
    /// given pair of types.
    pub(crate) fn checked_against<U: Send + Sync + 'static>(
        &self,
        partner: &Shared<U>,
        check: impl FnOnce(&T, &U) -> crate::Result<()>,
    ) -> crate::Result<()> {
        let partner_ptr = Arc::as_ptr(&partner.0) as *const ();
        let recorded = self
            .0
            .partner_checked
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .is_some_and(|w| w.as_ptr() as *const () == partner_ptr);
        if recorded {
            return Ok(());
        }
        check(&self.0.value, &partner.0.value)?;
        let weak: Weak<Payload<U>> = Arc::downgrade(&partner.0);
        *self
            .0
            .partner_checked
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(weak as AnyPartner);
        Ok(())
    }
}
/// A handle that remembers one `Shared` allocation without keeping its
/// value alive. Its address stays reserved while this exists, and a change
/// through `DerefMut` always moves a value that has weak handles to a new
/// allocation (`Arc::make_mut`), so `is` means "these exact contents".
pub struct WeakShared<T>(Weak<Payload<T>>);
impl<T> Shared<T> {
    pub fn downgrade(&self) -> WeakShared<T> {
        WeakShared(Arc::downgrade(&self.0))
    }
}
impl<T> WeakShared<T> {
    /// True when `shared` is the remembered allocation (same contents).
    pub fn is(&self, shared: &Shared<T>) -> bool {
        std::ptr::eq(self.0.as_ptr(), Arc::as_ptr(&shared.0))
    }
}
impl<T> Clone for WeakShared<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T: Clone> Shared<T> {
    /// The value itself: moved out when this is the only handle, copied
    /// otherwise.
    pub fn into_value(self) -> T {
        match Arc::try_unwrap(self.0) {
            Ok(payload) => payload.value,
            Err(shared) => shared.value.clone(),
        }
    }
    /// Owned copy of the value, for callers that need a detached `T`.
    pub fn to_owned_value(&self) -> T {
        self.0.value.clone()
    }
}
impl<T: Default> Default for Shared<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}
impl<T> Clone for Shared<T> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}
impl<T> std::ops::Deref for Shared<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.0.value
    }
}
impl<T: Clone> std::ops::DerefMut for Shared<T> {
    fn deref_mut(&mut self) -> &mut T {
        let payload = Arc::make_mut(&mut self.0);
        // Unique holder about to edit in place: earlier passes no longer apply.
        payload.forget();
        &mut payload.value
    }
}
impl<T> From<T> for Shared<T> {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}
impl<T: PartialEq> PartialEq for Shared<T> {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0) || self.0.value == other.0.value
    }
}
impl<T: Eq> Eq for Shared<T> {}
/// Lets callers compare a shared payload with a detached value directly.
impl<T: PartialEq> PartialEq<T> for Shared<T> {
    fn eq(&self, other: &T) -> bool {
        self.0.value == *other
    }
}
impl<T: std::fmt::Debug> std::fmt::Debug for Shared<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.value.fmt(f)
    }
}
impl<T: Serialize> Serialize for Shared<T> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.0.value.serialize(s)
    }
}
impl<'de, T: Deserialize<'de>> Deserialize<'de> for Shared<T> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        T::deserialize(d).map(Self::new)
    }
}

/// A shared collection iterates and collects like the collection itself,
/// so a field can become shared without touching its readers.
impl<'a, T> IntoIterator for &'a Shared<T>
where
    &'a T: IntoIterator,
{
    type Item = <&'a T as IntoIterator>::Item;
    type IntoIter = <&'a T as IntoIterator>::IntoIter;
    fn into_iter(self) -> Self::IntoIter {
        (&self.0.value).into_iter()
    }
}
impl<'a, T: Clone> IntoIterator for &'a mut Shared<T>
where
    &'a mut T: IntoIterator,
{
    type Item = <&'a mut T as IntoIterator>::Item;
    type IntoIter = <&'a mut T as IntoIterator>::IntoIter;
    fn into_iter(self) -> Self::IntoIter {
        std::ops::DerefMut::deref_mut(self).into_iter()
    }
}
impl<T: Clone + IntoIterator> IntoIterator for Shared<T> {
    type Item = T::Item;
    type IntoIter = T::IntoIter;
    fn into_iter(self) -> Self::IntoIter {
        self.into_value().into_iter()
    }
}
impl<A, T: FromIterator<A>> FromIterator<A> for Shared<T> {
    fn from_iter<I: IntoIterator<Item = A>>(iter: I) -> Self {
        Self::new(iter.into_iter().collect())
    }
}
impl<T: PartialEq> PartialEq<Shared<Vec<T>>> for [T] {
    fn eq(&self, other: &Shared<Vec<T>>) -> bool {
        *self == other.0.value[..]
    }
}
impl<T: PartialEq> PartialEq<Shared<Vec<T>>> for &[T] {
    fn eq(&self, other: &Shared<Vec<T>>) -> bool {
        **self == other.0.value[..]
    }
}
impl<T: PartialEq> PartialEq<Shared<Vec<T>>> for Vec<T> {
    fn eq(&self, other: &Shared<Vec<T>>) -> bool {
        *self == other.0.value
    }
}
impl<T: Extend<A> + Clone, A> Extend<A> for Shared<T> {
    fn extend<I: IntoIterator<Item = A>>(&mut self, iter: I) {
        std::ops::DerefMut::deref_mut(self).extend(iter)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn memo_never_survives_a_change_of_contents() {
        let calls = std::cell::Cell::new(0);
        let check = |v: &Vec<i32>| {
            calls.set(calls.get() + 1);
            if v.contains(&0) {
                Err("zero".to_owned())
            } else {
                Ok(())
            }
        };
        let mut a = Shared::new(vec![1, 2]);
        a.checked(check).unwrap();
        a.checked(check).unwrap();
        assert_eq!(calls.get(), 1, "second pass is memoized");
        let snapshot = a.clone();
        a.push(0); // shared: copy-on-write into a fresh, unchecked allocation
        assert!(a.checked(check).is_err());
        snapshot.checked(check).unwrap();
        assert_eq!(calls.get(), 2, "snapshot keeps its own pass");
        let mut b = Shared::new(vec![3]);
        b.checked(check).unwrap();
        b.push(0); // unique: edited in place, memo cleared
        assert!(b.checked(check).is_err());
        assert!(b.checked(check).is_err(), "failures are never memoized");
        assert_eq!(calls.get(), 5);
    }
    #[test]
    fn partner_memo_is_specific_to_the_partner_allocation() {
        let calls = std::cell::Cell::new(0);
        let check = |a: &u8, b: &u8| {
            calls.set(calls.get() + 1);
            if a == b {
                Ok(())
            } else {
                Err("mismatch".to_owned())
            }
        };
        let ids = Shared::new(1u8);
        let mesh = Shared::new(1u8);
        ids.checked_against(&mesh, check).unwrap();
        ids.checked_against(&mesh.clone(), check).unwrap();
        assert_eq!(calls.get(), 1);
        let other = Shared::new(2u8);
        assert!(ids.checked_against(&other, check).is_err());
        let equal_copy = Shared::new(1u8);
        ids.checked_against(&equal_copy, check).unwrap();
        assert_eq!(calls.get(), 3);
        assert!(ids == Shared::new(1u8) && mesh == 1u8);
    }
}
