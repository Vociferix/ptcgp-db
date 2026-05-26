//! String table: a sorted, deduplicated store of `&'static str`s with fast search.

use std::marker::PhantomData;

use smallvec::SmallVec;
use smol_str::{SmolStr, StrExt};

/// A sorted, deduplicated table of `&'static str`s.
///
/// Strings are alphabetically sorted at build time. Each string has a stable numeric ID
/// usable for O(1) retrieval via [`get`] / [`get_entry`] and for compact storage (e.g., inside
/// generated data structures). [`search`] performs a tokenized, case-insensitive substring
/// search across the whole table.
///
/// [`get`]: StrTable::get
/// [`get_entry`]: StrTable::get_entry
/// [`search`]: StrTable::search
pub struct StrTable {
    strs: &'static [&'static str],
    lower: &'static [&'static str],
}

/// A string together with its [`StrTable`] index.
///
/// The ID is stable and can be used for equality comparisons or as a compact storage key.
/// Derefs to `&str` for transparent string operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StrEntry {
    id: usize,
    str: &'static str,
}

/// An iterator type alias for iterating over all strings in a [`StrTable`] in sorted order.
pub type Iter = std::iter::Copied<std::slice::Iter<'static, &'static str>>;

/// A tokenized, case-insensitive search iterator over a [`StrTable`].
///
/// Produced by [`StrTable::search`]. The query is split on whitespace; the iterator yields
/// all entries whose lowercase text contains any token as a substring. Implements
/// [`DoubleEndedIterator`].
#[derive(Debug, Clone)]
pub struct Search<'table> {
    front: usize,
    back: usize,
    strs: *const &'static str,
    lower: *const &'static str,
    needles: SmallVec<[SmolStr; 4]>,
    _phantom: PhantomData<&'table ()>,
}

impl StrTable {
    #[allow(dead_code)]
    pub(crate) const fn empty() -> Self {
        Self {
            strs: &[],
            lower: &[],
        }
    }

    // SAFETY: `strs` and `lower` must be the same length, and
    //         `strs` must be alphbetically sorted. Each index
    //         must satisfy `str.to_lower() == lower.to_string()`.
    pub(crate) const unsafe fn new_unchecked(
        strs: &'static [&'static str],
        lower: &'static [&'static str],
    ) -> Self {
        Self { strs, lower }
    }

    /// Number of strings in the table.
    pub const fn len(&self) -> usize {
        self.strs.len()
    }

    /// True if the table contains no strings.
    pub const fn is_empty(&self) -> bool {
        self.strs.is_empty()
    }

    /// The underlying `&'static str` slice in alphabetically sorted order.
    pub const fn as_strs(&self) -> &'static [&'static str] {
        self.strs
    }

    /// Iterator over all strings in the table in sorted order.
    pub fn iter(&self) -> Iter {
        self.strs.iter().copied()
    }

    /// Returns the string at `id` without bounds checking.
    ///
    /// # Safety
    ///
    /// `id` must be less than `self.len()`.
    pub const unsafe fn get_unchecked(&self, id: usize) -> &'static str {
        unsafe { *self.strs.as_ptr().add(id) }
    }

    /// Returns the string at `id`, or `None` if out of range.
    pub const fn get(&self, id: usize) -> Option<&'static str> {
        if id < self.strs.len() {
            Some(unsafe { self.get_unchecked(id) })
        } else {
            None
        }
    }

    /// Returns the string and its ID at `id` without bounds checking.
    ///
    /// # Safety
    ///
    /// `id` must be less than `self.len()`.
    pub const unsafe fn get_entry_unchecked(&self, id: usize) -> StrEntry {
        StrEntry {
            id,
            str: unsafe { self.get_unchecked(id) },
        }
    }

    /// Returns the string and its ID at `id`, or `None` if out of range.
    pub const fn get_entry(&self, id: usize) -> Option<StrEntry> {
        if id < self.strs.len() {
            Some(unsafe { self.get_entry_unchecked(id) })
        } else {
            None
        }
    }

    /// Returns an iterator over all entries whose lowercase text contains any
    /// whitespace-separated token from `needle` as a case-insensitive substring.
    ///
    /// An empty or whitespace-only `needle` matches nothing (the token list is empty).
    pub fn search(&self, needle: &str) -> Search<'_> {
        let needles: SmallVec<[SmolStr; 4]> = needle
            .trim()
            .split(char::is_whitespace)
            .filter_map(|needle| {
                let needle = needle.trim();
                if needle.is_empty() {
                    None
                } else {
                    Some(needle.to_lowercase_smolstr())
                }
            })
            .collect();
        Search {
            front: 0,
            back: self.strs.len(),
            strs: self.strs.as_ptr(),
            lower: self.lower.as_ptr(),
            needles,
            _phantom: PhantomData,
        }
    }
}

impl std::fmt::Debug for StrTable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self.strs, f)
    }
}

impl IntoIterator for &StrTable {
    type Item = &'static str;

    type IntoIter = Iter;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl std::ops::Index<usize> for StrTable {
    type Output = str;

    fn index(&self, index: usize) -> &str {
        self.strs[index]
    }
}

impl StrEntry {
    /// Numeric index into the originating [`StrTable`]. Stable across program runs; suitable
    /// as a compact storage key.
    pub const fn id(&self) -> usize {
        self.id
    }

    /// The string value.
    pub const fn as_str(&self) -> &str {
        self.str
    }

    /// Byte length of the string.
    pub const fn len(&self) -> usize {
        self.str.len()
    }

    /// True if the string is empty.
    pub const fn is_empty(&self) -> bool {
        self.str.is_empty()
    }
}

impl std::ops::Deref for StrEntry {
    type Target = str;

    fn deref(&self) -> &str {
        self.str
    }
}

impl std::borrow::Borrow<str> for StrEntry {
    fn borrow(&self) -> &str {
        self.str
    }
}

impl AsRef<str> for StrEntry {
    fn as_ref(&self) -> &str {
        self.str
    }
}

impl AsRef<[u8]> for StrEntry {
    fn as_ref(&self) -> &[u8] {
        self.str.as_bytes()
    }
}

impl std::fmt::Display for StrEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.str)
    }
}

impl Iterator for Search<'_> {
    type Item = StrEntry;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let pos = self.front;

            if pos >= self.back {
                return None;
            }

            self.front = unsafe { pos.unchecked_add(1) };

            let lower = unsafe { *self.lower.add(pos) };

            if self
                .needles
                .iter()
                .any(|needle| memchr::memmem::find(lower.as_bytes(), needle.as_bytes()).is_some())
            {
                return Some(StrEntry {
                    id: pos,
                    str: unsafe { *self.strs.add(pos) },
                });
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(self.back - self.front))
    }

    fn last(mut self) -> Option<Self::Item> {
        self.next_back()
    }
}

impl DoubleEndedIterator for Search<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        loop {
            let back = self.back;
            if back <= self.front {
                return None;
            }

            let pos = unsafe { back.unchecked_sub(1) };
            self.back = pos;

            let lower = unsafe { *self.lower.add(pos) };

            if self
                .needles
                .iter()
                .any(|needle| memchr::memmem::find(lower.as_bytes(), needle.as_bytes()).is_some())
            {
                return Some(StrEntry {
                    id: pos,
                    str: unsafe { *self.strs.add(pos) },
                });
            }
        }
    }
}

impl std::iter::FusedIterator for Search<'_> {}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_table(
        strs: &'static [&'static str],
        lower: &'static [&'static str],
    ) -> StrTable {
        unsafe { StrTable::new_unchecked(strs, lower) }
    }

    #[test]
    fn len_and_is_empty() {
        let empty = make_table(&[], &[]);
        assert!(empty.is_empty());
        assert_eq!(empty.len(), 0);

        let one = make_table(&["hello"], &["hello"]);
        assert!(!one.is_empty());
        assert_eq!(one.len(), 1);
    }

    #[test]
    fn get_and_get_entry() {
        let table = make_table(&["Alpha", "Beta"], &["alpha", "beta"]);
        assert_eq!(table.get(0), Some("Alpha"));
        assert_eq!(table.get(1), Some("Beta"));
        assert_eq!(table.get(2), None);

        let entry = table.get_entry(0).unwrap();
        assert_eq!(entry.id(), 0);
        assert_eq!(entry.as_str(), "Alpha");
        assert_eq!(&*entry, "Alpha");
        assert_eq!(entry.len(), 5);
    }

    #[test]
    fn search_case_insensitive() {
        let table = make_table(
            &["Bulbasaur", "Charmander", "Squirtle"],
            &["bulbasaur", "charmander", "squirtle"],
        );
        let results: Vec<_> = table.search("Char").collect();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].as_str(), "Charmander");
        assert_eq!(results[0].id(), 1);

        // All-lowercase query also matches
        let results: Vec<_> = table.search("char").collect();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn search_empty_needle_matches_nothing() {
        let table = make_table(&["Bulbasaur", "Charmander"], &["bulbasaur", "charmander"]);
        assert_eq!(table.search("").count(), 0);
        assert_eq!(table.search("   ").count(), 0);
        assert_eq!(table.search("\t\n").count(), 0);
    }

    #[test]
    fn search_multi_token_matches_any() {
        // Any one token matching is sufficient to include the entry
        let table = make_table(
            &["Bulbasaur", "Charmander", "Squirtle"],
            &["bulbasaur", "charmander", "squirtle"],
        );
        let mut results: Vec<_> = table.search("bulb squirt").collect();
        results.sort_by_key(|e| e.id());
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].as_str(), "Bulbasaur");
        assert_eq!(results[1].as_str(), "Squirtle");
    }

    #[test]
    fn search_double_ended() {
        let table = make_table(
            &["Bulbasaur", "Charmander", "Squirtle"],
            &["bulbasaur", "charmander", "squirtle"],
        );
        // All three contain 'r'; iterate from the back
        let mut iter = table.search("r");
        assert_eq!(iter.next_back().as_deref(), Some("Squirtle"));
        assert_eq!(iter.next_back().as_deref(), Some("Charmander"));
        assert_eq!(iter.next_back().as_deref(), Some("Bulbasaur"));
        assert_eq!(iter.next_back(), None);
    }

    #[test]
    fn search_meets_in_middle() {
        // Consuming from both ends should stop correctly when they meet
        let table = make_table(
            &["Ant", "Bear", "Cat", "Dog"],
            &["ant", "bear", "cat", "dog"],
        );
        let mut iter = table.search("a"); // Ant, Bear, Cat match (Dog has no 'a')
        assert_eq!(iter.next().as_deref(), Some("Ant"));
        assert_eq!(iter.next_back().as_deref(), Some("Cat"));
        assert_eq!(iter.next().as_deref(), Some("Bear"));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn str_entry_deref_and_borrow() {
        let table = make_table(&["hello"], &["hello"]);
        let entry = table.get_entry(0).unwrap();
        // Deref to &str
        assert_eq!(&*entry, "hello");
        // Borrow as &str
        let s: &str = &entry;
        assert_eq!(s, "hello");
        // starts_with via Deref
        assert!(entry.starts_with("hel"));
    }
}
