use randomon_java::JavaRng;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ItemList {
    items: Vec<bool>,
    tms: Vec<bool>,
}

impl ItemList {
    pub fn new(highest_index: usize) -> Self {
        let mut items = vec![false; highest_index + 1];
        for allowed in items.iter_mut().skip(1) {
            *allowed = true;
        }
        Self {
            items,
            tms: vec![false; highest_index + 1],
        }
    }

    fn in_range(&self, index: i32) -> Option<usize> {
        usize::try_from(index)
            .ok()
            .filter(|&index| index < self.tms.len())
    }

    pub fn is_tm(&self, index: i32) -> bool {
        self.in_range(index).is_some_and(|index| self.tms[index])
    }

    pub fn is_allowed(&self, index: i32) -> bool {
        self.in_range(index).is_some_and(|index| self.items[index])
    }

    pub fn ban_singles(&mut self, indexes: &[i32]) {
        for &index in indexes {
            self.items[index as usize] = false;
        }
    }

    pub fn ban_range(&mut self, start: i32, length: i32) {
        for offset in 0..length {
            self.items[(start + offset) as usize] = false;
        }
    }

    pub fn tm_range(&mut self, start: i32, length: i32) {
        for offset in 0..length {
            self.tms[(start + offset) as usize] = true;
        }
    }

    fn bound(&self) -> i32 {
        self.items.len() as i32
    }

    pub fn random_item(&self, random: &mut impl JavaRng) -> i32 {
        let mut chosen = 0usize;
        while !self.items[chosen] {
            chosen = random.next_int_bounded(self.bound()) as usize;
        }
        chosen as i32
    }

    pub fn random_non_tm(&self, random: &mut impl JavaRng) -> i32 {
        let mut chosen = 0usize;
        while !self.items[chosen] || self.tms[chosen] {
            chosen = random.next_int_bounded(self.bound()) as usize;
        }
        chosen as i32
    }

    pub fn random_tm(&self, random: &mut impl JavaRng) -> i32 {
        let mut chosen = 0usize;
        while !self.tms[chosen] {
            chosen = random.next_int_bounded(self.bound()) as usize;
        }
        chosen as i32
    }

    pub fn copy_with_max(&self, new_max: usize) -> ItemList {
        let mut other = ItemList::new(new_max);
        other.items[..self.items.len()].copy_from_slice(&self.items);
        other.tms[..self.tms.len()].copy_from_slice(&self.tms);
        other
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Shop {
    pub items: Vec<i32>,
    pub name: String,
    pub is_main_game: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PickupItem {
    pub item: i32,
    pub probabilities: Vec<i32>,
}

impl PickupItem {
    pub fn new(item: i32) -> Self {
        Self {
            item,
            probabilities: vec![0; 10],
        }
    }
}
