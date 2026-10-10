use std::borrow::Borrow;
use std::fmt;

use crate::hash::JavaHash;
use crate::hash_map::JavaHashMap;

pub struct JavaHashSet<E> {
    map: JavaHashMap<E, ()>,
}

pub fn jdk8_hash_set_copy_capacity(source_size: usize) -> i32 {
    ((source_size as f32 / 0.75) as i32 + 1).max(16)
}

impl<E> Default for JavaHashSet<E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<E> JavaHashSet<E> {
    pub fn new() -> Self {
        Self {
            map: JavaHashMap::new(),
        }
    }

    pub fn with_capacity(initial_capacity: i32) -> Self {
        Self {
            map: JavaHashMap::with_capacity(initial_capacity),
        }
    }

    pub fn with_capacity_and_load_factor(initial_capacity: i32, load_factor: f32) -> Self {
        Self {
            map: JavaHashMap::with_capacity_and_load_factor(initial_capacity, load_factor),
        }
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn capacity(&self) -> usize {
        self.map.capacity()
    }

    pub fn clear(&mut self) {
        self.map.clear();
    }

    pub fn iter(&self) -> impl ExactSizeIterator<Item = &E> {
        self.map.iter().map(|(element, _)| element)
    }
}

impl<E: JavaHash + Eq> JavaHashSet<E> {
    pub fn from_collection<I>(elements: I) -> Self
    where
        I: IntoIterator<Item = E>,
        I::IntoIter: ExactSizeIterator,
    {
        let elements = elements.into_iter();
        let mut set = Self::with_capacity(jdk8_hash_set_copy_capacity(elements.len()));
        set.add_all(elements);
        set
    }

    pub fn java_clone(&self) -> Self
    where
        E: Clone,
    {
        Self {
            map: self.map.java_clone(),
        }
    }

    pub fn add(&mut self, element: E) -> bool {
        self.map.put(element, ()).is_none()
    }

    pub fn add_all(&mut self, elements: impl IntoIterator<Item = E>) -> bool {
        elements
            .into_iter()
            .fold(false, |changed, element| self.add(element) | changed)
    }

    pub fn contains<Q>(&self, element: &Q) -> bool
    where
        E: Borrow<Q>,
        Q: JavaHash + Eq + ?Sized,
    {
        self.map.contains_key(element)
    }

    pub fn remove<Q>(&mut self, element: &Q) -> bool
    where
        E: Borrow<Q>,
        Q: JavaHash + Eq + ?Sized,
    {
        self.map.remove(element).is_some()
    }

    pub fn retain(&mut self, mut keep: impl FnMut(&E) -> bool) {
        self.map.retain(|element, _| keep(element));
    }

    pub fn remove_all_list(&mut self, others: &[E]) -> bool {
        if self.len() > others.len() {
            others
                .iter()
                .fold(false, |changed, element| self.remove(element) | changed)
        } else {
            let before = self.len();
            self.retain(|element| !others.contains(element));
            before != self.len()
        }
    }

    pub fn remove_all_set(&mut self, others: &JavaHashSet<E>) -> bool {
        if self.len() > others.len() {
            others
                .iter()
                .fold(false, |changed, element| self.remove(element) | changed)
        } else {
            let before = self.len();
            self.retain(|element| !others.contains(element));
            before != self.len()
        }
    }

    pub fn retain_all(&mut self, keep: impl Fn(&E) -> bool) -> bool {
        let before = self.len();
        self.retain(keep);
        before != self.len()
    }
}

impl<'a, E> IntoIterator for &'a JavaHashSet<E> {
    type Item = &'a E;
    type IntoIter = std::iter::Map<crate::hash_map::Iter<'a, E, ()>, fn((&'a E, &'a ())) -> &'a E>;

    fn into_iter(self) -> Self::IntoIter {
        self.map.iter().map(|(element, _)| element)
    }
}

impl<E: fmt::Debug> fmt::Debug for JavaHashSet<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_set().entries(self.iter()).finish()
    }
}

impl<E: JavaHash + Eq> FromIterator<E> for JavaHashSet<E> {
    fn from_iter<I: IntoIterator<Item = E>>(elements: I) -> Self {
        let mut set = Self::new();
        set.add_all(elements);
        set
    }
}

impl<E: JavaHash + Eq> Extend<E> for JavaHashSet<E> {
    fn extend<I: IntoIterator<Item = E>>(&mut self, elements: I) {
        self.add_all(elements);
    }
}
