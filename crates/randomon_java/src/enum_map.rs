use std::fmt;

pub trait JavaEnum: Copy + Eq {
    fn ordinal(&self) -> usize;
}

pub struct JavaEnumMap<K, V> {
    slots: Vec<Option<(K, V)>>,
    size: usize,
}

impl<K, V> Default for JavaEnumMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K, V> JavaEnumMap<K, V> {
    pub fn new() -> Self {
        Self {
            slots: Vec::new(),
            size: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.size
    }

    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    pub fn clear(&mut self) {
        self.slots.clear();
        self.size = 0;
    }

    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.slots.iter().flatten().map(|(key, value)| (key, value))
    }

    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.iter().map(|(key, _)| key)
    }

    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.iter().map(|(_, value)| value)
    }
}

impl<K: JavaEnum, V> JavaEnumMap<K, V> {
    pub fn put(&mut self, key: K, value: V) -> Option<V> {
        let ordinal = key.ordinal();
        if self.slots.len() <= ordinal {
            self.slots.resize_with(ordinal + 1, || None);
        }
        let previous = self.slots[ordinal]
            .replace((key, value))
            .map(|(_, old)| old);
        if previous.is_none() {
            self.size += 1;
        }
        previous
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        self.slots
            .get(key.ordinal())?
            .as_ref()
            .map(|(_, value)| value)
    }

    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        self.slots
            .get_mut(key.ordinal())?
            .as_mut()
            .map(|(_, value)| value)
    }

    pub fn contains_key(&self, key: &K) -> bool {
        self.get(key).is_some()
    }

    pub fn remove(&mut self, key: &K) -> Option<V> {
        let removed = self
            .slots
            .get_mut(key.ordinal())?
            .take()
            .map(|(_, value)| value);
        if removed.is_some() {
            self.size -= 1;
        }
        removed
    }
}

impl<K: fmt::Debug, V: fmt::Debug> fmt::Debug for JavaEnumMap<K, V> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_map().entries(self.iter()).finish()
    }
}

impl<K: JavaEnum, V> FromIterator<(K, V)> for JavaEnumMap<K, V> {
    fn from_iter<I: IntoIterator<Item = (K, V)>>(entries: I) -> Self {
        let mut map = Self::new();
        for (key, value) in entries {
            map.put(key, value);
        }
        map
    }
}

pub struct JavaEnumSet<E> {
    map: JavaEnumMap<E, ()>,
}

impl<E> Default for JavaEnumSet<E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<E> JavaEnumSet<E> {
    pub fn new() -> Self {
        Self {
            map: JavaEnumMap::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &E> {
        self.map.keys()
    }
}

impl<E: JavaEnum> JavaEnumSet<E> {
    pub fn add(&mut self, element: E) -> bool {
        self.map.put(element, ()).is_none()
    }

    pub fn contains(&self, element: &E) -> bool {
        self.map.contains_key(element)
    }

    pub fn remove(&mut self, element: &E) -> bool {
        self.map.remove(element).is_some()
    }
}

impl<E: fmt::Debug> fmt::Debug for JavaEnumSet<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_set().entries(self.iter()).finish()
    }
}

impl<E: JavaEnum> FromIterator<E> for JavaEnumSet<E> {
    fn from_iter<I: IntoIterator<Item = E>>(elements: I) -> Self {
        let mut set = Self::new();
        for element in elements {
            set.add(element);
        }
        set
    }
}
