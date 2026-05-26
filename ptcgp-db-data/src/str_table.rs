use std::marker::PhantomData;

use smallvec::SmallVec;
use smol_str::{SmolStr, StrExt};

pub struct StrTable {
    strs: &'static [&'static str],
    lower: &'static [&'static str],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StrEntry {
    id: usize,
    str: &'static str,
}

pub type Iter = std::iter::Copied<std::slice::Iter<'static, &'static str>>;

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

    pub const fn len(&self) -> usize {
        self.strs.len()
    }

    pub const fn is_empty(&self) -> bool {
        self.strs.is_empty()
    }

    pub const fn as_strs(&self) -> &'static [&'static str] {
        self.strs
    }

    pub fn iter(&self) -> Iter {
        self.strs.iter().copied()
    }

    pub const unsafe fn get_unchecked(&self, id: usize) -> &'static str {
        unsafe { *self.strs.as_ptr().add(id) }
    }

    pub const fn get(&self, id: usize) -> Option<&'static str> {
        if id < self.strs.len() {
            Some(unsafe { self.get_unchecked(id) })
        } else {
            None
        }
    }

    pub const unsafe fn get_entry_unchecked(&self, id: usize) -> StrEntry {
        StrEntry {
            id,
            str: unsafe { self.get_unchecked(id) },
        }
    }

    pub const fn get_entry(&self, id: usize) -> Option<StrEntry> {
        if id < self.strs.len() {
            Some(unsafe { self.get_entry_unchecked(id) })
        } else {
            None
        }
    }

    // Performs a tokenized, case insensitive search. `needle` is split on whitespace
    // to create multiple "tokens". The returned iterator yields all entries that
    // contain (case insensitively) any of these tokens.
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
    pub const fn id(&self) -> usize {
        self.id
    }

    pub const fn as_str(&self) -> &str {
        self.str
    }

    pub const fn len(&self) -> usize {
        self.str.len()
    }

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
