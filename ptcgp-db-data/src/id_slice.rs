//! Indirection slice: a DST that resolves stored IDs to `&'static T` on access.

use std::marker::PhantomData;

/// A dynamically-sized slice of IDs that resolves each ID to `&'static T` on access.
///
/// Stored in memory as a `[usize]` (via `transmute`). Accessing an element performs a single
/// array index into `T::INDEXED` — no allocation, no copying. This allows many-to-many
/// relationships (e.g., a card's attacks, a pack's card pool) to be stored compactly without
/// duplicating the pointed-to data.
pub struct IdSlice<T: 'static> {
    _phantom: PhantomData<fn() -> &'static T>,
    slice: [usize],
}

/// Iterator over an [`IdSlice<T>`], yielding `&'static T`.
pub struct Iter<'a, T: 'static> {
    iter: std::slice::Iter<'a, usize>,
    _phantom: PhantomData<fn() -> &'static T>,
}

/// Marker trait connecting `T` to its global backing slice, enabling [`IdSlice<T>`] to resolve
/// IDs to references.
pub trait Indexed: Sized + 'static {
    /// The global static slice that IDs index into.
    const INDEXED: &'static [Self];
}

mod sealed {
    pub trait Sealed {}

    impl Sealed for usize {}

    impl Sealed for [usize] {}
}

/// Bridges the output of [`SliceIndex`] on `[usize]` to the corresponding output on [`IdSlice<T>`].
///
/// Implemented for:
/// - `usize` — the output of single-element indexing (`slice[i]`); resolves to `&T`
/// - `[usize]` — the output of range indexing (`slice[a..b]`, `slice[..n]`, etc.); resolves to
///   `&IdSlice<T>`, rewrapping the sub-slice of IDs
///
/// Callers index with a `usize` or a range; `[usize]` is an intermediate output type produced
/// by [`SliceIndex`], never passed directly by the user.
///
/// [`SliceIndex`]: std::slice::SliceIndex
pub trait IdSliceIndex<T: Indexed>: sealed::Sealed + 'static {
    type Output: ?Sized;

    fn indirect(&self) -> &Self::Output;
}

impl<T: Indexed> IdSliceIndex<T> for usize {
    type Output = T;

    fn indirect(&self) -> &Self::Output {
        &T::INDEXED[*self]
    }
}

impl<T: Indexed> IdSliceIndex<T> for [usize] {
    type Output = IdSlice<T>;

    fn indirect(&self) -> &Self::Output {
        unsafe { IdSlice::new_unchecked(self) }
    }
}

impl<T: Indexed> IdSlice<T> {
    /// Creates an `IdSlice` from a slice of IDs without validating that they are in range.
    ///
    /// # Safety
    ///
    /// Every element of `indexes` must be a valid index into `T::INDEXED`.
    pub const unsafe fn new_unchecked(indexes: &[usize]) -> &Self {
        unsafe { core::mem::transmute(indexes) }
    }

    /// Number of elements.
    pub const fn len(&self) -> usize {
        self.slice.len()
    }

    /// True if the slice contains no elements.
    pub const fn is_empty(&self) -> bool {
        self.slice.is_empty()
    }

    /// The underlying ID slice, useful for serialization or binary search by ID.
    pub const fn as_ids(&self) -> &[usize] {
        &self.slice
    }

    /// First element, or `None` if empty.
    pub const fn first(&self) -> Option<&'static T> {
        if let Some(id) = self.slice.first() {
            Some(&T::INDEXED[*id])
        } else {
            None
        }
    }

    /// Last element, or `None` if empty.
    pub const fn last(&self) -> Option<&'static T> {
        if let Some(id) = self.slice.last() {
            Some(&T::INDEXED[*id])
        } else {
            None
        }
    }

    /// Element at position `n`. Panics if out of bounds.
    pub const fn get_at(&self, n: usize) -> &'static T {
        &T::INDEXED[self.slice[n]]
    }

    /// Sub-slice over the given index range. Panics if the range is invalid or out of bounds.
    pub const fn get_slice(&self, range: std::ops::Range<usize>) -> &Self {
        if range.start > range.end {
            panic!("invalid slice range");
        }
        if range.start > self.slice.len() || range.end > self.slice.len() {
            panic!("slice range out of range");
        }
        let ptr = unsafe { self.slice.as_ptr().add(range.start) };
        let len = range.end - range.start;
        unsafe { Self::new_unchecked(std::slice::from_raw_parts(ptr, len)) }
    }

    /// Returns the element or sub-slice at `index` without bounds checking.
    ///
    /// # Safety
    ///
    /// `index` must be in range for the underlying ID slice.
    pub unsafe fn get_unchecked<I>(&self, index: I) -> &<I::Output as IdSliceIndex<T>>::Output
    where
        I: std::slice::SliceIndex<[usize]>,
        I::Output: IdSliceIndex<T>,
    {
        let index = unsafe { self.slice.get_unchecked(index) };
        index.indirect()
    }

    /// Returns the element or sub-slice at `index`, or `None` if out of bounds.
    pub fn get<I>(&self, index: I) -> Option<&<I::Output as IdSliceIndex<T>>::Output>
    where
        I: std::slice::SliceIndex<[usize]>,
        I::Output: IdSliceIndex<T>,
    {
        if let Some(index) = self.slice.get(index) {
            Some(index.indirect())
        } else {
            None
        }
    }

    /// Splits off the first element, returning `(first, rest)`, or `None` if empty.
    pub const fn split_first(&self) -> Option<(&'static T, &Self)> {
        if let Some((head, tail)) = self.slice.split_first() {
            let head = &T::INDEXED[*head];
            let tail = unsafe { Self::new_unchecked(tail) };
            Some((head, tail))
        } else {
            None
        }
    }

    /// Splits off the last element, returning `(last, rest)`, or `None` if empty.
    pub const fn split_last(&self) -> Option<(&'static T, &Self)> {
        if let Some((tail, head)) = self.slice.split_last() {
            let tail = &T::INDEXED[*tail];
            let head = unsafe { Self::new_unchecked(head) };
            Some((tail, head))
        } else {
            None
        }
    }

    /// Splits at position `mid` without bounds checking, returning `(head, tail)`.
    ///
    /// # Safety
    ///
    /// `mid` must be ≤ `self.len()`.
    pub const unsafe fn split_at_unchecked(&self, mid: usize) -> (&Self, &Self) {
        let (head, tail) = unsafe { self.slice.split_at_unchecked(mid) };
        (unsafe { Self::new_unchecked(head) }, unsafe {
            Self::new_unchecked(tail)
        })
    }

    /// Splits at position `mid`, returning `(head, tail)`. Panics if `mid > self.len()`.
    pub const fn split_at(&self, mid: usize) -> (&Self, &Self) {
        let (head, tail) = self.slice.split_at(mid);
        (unsafe { Self::new_unchecked(head) }, unsafe {
            Self::new_unchecked(tail)
        })
    }

    /// Splits at position `mid`, returning `Some((head, tail))`, or `None` if out of bounds.
    pub const fn split_at_checked(&self, mid: usize) -> Option<(&Self, &Self)> {
        if let Some((head, tail)) = self.slice.split_at_checked(mid) {
            Some((unsafe { Self::new_unchecked(head) }, unsafe {
                Self::new_unchecked(tail)
            }))
        } else {
            None
        }
    }

    /// Returns an iterator yielding `&'static T` in order.
    pub fn iter(&self) -> Iter<'_, T> {
        Iter {
            iter: self.slice.iter(),
            _phantom: PhantomData,
        }
    }
}

impl<T: 'static> Default for &IdSlice<T> {
    fn default() -> Self {
        const DEFAULT_INDEXES: &[usize] = &[];
        const { unsafe { std::mem::transmute(DEFAULT_INDEXES) } }
    }
}

impl<T> std::fmt::Debug for IdSlice<T>
where
    T: Indexed + std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self).finish()
    }
}

impl<T: Eq + Indexed> Eq for IdSlice<T> {}

impl<T> PartialEq for IdSlice<T>
where
    T: PartialEq + Indexed,
{
    fn eq(&self, other: &Self) -> bool {
        self.iter().eq(other)
    }
}

impl<T> PartialEq<[T]> for IdSlice<T>
where
    T: PartialEq + Indexed,
{
    fn eq(&self, other: &[T]) -> bool {
        self.iter().eq(other)
    }
}

impl<T> PartialEq<IdSlice<T>> for [T]
where
    T: PartialEq + Indexed,
{
    fn eq(&self, other: &IdSlice<T>) -> bool {
        self.iter().eq(other)
    }
}

impl<T> Ord for IdSlice<T>
where
    T: Ord + Indexed,
{
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.iter().cmp(other)
    }
}

impl<T> PartialOrd for IdSlice<T>
where
    T: PartialOrd + Indexed,
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.iter().partial_cmp(other)
    }
}

impl<T> PartialOrd<[T]> for IdSlice<T>
where
    T: PartialOrd + Indexed,
{
    fn partial_cmp(&self, other: &[T]) -> Option<std::cmp::Ordering> {
        self.iter().partial_cmp(other)
    }
}

impl<T> PartialOrd<IdSlice<T>> for [T]
where
    T: PartialOrd + Indexed,
{
    fn partial_cmp(&self, other: &IdSlice<T>) -> Option<std::cmp::Ordering> {
        self.iter().partial_cmp(other)
    }
}

impl<T> std::hash::Hash for IdSlice<T>
where
    T: std::hash::Hash + Indexed,
{
    fn hash<H>(&self, state: &mut H)
    where
        H: std::hash::Hasher,
    {
        self.iter().for_each(|item| item.hash(state));
    }
}

impl<'a, T: Indexed> IntoIterator for &'a IdSlice<T> {
    type Item = &'static T;
    type IntoIter = Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<T, I> std::ops::Index<I> for IdSlice<T>
where
    T: Indexed,
    I: std::slice::SliceIndex<[usize]>,
    I::Output: IdSliceIndex<T>,
{
    type Output = <I::Output as IdSliceIndex<T>>::Output;

    fn index(&self, index: I) -> &Self::Output {
        self.slice.index(index).indirect()
    }
}

impl<T: Indexed> Clone for Iter<'_, T> {
    fn clone(&self) -> Self {
        Self {
            iter: self.iter.clone(),
            _phantom: PhantomData,
        }
    }
}

impl<T> std::fmt::Debug for Iter<'_, T>
where
    T: Indexed + std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.clone()).finish()
    }
}

impl<T> Iterator for Iter<'_, T>
where
    T: Indexed,
{
    type Item = &'static T;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|id| &T::INDEXED[*id])
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.iter.nth(n).map(|id| &T::INDEXED[*id])
    }

    fn count(self) -> usize {
        self.iter.count()
    }

    fn last(self) -> Option<Self::Item> {
        self.iter.last().map(|id| &T::INDEXED[*id])
    }

    fn fold<B, F>(self, init: B, mut f: F) -> B
    where
        F: FnMut(B, Self::Item) -> B,
    {
        self.iter
            .fold(init, move |init, id| f(init, &T::INDEXED[*id]))
    }
}

impl<T> DoubleEndedIterator for Iter<'_, T>
where
    T: Indexed,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        self.iter.next_back().map(|id| &T::INDEXED[*id])
    }

    fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
        self.iter.nth_back(n).map(|id| &T::INDEXED[*id])
    }

    fn rfold<B, F>(self, init: B, mut f: F) -> B
    where
        F: FnMut(B, Self::Item) -> B,
    {
        self.iter
            .rfold(init, move |init, id| f(init, &T::INDEXED[*id]))
    }
}

impl<T> ExactSizeIterator for Iter<'_, T>
where
    T: Indexed,
{
    fn len(&self) -> usize {
        self.iter.len()
    }
}

impl<T: Indexed> std::iter::FusedIterator for Iter<'_, T> {}
