use std::borrow::Borrow;
use std::cmp::Ordering;
use std::fmt;

use crate::hash::{JavaHash, spread};

const DEFAULT_INITIAL_CAPACITY: i32 = 16;
const MAXIMUM_CAPACITY: i32 = 1 << 30;
const DEFAULT_LOAD_FACTOR: f32 = 0.75;
const TREEIFY_THRESHOLD: i32 = 8;
const UNTREEIFY_THRESHOLD: i32 = 6;
const MIN_TREEIFY_CAPACITY: usize = 64;

type Link = Option<usize>;

#[derive(Debug, Clone, Copy, Default)]
struct TreeLinks {
    parent: Link,
    left: Link,
    right: Link,
    prev: Link,
    red: bool,
}

#[derive(Debug)]
struct Node<K, V> {
    hash: i32,
    key: K,
    value: V,
    next: Link,
    tree: Option<TreeLinks>,
}

pub struct JavaHashMap<K, V> {
    nodes: Vec<Option<Node<K, V>>>,
    free_slots: Vec<usize>,
    table: Vec<Link>,
    size: i32,
    threshold: i32,
    load_factor: f32,
}

pub fn table_size_for(capacity: i32) -> i32 {
    let n = if capacity <= 1 {
        -1
    } else {
        ((-1i32) as u32 >> (capacity - 1).leading_zeros()) as i32
    };
    if n < 0 {
        1
    } else if n >= MAXIMUM_CAPACITY {
        MAXIMUM_CAPACITY
    } else {
        n + 1
    }
}

pub fn jdk8_copy_constructor_capacity(source_size: usize, load_factor: f32) -> i32 {
    let ft = source_size as f32 / load_factor + 1.0;
    if ft < MAXIMUM_CAPACITY as f32 {
        ft as i32
    } else {
        MAXIMUM_CAPACITY
    }
}

fn java_hash_of<Q: JavaHash + ?Sized>(key: &Q) -> i32 {
    spread(key.java_hash_code())
}

fn compare_direction<K: JavaHash>(key: &K, other: &K) -> Option<i32> {
    match key.java_compare_to(other) {
        Some(Ordering::Less) => Some(-1),
        Some(Ordering::Greater) => Some(1),
        _ => None,
    }
}

fn tie_break_order() -> i32 {
    -1
}

impl<K, V> Default for JavaHashMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K, V> JavaHashMap<K, V> {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            free_slots: Vec::new(),
            table: Vec::new(),
            size: 0,
            threshold: 0,
            load_factor: DEFAULT_LOAD_FACTOR,
        }
    }

    pub fn with_capacity(initial_capacity: i32) -> Self {
        Self::with_capacity_and_load_factor(initial_capacity, DEFAULT_LOAD_FACTOR)
    }

    pub fn with_capacity_and_load_factor(initial_capacity: i32, load_factor: f32) -> Self {
        assert!(
            initial_capacity >= 0,
            "Illegal initial capacity: {initial_capacity}"
        );
        assert!(
            load_factor > 0.0 && !load_factor.is_nan(),
            "Illegal load factor: {load_factor}"
        );
        let capacity = initial_capacity.min(MAXIMUM_CAPACITY);
        Self {
            threshold: table_size_for(capacity),
            load_factor,
            ..Self::new()
        }
    }

    pub fn len(&self) -> usize {
        self.size as usize
    }

    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    pub fn capacity(&self) -> usize {
        self.table.len()
    }

    pub fn threshold(&self) -> i32 {
        self.threshold
    }

    pub fn clear(&mut self) {
        if !self.table.is_empty() && self.size > 0 {
            self.size = 0;
            self.table.iter_mut().for_each(|bucket| *bucket = None);
            self.nodes.clear();
            self.free_slots.clear();
        }
    }

    pub fn iter(&self) -> Iter<'_, K, V> {
        Iter {
            map: self,
            bucket: 0,
            current: None,
            remaining: self.len(),
        }
    }

    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.iter().map(|(key, _)| key)
    }

    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.iter().map(|(_, value)| value)
    }

    pub fn for_each_mut(&mut self, mut action: impl FnMut(&K, &mut V)) {
        for index in self.iteration_order() {
            let node = self.node_mut(index);
            action(&node.key, &mut node.value);
        }
    }

    fn iteration_order(&self) -> Vec<usize> {
        let mut order = Vec::with_capacity(self.len());
        for head in &self.table {
            let mut link = *head;
            while let Some(index) = link {
                order.push(index);
                link = self.node(index).next;
            }
        }
        order
    }

    fn node(&self, index: usize) -> &Node<K, V> {
        self.nodes[index].as_ref().expect("live node")
    }

    fn node_mut(&mut self, index: usize) -> &mut Node<K, V> {
        self.nodes[index].as_mut().expect("live node")
    }

    fn allocate(&mut self, node: Node<K, V>) -> usize {
        match self.free_slots.pop() {
            Some(slot) => {
                self.nodes[slot] = Some(node);
                slot
            }
            None => {
                self.nodes.push(Some(node));
                self.nodes.len() - 1
            }
        }
    }

    fn release(&mut self, index: usize) -> Node<K, V> {
        self.free_slots.push(index);
        self.nodes[index].take().expect("live node")
    }

    fn new_plain_node(&mut self, hash: i32, key: K, value: V, next: Link) -> usize {
        self.allocate(Node {
            hash,
            key,
            value,
            next,
            tree: None,
        })
    }

    fn bucket_index(&self, hash: i32) -> usize {
        (hash as u32 as usize) & (self.table.len() - 1)
    }

    fn is_tree(&self, index: usize) -> bool {
        self.node(index).tree.is_some()
    }

    fn links(&self, index: usize) -> &TreeLinks {
        self.node(index).tree.as_ref().expect("tree node")
    }

    fn links_mut(&mut self, index: usize) -> &mut TreeLinks {
        self.node_mut(index).tree.as_mut().expect("tree node")
    }

    fn parent(&self, index: usize) -> Link {
        self.links(index).parent
    }

    fn left(&self, index: usize) -> Link {
        self.links(index).left
    }

    fn right(&self, index: usize) -> Link {
        self.links(index).right
    }

    fn prev(&self, index: usize) -> Link {
        self.links(index).prev
    }

    fn is_red(&self, link: Link) -> bool {
        link.is_some_and(|index| self.links(index).red)
    }

    fn set_parent(&mut self, index: usize, parent: Link) {
        self.links_mut(index).parent = parent;
    }

    fn set_left(&mut self, index: usize, left: Link) {
        self.links_mut(index).left = left;
    }

    fn set_right(&mut self, index: usize, right: Link) {
        self.links_mut(index).right = right;
    }

    fn set_prev(&mut self, index: usize, prev: Link) {
        self.links_mut(index).prev = prev;
    }

    fn set_red(&mut self, index: usize, red: bool) {
        self.links_mut(index).red = red;
    }

    fn set_next(&mut self, index: usize, next: Link) {
        self.node_mut(index).next = next;
    }
}

impl<K: JavaHash + Eq, V> JavaHashMap<K, V> {
    fn resize(&mut self) {
        let old_cap = self.table.len() as i32;
        let old_thr = self.threshold;
        let new_cap;
        let mut new_thr = 0;
        if old_cap > 0 {
            if old_cap >= MAXIMUM_CAPACITY {
                self.threshold = i32::MAX;
                return;
            }
            new_cap = old_cap << 1;
            if new_cap < MAXIMUM_CAPACITY && old_cap >= DEFAULT_INITIAL_CAPACITY {
                new_thr = old_thr << 1;
            }
        } else if old_thr > 0 {
            new_cap = old_thr;
        } else {
            new_cap = DEFAULT_INITIAL_CAPACITY;
            new_thr = (DEFAULT_LOAD_FACTOR * DEFAULT_INITIAL_CAPACITY as f32) as i32;
        }
        if new_thr == 0 {
            let ft = new_cap as f32 * self.load_factor;
            new_thr = if new_cap < MAXIMUM_CAPACITY && ft < MAXIMUM_CAPACITY as f32 {
                ft as i32
            } else {
                i32::MAX
            };
        }
        self.threshold = new_thr;
        let old_table = std::mem::replace(&mut self.table, vec![None; new_cap as usize]);
        for (j, head) in old_table.into_iter().enumerate() {
            let Some(first) = head else { continue };
            if self.node(first).next.is_none() {
                let index = self.bucket_index(self.node(first).hash);
                self.table[index] = Some(first);
            } else if self.is_tree(first) {
                self.split_tree_bin(first, j, old_cap as usize);
            } else {
                self.split_list_bin(first, j, old_cap as usize);
            }
        }
    }

    fn split_list_bin(&mut self, first: usize, index: usize, bit: usize) {
        let (mut lo_head, mut lo_tail, mut hi_head, mut hi_tail): (Link, Link, Link, Link) =
            (None, None, None, None);
        let mut link = Some(first);
        while let Some(e) = link {
            link = self.node(e).next;
            let (head, tail) = if (self.node(e).hash as u32 as usize) & bit == 0 {
                (&mut lo_head, &mut lo_tail)
            } else {
                (&mut hi_head, &mut hi_tail)
            };
            match *tail {
                None => *head = Some(e),
                Some(t) => self.nodes[t].as_mut().expect("live node").next = Some(e),
            }
            *tail = Some(e);
        }
        if let Some(t) = lo_tail {
            self.set_next(t, None);
            self.table[index] = lo_head;
        }
        if let Some(t) = hi_tail {
            self.set_next(t, None);
            self.table[index + bit] = hi_head;
        }
    }

    fn split_tree_bin(&mut self, first: usize, index: usize, bit: usize) {
        let (mut lo_head, mut lo_tail, mut hi_head, mut hi_tail): (Link, Link, Link, Link) =
            (None, None, None, None);
        let (mut lo_count, mut hi_count) = (0, 0);
        let mut link = Some(first);
        while let Some(e) = link {
            link = self.node(e).next;
            self.set_next(e, None);
            let is_low = (self.node(e).hash as u32 as usize) & bit == 0;
            let (head, tail, count) = if is_low {
                (&mut lo_head, &mut lo_tail, &mut lo_count)
            } else {
                (&mut hi_head, &mut hi_tail, &mut hi_count)
            };
            self.nodes[e]
                .as_mut()
                .expect("live node")
                .tree
                .as_mut()
                .expect("tree node")
                .prev = *tail;
            match *tail {
                None => *head = Some(e),
                Some(t) => self.nodes[t].as_mut().expect("live node").next = Some(e),
            }
            *tail = Some(e);
            *count += 1;
        }
        if let Some(lo) = lo_head {
            if lo_count <= UNTREEIFY_THRESHOLD {
                self.table[index] = Some(self.untreeify(lo));
            } else {
                self.table[index] = Some(lo);
                if hi_head.is_some() {
                    self.treeify(lo);
                }
            }
        }
        if let Some(hi) = hi_head {
            if hi_count <= UNTREEIFY_THRESHOLD {
                self.table[index + bit] = Some(self.untreeify(hi));
            } else {
                self.table[index + bit] = Some(hi);
                if lo_head.is_some() {
                    self.treeify(hi);
                }
            }
        }
    }

    fn untreeify(&mut self, head: usize) -> usize {
        let mut link = Some(head);
        while let Some(index) = link {
            self.node_mut(index).tree = None;
            link = self.node(index).next;
        }
        head
    }

    fn root_of(&self, start: usize) -> usize {
        let mut r = start;
        while let Some(p) = self.parent(r) {
            r = p;
        }
        r
    }

    fn move_root_to_front(&mut self, root: usize) {
        if self.table.is_empty() {
            return;
        }
        let index = self.bucket_index(self.node(root).hash);
        let first = self.table[index];
        if first == Some(root) {
            return;
        }
        self.table[index] = Some(root);
        let rp = self.prev(root);
        let rn = self.node(root).next;
        if let Some(rn) = rn {
            self.set_prev(rn, rp);
        }
        if let Some(rp) = rp {
            self.set_next(rp, rn);
        }
        if let Some(first) = first {
            self.set_prev(first, Some(root));
        }
        self.set_next(root, first);
        self.set_prev(root, None);
    }

    fn rotate_left(&mut self, mut root: usize, p: usize) -> usize {
        let Some(r) = self.right(p) else { return root };
        let rl = self.left(r);
        self.set_right(p, rl);
        if let Some(rl) = rl {
            self.set_parent(rl, Some(p));
        }
        let pp = self.parent(p);
        self.set_parent(r, pp);
        match pp {
            None => {
                root = r;
                self.set_red(r, false);
            }
            Some(pp) if self.left(pp) == Some(p) => self.set_left(pp, Some(r)),
            Some(pp) => self.set_right(pp, Some(r)),
        }
        self.set_left(r, Some(p));
        self.set_parent(p, Some(r));
        root
    }

    fn rotate_right(&mut self, mut root: usize, p: usize) -> usize {
        let Some(l) = self.left(p) else { return root };
        let lr = self.right(l);
        self.set_left(p, lr);
        if let Some(lr) = lr {
            self.set_parent(lr, Some(p));
        }
        let pp = self.parent(p);
        self.set_parent(l, pp);
        match pp {
            None => {
                root = l;
                self.set_red(l, false);
            }
            Some(pp) if self.right(pp) == Some(p) => self.set_right(pp, Some(l)),
            Some(pp) => self.set_left(pp, Some(l)),
        }
        self.set_right(l, Some(p));
        self.set_parent(p, Some(l));
        root
    }

    fn balance_insertion(&mut self, mut root: usize, mut x: usize) -> usize {
        self.set_red(x, true);
        loop {
            let Some(mut xp) = self.parent(x) else {
                self.set_red(x, false);
                return x;
            };
            if !self.is_red(Some(xp)) {
                return root;
            }
            let Some(mut xpp) = self.parent(xp) else {
                return root;
            };
            let xppl = self.left(xpp);
            if Some(xp) == xppl {
                let xppr = self.right(xpp);
                if let Some(uncle) = xppr.filter(|u| self.links(*u).red) {
                    self.set_red(uncle, false);
                    self.set_red(xp, false);
                    self.set_red(xpp, true);
                    x = xpp;
                } else {
                    let mut grandparent = Some(xpp);
                    let mut parent = Some(xp);
                    if Some(x) == self.right(xp) {
                        x = xp;
                        root = self.rotate_left(root, x);
                        parent = self.parent(x);
                        grandparent = parent.and_then(|p| self.parent(p));
                    }
                    if let Some(p) = parent {
                        xp = p;
                        self.set_red(xp, false);
                        if let Some(g) = grandparent {
                            xpp = g;
                            self.set_red(xpp, true);
                            root = self.rotate_right(root, xpp);
                        }
                    }
                }
            } else if let Some(uncle) = xppl.filter(|u| self.links(*u).red) {
                self.set_red(uncle, false);
                self.set_red(xp, false);
                self.set_red(xpp, true);
                x = xpp;
            } else {
                let mut grandparent = Some(xpp);
                let mut parent = Some(xp);
                if Some(x) == self.left(xp) {
                    x = xp;
                    root = self.rotate_right(root, x);
                    parent = self.parent(x);
                    grandparent = parent.and_then(|p| self.parent(p));
                }
                if let Some(p) = parent {
                    xp = p;
                    self.set_red(xp, false);
                    if let Some(g) = grandparent {
                        xpp = g;
                        self.set_red(xpp, true);
                        root = self.rotate_left(root, xpp);
                    }
                }
            }
        }
    }

    fn balance_deletion(&mut self, mut root: usize, start: Link) -> usize {
        let mut x = start;
        loop {
            let Some(xi) = x else { return root };
            if xi == root {
                return root;
            }
            let Some(parent) = self.parent(xi) else {
                self.set_red(xi, false);
                return xi;
            };
            if self.links(xi).red {
                self.set_red(xi, false);
                return root;
            }
            let (next_x, next_root) = if self.left(parent) == Some(xi) {
                self.rebalance_left_deficit(root, xi, parent)
            } else {
                self.rebalance_right_deficit(root, xi, parent)
            };
            x = next_x;
            root = next_root;
        }
    }

    fn rebalance_left_deficit(
        &mut self,
        mut root: usize,
        x: usize,
        parent: usize,
    ) -> (Link, usize) {
        let mut xp = Some(parent);
        let mut xpr = self.right(parent);
        if let Some(sibling) = xpr.filter(|s| self.links(*s).red) {
            self.set_red(sibling, false);
            self.set_red(parent, true);
            root = self.rotate_left(root, parent);
            xp = self.parent(x);
            xpr = xp.and_then(|p| self.right(p));
        }
        let Some(sibling) = xpr else {
            return (xp, root);
        };
        let sl = self.left(sibling);
        let sr = self.right(sibling);
        if !self.is_red(sr) && !self.is_red(sl) {
            self.set_red(sibling, true);
            return (xp, root);
        }
        if !self.is_red(sr) {
            if let Some(sl) = sl {
                self.set_red(sl, false);
            }
            self.set_red(sibling, true);
            root = self.rotate_right(root, sibling);
            xp = self.parent(x);
            xpr = xp.and_then(|p| self.right(p));
        }
        if let Some(sibling) = xpr {
            let parent_red = self.is_red(xp);
            self.set_red(sibling, parent_red);
            if let Some(far) = self.right(sibling) {
                self.set_red(far, false);
            }
        }
        if let Some(p) = xp {
            self.set_red(p, false);
            root = self.rotate_left(root, p);
        }
        (Some(root), root)
    }

    fn rebalance_right_deficit(
        &mut self,
        mut root: usize,
        x: usize,
        parent: usize,
    ) -> (Link, usize) {
        let mut xp = Some(parent);
        let mut xpl = self.left(parent);
        if let Some(sibling) = xpl.filter(|s| self.links(*s).red) {
            self.set_red(sibling, false);
            self.set_red(parent, true);
            root = self.rotate_right(root, parent);
            xp = self.parent(x);
            xpl = xp.and_then(|p| self.left(p));
        }
        let Some(sibling) = xpl else {
            return (xp, root);
        };
        let sl = self.left(sibling);
        let sr = self.right(sibling);
        if !self.is_red(sl) && !self.is_red(sr) {
            self.set_red(sibling, true);
            return (xp, root);
        }
        if !self.is_red(sl) {
            if let Some(sr) = sr {
                self.set_red(sr, false);
            }
            self.set_red(sibling, true);
            root = self.rotate_left(root, sibling);
            xp = self.parent(x);
            xpl = xp.and_then(|p| self.left(p));
        }
        if let Some(sibling) = xpl {
            let parent_red = self.is_red(xp);
            self.set_red(sibling, parent_red);
            if let Some(far) = self.left(sibling) {
                self.set_red(far, false);
            }
        }
        if let Some(p) = xp {
            self.set_red(p, false);
            root = self.rotate_right(root, p);
        }
        (Some(root), root)
    }
}

impl<K: JavaHash + Eq, V> JavaHashMap<K, V> {
    pub fn from_entries_like_copy_constructor<I>(entries: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        I::IntoIter: ExactSizeIterator,
    {
        let mut map = Self::new();
        map.put_all(entries);
        map
    }

    pub fn java_clone(&self) -> Self
    where
        K: Clone,
        V: Clone,
    {
        let entries: Vec<(K, V)> = self
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect();
        Self::from_entries_like_copy_constructor(entries)
    }

    pub fn put_all<I>(&mut self, entries: I)
    where
        I: IntoIterator<Item = (K, V)>,
        I::IntoIter: ExactSizeIterator,
    {
        let entries = entries.into_iter();
        self.presize_for(entries.len());
        for (key, value) in entries {
            self.put(key, value);
        }
    }

    fn presize_for(&mut self, incoming: usize) {
        if incoming == 0 {
            return;
        }
        if self.table.is_empty() {
            let t = jdk8_copy_constructor_capacity(incoming, self.load_factor);
            if t > self.threshold {
                self.threshold = table_size_for(t);
            }
        } else if incoming > self.threshold as usize {
            self.resize();
        }
    }

    fn find_node<Q>(&self, key: &Q) -> Option<usize>
    where
        K: Borrow<Q>,
        Q: JavaHash + Eq + ?Sized,
    {
        if self.table.is_empty() {
            return None;
        }
        let hash = java_hash_of(key);
        self.find_in_bucket(self.bucket_index(hash), hash, key)
            .map(|(index, _)| index)
    }

    fn find_in_bucket<Q>(&self, bucket: usize, hash: i32, key: &Q) -> Option<(usize, Link)>
    where
        K: Borrow<Q>,
        Q: Eq + ?Sized,
    {
        let mut previous = None;
        let mut link = self.table[bucket];
        while let Some(index) = link {
            let node = self.node(index);
            if node.hash == hash && node.key.borrow() == key {
                return Some((index, previous));
            }
            previous = Some(index);
            link = node.next;
        }
        None
    }

    fn bucket_length(&self, bucket: usize) -> i32 {
        let mut length = 0;
        let mut link = self.table[bucket];
        while let Some(index) = link {
            length += 1;
            link = self.node(index).next;
        }
        length
    }

    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: JavaHash + Eq + ?Sized,
    {
        self.find_node(key).map(|index| &self.node(index).value)
    }

    pub fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
        Q: JavaHash + Eq + ?Sized,
    {
        self.find_node(key)
            .map(|index| &mut self.node_mut(index).value)
    }

    pub fn get_or_default<'a, Q>(&'a self, key: &Q, default: &'a V) -> &'a V
    where
        K: Borrow<Q>,
        Q: JavaHash + Eq + ?Sized,
    {
        self.get(key).unwrap_or(default)
    }

    pub fn get_key_value<Q>(&self, key: &Q) -> Option<(&K, &V)>
    where
        K: Borrow<Q>,
        Q: JavaHash + Eq + ?Sized,
    {
        self.find_node(key).map(|index| {
            let node = self.node(index);
            (&node.key, &node.value)
        })
    }

    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: JavaHash + Eq + ?Sized,
    {
        self.find_node(key).is_some()
    }

    pub fn put(&mut self, key: K, value: V) -> Option<V> {
        let hash = java_hash_of(&key);
        let bucket = self.ensure_bucket_for(hash);
        if let Some((existing, _)) = self.find_in_bucket(bucket, hash, &key) {
            return Some(std::mem::replace(&mut self.node_mut(existing).value, value));
        }
        self.insert_new_at_tail(bucket, hash, key, value);
        self.size += 1;
        if self.size > self.threshold {
            self.resize();
        }
        None
    }

    pub fn put_if_absent(&mut self, key: K, value: V) -> bool {
        if self.contains_key(&key) {
            return false;
        }
        self.put(key, value);
        true
    }

    fn ensure_bucket_for(&mut self, hash: i32) -> usize {
        if self.table.is_empty() {
            self.resize();
        }
        self.bucket_index(hash)
    }

    fn insert_new_at_tail(&mut self, bucket: usize, hash: i32, key: K, value: V) -> usize {
        match self.table[bucket] {
            None => {
                let index = self.new_plain_node(hash, key, value, None);
                self.table[bucket] = Some(index);
                index
            }
            Some(first) if self.is_tree(first) => self.put_tree_val(first, hash, key, value),
            Some(first) => {
                let bin_count = self.bucket_length(bucket) - 1;
                let mut tail = first;
                while let Some(next) = self.node(tail).next {
                    tail = next;
                }
                let index = self.new_plain_node(hash, key, value, None);
                self.set_next(tail, Some(index));
                if bin_count >= TREEIFY_THRESHOLD - 1 {
                    self.treeify_bin(hash);
                }
                index
            }
        }
    }

    fn insert_new_at_head(&mut self, bucket: usize, hash: i32, key: K, value: V) -> usize {
        match self.table[bucket] {
            Some(first) if self.is_tree(first) => self.put_tree_val(first, hash, key, value),
            first => {
                let bin_count = self.bucket_length(bucket);
                let index = self.new_plain_node(hash, key, value, first);
                self.table[bucket] = Some(index);
                if bin_count >= TREEIFY_THRESHOLD - 1 {
                    self.treeify_bin(hash);
                }
                index
            }
        }
    }

    fn presize_before_compute(&mut self, hash: i32) -> usize {
        if self.size > self.threshold || self.table.is_empty() {
            self.resize();
        }
        self.bucket_index(hash)
    }

    pub fn compute_if_absent(&mut self, key: K, mapping: impl FnOnce(&K) -> V) -> &mut V {
        let hash = java_hash_of(&key);
        let bucket = self.presize_before_compute(hash);
        if let Some((existing, _)) = self.find_in_bucket(bucket, hash, &key) {
            return &mut self.node_mut(existing).value;
        }
        let value = mapping(&key);
        let index = self.insert_new_at_head(bucket, hash, key, value);
        self.size += 1;
        &mut self.node_mut(index).value
    }

    pub fn compute(
        &mut self,
        key: K,
        remapping: impl FnOnce(&K, Option<&V>) -> Option<V>,
    ) -> Option<&mut V> {
        let hash = java_hash_of(&key);
        let bucket = self.presize_before_compute(hash);
        match self.find_in_bucket(bucket, hash, &key) {
            Some((existing, _)) => match remapping(&key, Some(&self.node(existing).value)) {
                Some(value) => {
                    self.node_mut(existing).value = value;
                    Some(&mut self.node_mut(existing).value)
                }
                None => {
                    self.remove_node(hash, &key, true);
                    None
                }
            },
            None => {
                let value = remapping(&key, None)?;
                let index = self.insert_new_at_head(bucket, hash, key, value);
                self.size += 1;
                Some(&mut self.node_mut(index).value)
            }
        }
    }

    pub fn merge(
        &mut self,
        key: K,
        value: V,
        remapping: impl FnOnce(&V, V) -> Option<V>,
    ) -> Option<&mut V> {
        let hash = java_hash_of(&key);
        let bucket = self.presize_before_compute(hash);
        match self.find_in_bucket(bucket, hash, &key) {
            Some((existing, _)) => match remapping(&self.node(existing).value, value) {
                Some(merged) => {
                    self.node_mut(existing).value = merged;
                    Some(&mut self.node_mut(existing).value)
                }
                None => {
                    self.remove_node(hash, &key, true);
                    None
                }
            },
            None => {
                let index = self.insert_new_at_head(bucket, hash, key, value);
                self.size += 1;
                Some(&mut self.node_mut(index).value)
            }
        }
    }

    pub fn remove<Q>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: JavaHash + Eq + ?Sized,
    {
        self.remove_entry(key).map(|(_, value)| value)
    }

    pub fn remove_entry<Q>(&mut self, key: &Q) -> Option<(K, V)>
    where
        K: Borrow<Q>,
        Q: JavaHash + Eq + ?Sized,
    {
        self.remove_node(java_hash_of(key), key, true)
    }

    pub fn remove_via_iterator<Q>(&mut self, key: &Q) -> Option<(K, V)>
    where
        K: Borrow<Q>,
        Q: JavaHash + Eq + ?Sized,
    {
        self.remove_node(java_hash_of(key), key, false)
    }

    pub fn retain(&mut self, mut keep: impl FnMut(&K, &mut V) -> bool) {
        for index in self.iteration_order() {
            let node = self.node_mut(index);
            if !keep(&node.key, &mut node.value) {
                let hash = node.hash;
                self.unlink_node(index, hash, false);
            }
        }
    }

    fn remove_node<Q>(&mut self, hash: i32, key: &Q, movable: bool) -> Option<(K, V)>
    where
        K: Borrow<Q>,
        Q: Eq + ?Sized,
    {
        if self.table.is_empty() {
            return None;
        }
        let (index, _) = self.find_in_bucket(self.bucket_index(hash), hash, key)?;
        Some(self.unlink_node(index, hash, movable))
    }

    fn unlink_node(&mut self, index: usize, hash: i32, movable: bool) -> (K, V) {
        let bucket = self.bucket_index(hash);
        if self.is_tree(index) {
            self.remove_tree_node(index, movable);
        } else {
            let previous = self.find_predecessor(bucket, index);
            let next = self.node(index).next;
            match previous {
                None => self.table[bucket] = next,
                Some(p) => self.set_next(p, next),
            }
        }
        self.size -= 1;
        let node = self.release(index);
        (node.key, node.value)
    }

    fn find_predecessor(&self, bucket: usize, target: usize) -> Link {
        let mut previous = None;
        let mut link = self.table[bucket];
        while let Some(index) = link {
            if index == target {
                return previous;
            }
            previous = Some(index);
            link = self.node(index).next;
        }
        None
    }

    fn treeify_bin(&mut self, hash: i32) {
        if self.table.len() < MIN_TREEIFY_CAPACITY {
            self.resize();
            return;
        }
        let bucket = self.bucket_index(hash);
        let Some(head) = self.table[bucket] else {
            return;
        };
        let mut previous = None;
        let mut link = Some(head);
        while let Some(index) = link {
            self.node_mut(index).tree = Some(TreeLinks {
                prev: previous,
                ..TreeLinks::default()
            });
            previous = Some(index);
            link = self.node(index).next;
        }
        self.treeify(head);
    }

    fn tree_direction(&self, hash: i32, key: &K, against: usize) -> Option<i32> {
        let other = self.node(against);
        if other.hash > hash {
            Some(-1)
        } else if other.hash < hash {
            Some(1)
        } else {
            compare_direction(key, &other.key)
        }
    }

    fn treeify(&mut self, head: usize) {
        let mut root: Link = None;
        let mut link = Some(head);
        while let Some(x) = link {
            link = self.node(x).next;
            self.set_left(x, None);
            self.set_right(x, None);
            match root {
                None => {
                    self.set_parent(x, None);
                    self.set_red(x, false);
                    root = Some(x);
                }
                Some(current_root) => {
                    root = Some(self.attach_during_treeify(current_root, x));
                }
            }
        }
        if let Some(root) = root {
            self.move_root_to_front(root);
        }
    }

    fn attach_during_treeify(&mut self, root: usize, x: usize) -> usize {
        let hash = self.node(x).hash;
        let mut p = root;
        loop {
            let dir = self
                .tree_direction_between(hash, x, p)
                .unwrap_or_else(tie_break_order);
            let child = if dir <= 0 {
                self.left(p)
            } else {
                self.right(p)
            };
            match child {
                Some(next) => p = next,
                None => {
                    self.set_parent(x, Some(p));
                    if dir <= 0 {
                        self.set_left(p, Some(x));
                    } else {
                        self.set_right(p, Some(x));
                    }
                    return self.balance_insertion(root, x);
                }
            }
        }
    }

    fn tree_direction_between(&self, hash: i32, x: usize, p: usize) -> Option<i32> {
        self.tree_direction(hash, &self.node(x).key, p)
    }

    fn tree_find(&self, start: usize, hash: i32, key: &K) -> Option<usize> {
        let mut link = Some(start);
        while let Some(p) = link {
            let node = self.node(p);
            let pl = self.left(p);
            let pr = self.right(p);
            if node.hash > hash {
                link = pl;
            } else if node.hash < hash {
                link = pr;
            } else if node.key == *key {
                return Some(p);
            } else if pl.is_none() {
                link = pr;
            } else if pr.is_none() {
                link = pl;
            } else if let Some(dir) = compare_direction(key, &node.key) {
                link = if dir < 0 { pl } else { pr };
            } else if let Some(found) = pr.and_then(|right| self.tree_find(right, hash, key)) {
                return Some(found);
            } else {
                link = pl;
            }
        }
        None
    }

    fn put_tree_val(&mut self, first: usize, hash: i32, key: K, value: V) -> usize {
        let root = if self.parent(first).is_some() {
            self.root_of(first)
        } else {
            first
        };
        let mut searched = false;
        let mut p = root;
        loop {
            let dir = match self.tree_direction(hash, &key, p) {
                Some(dir) => dir,
                None => {
                    if !searched {
                        searched = true;
                        let in_left = self
                            .left(p)
                            .and_then(|child| self.tree_find(child, hash, &key));
                        let in_right = self
                            .right(p)
                            .and_then(|child| self.tree_find(child, hash, &key));
                        if let Some(found) = in_left.or(in_right) {
                            self.node_mut(found).value = value;
                            return found;
                        }
                    }
                    tie_break_order()
                }
            };
            let child = if dir <= 0 {
                self.left(p)
            } else {
                self.right(p)
            };
            if let Some(next) = child {
                p = next;
                continue;
            }
            let xpn = self.node(p).next;
            let x = self.allocate(Node {
                hash,
                key,
                value,
                next: xpn,
                tree: Some(TreeLinks {
                    parent: Some(p),
                    prev: Some(p),
                    ..TreeLinks::default()
                }),
            });
            if dir <= 0 {
                self.set_left(p, Some(x));
            } else {
                self.set_right(p, Some(x));
            }
            self.set_next(p, Some(x));
            if let Some(xpn) = xpn {
                self.set_prev(xpn, Some(x));
            }
            let balanced_root = self.balance_insertion(root, x);
            self.move_root_to_front(balanced_root);
            return x;
        }
    }

    fn remove_tree_node(&mut self, this: usize, movable: bool) {
        if self.table.is_empty() {
            return;
        }
        let index = self.bucket_index(self.node(this).hash);
        let original_first = self.table[index].expect("bin containing node");
        let mut root = original_first;
        let succ = self.node(this).next;
        let pred = self.prev(this);
        let first = match pred {
            None => {
                self.table[index] = succ;
                succ
            }
            Some(pred) => {
                self.set_next(pred, succ);
                Some(original_first)
            }
        };
        if let Some(succ) = succ {
            self.set_prev(succ, pred);
        }
        let Some(first) = first else { return };
        if self.parent(root).is_some() {
            root = self.root_of(root);
        }
        if self.tree_too_small(root) {
            self.table[index] = Some(self.untreeify(first));
            return;
        }
        let replacement = self.detach_from_tree(this, &mut root);
        let r = if self.links(this).red {
            root
        } else {
            self.balance_deletion(root, Some(replacement))
        };
        if replacement == this {
            self.detach_leaf(this);
        }
        if movable {
            self.move_root_to_front(r);
        }
    }

    fn tree_too_small(&self, root: usize) -> bool {
        self.right(root).is_none()
            || match self.left(root) {
                None => true,
                Some(rl) => self.left(rl).is_none(),
            }
    }

    fn detach_from_tree(&mut self, p: usize, root: &mut usize) -> usize {
        let pl = self.left(p);
        let pr = self.right(p);
        let replacement = match (pl, pr) {
            (Some(pl), Some(pr)) => self.swap_with_successor(p, pl, pr, root),
            (Some(pl), None) => pl,
            (None, Some(pr)) => pr,
            (None, None) => p,
        };
        if replacement != p {
            let pp = self.parent(p);
            self.set_parent(replacement, pp);
            match pp {
                None => {
                    *root = replacement;
                    self.set_red(replacement, false);
                }
                Some(pp) if self.left(pp) == Some(p) => self.set_left(pp, Some(replacement)),
                Some(pp) => self.set_right(pp, Some(replacement)),
            }
            self.set_left(p, None);
            self.set_right(p, None);
            self.set_parent(p, None);
        }
        replacement
    }

    fn swap_with_successor(&mut self, p: usize, pl: usize, pr: usize, root: &mut usize) -> usize {
        let mut s = pr;
        while let Some(sl) = self.left(s) {
            s = sl;
        }
        let s_red = self.links(s).red;
        let p_red = self.links(p).red;
        self.set_red(s, p_red);
        self.set_red(p, s_red);
        let sr = self.right(s);
        let pp = self.parent(p);
        if s == pr {
            self.set_parent(p, Some(s));
            self.set_right(s, Some(p));
        } else {
            let sp = self.parent(s);
            self.set_parent(p, sp);
            if let Some(sp) = sp {
                if self.left(sp) == Some(s) {
                    self.set_left(sp, Some(p));
                } else {
                    self.set_right(sp, Some(p));
                }
            }
            self.set_right(s, Some(pr));
            self.set_parent(pr, Some(s));
        }
        self.set_left(p, None);
        self.set_right(p, sr);
        if let Some(sr) = sr {
            self.set_parent(sr, Some(p));
        }
        self.set_left(s, Some(pl));
        self.set_parent(pl, Some(s));
        self.set_parent(s, pp);
        match pp {
            None => *root = s,
            Some(pp) if self.left(pp) == Some(p) => self.set_left(pp, Some(s)),
            Some(pp) => self.set_right(pp, Some(s)),
        }
        sr.unwrap_or(p)
    }

    fn detach_leaf(&mut self, p: usize) {
        let pp = self.parent(p);
        self.set_parent(p, None);
        if let Some(pp) = pp {
            if self.left(pp) == Some(p) {
                self.set_left(pp, None);
            } else if self.right(pp) == Some(p) {
                self.set_right(pp, None);
            }
        }
    }
}

pub struct Iter<'a, K, V> {
    map: &'a JavaHashMap<K, V>,
    bucket: usize,
    current: Link,
    remaining: usize,
}

impl<'a, K, V> Iterator for Iter<'a, K, V> {
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        while self.current.is_none() {
            if self.bucket >= self.map.table.len() {
                return None;
            }
            self.current = self.map.table[self.bucket];
            self.bucket += 1;
        }
        let index = self.current?;
        let node = self.map.node(index);
        self.current = node.next;
        self.remaining -= 1;
        Some((&node.key, &node.value))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl<K, V> ExactSizeIterator for Iter<'_, K, V> {}

impl<'a, K, V> IntoIterator for &'a JavaHashMap<K, V> {
    type Item = (&'a K, &'a V);
    type IntoIter = Iter<'a, K, V>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<K: fmt::Debug, V: fmt::Debug> fmt::Debug for JavaHashMap<K, V> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_map().entries(self.iter()).finish()
    }
}

impl<K: JavaHash + Eq, V> FromIterator<(K, V)> for JavaHashMap<K, V> {
    fn from_iter<I: IntoIterator<Item = (K, V)>>(entries: I) -> Self {
        let mut map = Self::new();
        for (key, value) in entries {
            map.put(key, value);
        }
        map
    }
}

impl<K: JavaHash + Eq, V> Extend<(K, V)> for JavaHashMap<K, V> {
    fn extend<I: IntoIterator<Item = (K, V)>>(&mut self, entries: I) {
        for (key, value) in entries {
            self.put(key, value);
        }
    }
}
