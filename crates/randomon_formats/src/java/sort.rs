use std::fmt;

const MIN_MERGE: usize = 32;
const MIN_GALLOP: i64 = 7;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JavaSortError {
    ContractViolation,
    RunStackOverflow,
}

impl fmt::Display for JavaSortError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JavaSortError::ContractViolation => {
                formatter.write_str("Comparison method violates its general contract!")
            }
            JavaSortError::RunStackOverflow => formatter.write_str("TimSort run stack overflow"),
        }
    }
}

impl std::error::Error for JavaSortError {}

pub fn java_list_sort<T, F>(items: &mut [T], compare: F) -> Result<(), JavaSortError>
where
    T: Clone,
    F: FnMut(&T, &T) -> i32,
{
    let mut sorter = TimSort::new(items, compare);
    sorter.sort()
}

struct TimSort<'a, T, F> {
    items: &'a mut [T],
    compare: F,
    min_gallop: i64,
    run_base: Vec<usize>,
    run_len: Vec<usize>,
    stack_capacity: usize,
}

impl<'a, T, F> TimSort<'a, T, F>
where
    T: Clone,
    F: FnMut(&T, &T) -> i32,
{
    fn new(items: &'a mut [T], compare: F) -> Self {
        let len = items.len();
        let stack_capacity = match len {
            0..120 => 5,
            120..1542 => 10,
            1542..119151 => 24,
            _ => 49,
        };
        TimSort {
            items,
            compare,
            min_gallop: MIN_GALLOP,
            run_base: Vec::new(),
            run_len: Vec::new(),
            stack_capacity,
        }
    }

    fn cmp(&mut self, left: &T, right: &T) -> i32 {
        (self.compare)(left, right)
    }

    fn cmp_items(&mut self, left: usize, right: usize) -> i32 {
        let left_value = self.items[left].clone();
        let right_value = self.items[right].clone();
        self.cmp(&left_value, &right_value)
    }

    fn sort(&mut self) -> Result<(), JavaSortError> {
        let hi = self.items.len();
        let mut lo = 0usize;
        let mut remaining = hi;
        if remaining < 2 {
            return Ok(());
        }
        if remaining < MIN_MERGE {
            let initial_run = self.count_run_and_make_ascending(lo, hi);
            self.binary_sort(lo, hi, lo + initial_run);
            return Ok(());
        }
        let min_run = min_run_length(remaining);
        loop {
            let mut run_len = self.count_run_and_make_ascending(lo, hi);
            if run_len < min_run {
                let force = if remaining <= min_run {
                    remaining
                } else {
                    min_run
                };
                self.binary_sort(lo, lo + force, lo + run_len);
                run_len = force;
            }
            self.push_run(lo, run_len)?;
            self.merge_collapse()?;
            lo += run_len;
            remaining -= run_len;
            if remaining == 0 {
                break;
            }
        }
        self.merge_force_collapse()
    }

    fn binary_sort(&mut self, lo: usize, hi: usize, start: usize) {
        let mut start = if start == lo { start + 1 } else { start };
        while start < hi {
            let pivot = self.items[start].clone();
            let mut left = lo;
            let mut right = start;
            while left < right {
                let mid = (left + right) >> 1;
                let mid_value = self.items[mid].clone();
                if self.cmp(&pivot, &mid_value) < 0 {
                    right = mid;
                } else {
                    left = mid + 1;
                }
            }
            self.items[left..=start].rotate_right(1);
            self.items[left] = pivot;
            start += 1;
        }
    }

    fn count_run_and_make_ascending(&mut self, lo: usize, hi: usize) -> usize {
        let mut run_hi = lo + 1;
        if run_hi == hi {
            return 1;
        }
        let first_is_descending = self.cmp_items(run_hi, lo) < 0;
        run_hi += 1;
        if first_is_descending {
            while run_hi < hi && self.cmp_items(run_hi, run_hi - 1) < 0 {
                run_hi += 1;
            }
            self.items[lo..run_hi].reverse();
        } else {
            while run_hi < hi && self.cmp_items(run_hi, run_hi - 1) >= 0 {
                run_hi += 1;
            }
        }
        run_hi - lo
    }

    fn push_run(&mut self, base: usize, len: usize) -> Result<(), JavaSortError> {
        if self.run_base.len() >= self.stack_capacity {
            return Err(JavaSortError::RunStackOverflow);
        }
        self.run_base.push(base);
        self.run_len.push(len);
        Ok(())
    }

    fn merge_collapse(&mut self) -> Result<(), JavaSortError> {
        while self.run_len.len() > 1 {
            let mut n = self.run_len.len() - 2;
            let lens = &self.run_len;
            let first_violated = n > 0 && lens[n - 1] <= lens[n] + lens[n + 1];
            let second_violated = n > 1 && lens[n - 2] <= lens[n] + lens[n - 1];
            if first_violated || second_violated {
                if lens[n - 1] < lens[n + 1] {
                    n -= 1;
                }
            } else if lens[n] > lens[n + 1] {
                break;
            }
            self.merge_at(n)?;
        }
        Ok(())
    }

    fn merge_force_collapse(&mut self) -> Result<(), JavaSortError> {
        while self.run_len.len() > 1 {
            let mut n = self.run_len.len() - 2;
            if n > 0 && self.run_len[n - 1] < self.run_len[n + 1] {
                n -= 1;
            }
            self.merge_at(n)?;
        }
        Ok(())
    }

    fn merge_at(&mut self, i: usize) -> Result<(), JavaSortError> {
        let mut base1 = self.run_base[i];
        let mut len1 = self.run_len[i];
        let base2 = self.run_base[i + 1];
        let mut len2 = self.run_len[i + 1];
        self.run_len[i] = len1 + len2;
        self.run_base.remove(i + 1);
        self.run_len.remove(i + 1);

        let key = self.items[base2].clone();
        let k = self.gallop_right(&key, Source::Items, base1, len1, 0);
        base1 += k;
        len1 -= k;
        if len1 == 0 {
            return Ok(());
        }
        let key = self.items[base1 + len1 - 1].clone();
        len2 = self.gallop_left(&key, Source::Items, base2, len2, len2 - 1);
        if len2 == 0 {
            return Ok(());
        }
        if len1 <= len2 {
            self.merge_lo(base1, len1, base2, len2)
        } else {
            self.merge_hi(base1, len1, base2, len2)
        }
    }

    fn element(&self, source: Source, tmp: &[T], index: usize) -> T {
        match source {
            Source::Items => self.items[index].clone(),
            Source::Tmp => tmp[index].clone(),
        }
    }

    fn gallop_left(
        &mut self,
        key: &T,
        source: Source,
        base: usize,
        len: usize,
        hint: usize,
    ) -> usize {
        self.gallop_left_in(key, source, &[], base, len, hint)
    }

    fn gallop_right(
        &mut self,
        key: &T,
        source: Source,
        base: usize,
        len: usize,
        hint: usize,
    ) -> usize {
        self.gallop_right_in(key, source, &[], base, len, hint)
    }

    fn gallop_left_in(
        &mut self,
        key: &T,
        source: Source,
        tmp: &[T],
        base: usize,
        len: usize,
        hint: usize,
    ) -> usize {
        let (len, hint) = (len as i64, hint as i64);
        let mut last_ofs: i64 = 0;
        let mut ofs: i64 = 1;
        let at = |sorter: &Self, offset: i64| {
            sorter.element(source, tmp, (base as i64 + offset) as usize)
        };
        let hinted = at(self, hint);
        if self.cmp(key, &hinted) > 0 {
            let max_ofs = len - hint;
            while ofs < max_ofs {
                let candidate = at(self, hint + ofs);
                if self.cmp(key, &candidate) <= 0 {
                    break;
                }
                last_ofs = ofs;
                ofs = (ofs << 1) + 1;
                if ofs > i32::MAX as i64 {
                    ofs = max_ofs;
                }
            }
            if ofs > max_ofs {
                ofs = max_ofs;
            }
            last_ofs += hint;
            ofs += hint;
        } else {
            let max_ofs = hint + 1;
            while ofs < max_ofs {
                let candidate = at(self, hint - ofs);
                if self.cmp(key, &candidate) > 0 {
                    break;
                }
                last_ofs = ofs;
                ofs = (ofs << 1) + 1;
                if ofs > i32::MAX as i64 {
                    ofs = max_ofs;
                }
            }
            if ofs > max_ofs {
                ofs = max_ofs;
            }
            let previous = last_ofs;
            last_ofs = hint - ofs;
            ofs = hint - previous;
        }
        last_ofs += 1;
        while last_ofs < ofs {
            let m = last_ofs + ((ofs - last_ofs) >> 1);
            let candidate = at(self, m);
            if self.cmp(key, &candidate) > 0 {
                last_ofs = m + 1;
            } else {
                ofs = m;
            }
        }
        ofs as usize
    }

    fn gallop_right_in(
        &mut self,
        key: &T,
        source: Source,
        tmp: &[T],
        base: usize,
        len: usize,
        hint: usize,
    ) -> usize {
        let (len, hint) = (len as i64, hint as i64);
        let mut last_ofs: i64 = 0;
        let mut ofs: i64 = 1;
        let at = |sorter: &Self, offset: i64| {
            sorter.element(source, tmp, (base as i64 + offset) as usize)
        };
        let hinted = at(self, hint);
        if self.cmp(key, &hinted) < 0 {
            let max_ofs = hint + 1;
            while ofs < max_ofs {
                let candidate = at(self, hint - ofs);
                if self.cmp(key, &candidate) >= 0 {
                    break;
                }
                last_ofs = ofs;
                ofs = (ofs << 1) + 1;
                if ofs > i32::MAX as i64 {
                    ofs = max_ofs;
                }
            }
            if ofs > max_ofs {
                ofs = max_ofs;
            }
            let previous = last_ofs;
            last_ofs = hint - ofs;
            ofs = hint - previous;
        } else {
            let max_ofs = len - hint;
            while ofs < max_ofs {
                let candidate = at(self, hint + ofs);
                if self.cmp(key, &candidate) < 0 {
                    break;
                }
                last_ofs = ofs;
                ofs = (ofs << 1) + 1;
                if ofs > i32::MAX as i64 {
                    ofs = max_ofs;
                }
            }
            if ofs > max_ofs {
                ofs = max_ofs;
            }
            last_ofs += hint;
            ofs += hint;
        }
        last_ofs += 1;
        while last_ofs < ofs {
            let m = last_ofs + ((ofs - last_ofs) >> 1);
            let candidate = at(self, m);
            if self.cmp(key, &candidate) < 0 {
                ofs = m;
            } else {
                last_ofs = m + 1;
            }
        }
        ofs as usize
    }

    fn merge_lo(
        &mut self,
        base1: usize,
        len1: usize,
        base2: usize,
        len2: usize,
    ) -> Result<(), JavaSortError> {
        let tmp: Vec<T> = self.items[base1..base1 + len1].to_vec();
        let mut len1 = len1;
        let mut len2 = len2;
        let mut cursor1 = 0usize;
        let mut cursor2 = base2;
        let mut dest = base1;
        self.items[dest] = self.items[cursor2].clone();
        dest += 1;
        cursor2 += 1;
        len2 -= 1;
        if len2 == 0 {
            self.items[dest..dest + len1].clone_from_slice(&tmp[cursor1..cursor1 + len1]);
            return Ok(());
        }
        if len1 == 1 {
            self.items.copy_within_clone(cursor2, dest, len2);
            self.items[dest + len2] = tmp[cursor1].clone();
            return Ok(());
        }
        let mut min_gallop = self.min_gallop;
        'outer: loop {
            let mut count1: i64 = 0;
            let mut count2: i64 = 0;
            loop {
                let right = self.items[cursor2].clone();
                if self.cmp(&right, &tmp[cursor1]) < 0 {
                    self.items[dest] = right;
                    dest += 1;
                    cursor2 += 1;
                    count2 += 1;
                    count1 = 0;
                    len2 -= 1;
                    if len2 == 0 {
                        break 'outer;
                    }
                } else {
                    self.items[dest] = tmp[cursor1].clone();
                    dest += 1;
                    cursor1 += 1;
                    count1 += 1;
                    count2 = 0;
                    len1 -= 1;
                    if len1 == 1 {
                        break 'outer;
                    }
                }
                if (count1 | count2) >= min_gallop {
                    break;
                }
            }
            loop {
                let key = self.items[cursor2].clone();
                count1 = self.gallop_right_in(&key, Source::Tmp, &tmp, cursor1, len1, 0) as i64;
                if count1 != 0 {
                    let count = count1 as usize;
                    self.items[dest..dest + count].clone_from_slice(&tmp[cursor1..cursor1 + count]);
                    dest += count;
                    cursor1 += count;
                    len1 -= count;
                    if len1 <= 1 {
                        break 'outer;
                    }
                }
                self.items[dest] = self.items[cursor2].clone();
                dest += 1;
                cursor2 += 1;
                len2 -= 1;
                if len2 == 0 {
                    break 'outer;
                }
                let key = tmp[cursor1].clone();
                count2 = self.gallop_left_in(&key, Source::Items, &[], cursor2, len2, 0) as i64;
                if count2 != 0 {
                    let count = count2 as usize;
                    self.items.copy_within_clone(cursor2, dest, count);
                    dest += count;
                    cursor2 += count;
                    len2 -= count;
                    if len2 == 0 {
                        break 'outer;
                    }
                }
                self.items[dest] = tmp[cursor1].clone();
                dest += 1;
                cursor1 += 1;
                len1 -= 1;
                if len1 == 1 {
                    break 'outer;
                }
                min_gallop -= 1;
                if !(count1 >= MIN_GALLOP || count2 >= MIN_GALLOP) {
                    break;
                }
            }
            if min_gallop < 0 {
                min_gallop = 0;
            }
            min_gallop += 2;
        }
        self.min_gallop = if min_gallop < 1 { 1 } else { min_gallop };
        if len1 == 1 {
            self.items.copy_within_clone(cursor2, dest, len2);
            self.items[dest + len2] = tmp[cursor1].clone();
            Ok(())
        } else if len1 == 0 {
            Err(JavaSortError::ContractViolation)
        } else {
            self.items[dest..dest + len1].clone_from_slice(&tmp[cursor1..cursor1 + len1]);
            Ok(())
        }
    }

    fn merge_hi(
        &mut self,
        base1: usize,
        len1: usize,
        base2: usize,
        len2: usize,
    ) -> Result<(), JavaSortError> {
        let tmp: Vec<T> = self.items[base2..base2 + len2].to_vec();
        let mut len1 = len1 as i64;
        let mut len2 = len2 as i64;
        let mut cursor1 = base1 as i64 + len1 - 1;
        let mut cursor2 = len2 - 1;
        let mut dest = base2 as i64 + len2 - 1;
        self.items[dest as usize] = self.items[cursor1 as usize].clone();
        dest -= 1;
        cursor1 -= 1;
        len1 -= 1;
        if len1 == 0 {
            let start = (dest - (len2 - 1)) as usize;
            self.items[start..start + len2 as usize].clone_from_slice(&tmp[..len2 as usize]);
            return Ok(());
        }
        if len2 == 1 {
            dest -= len1;
            cursor1 -= len1;
            self.items.copy_within_clone(
                (cursor1 + 1) as usize,
                (dest + 1) as usize,
                len1 as usize,
            );
            self.items[dest as usize] = tmp[cursor2 as usize].clone();
            return Ok(());
        }
        let mut min_gallop = self.min_gallop;
        'outer: loop {
            let mut count1: i64 = 0;
            let mut count2: i64 = 0;
            loop {
                let left = self.items[cursor1 as usize].clone();
                if self.cmp(&tmp[cursor2 as usize], &left) < 0 {
                    self.items[dest as usize] = left;
                    dest -= 1;
                    cursor1 -= 1;
                    count1 += 1;
                    count2 = 0;
                    len1 -= 1;
                    if len1 == 0 {
                        break 'outer;
                    }
                } else {
                    self.items[dest as usize] = tmp[cursor2 as usize].clone();
                    dest -= 1;
                    cursor2 -= 1;
                    count2 += 1;
                    count1 = 0;
                    len2 -= 1;
                    if len2 == 1 {
                        break 'outer;
                    }
                }
                if (count1 | count2) >= min_gallop {
                    break;
                }
            }
            loop {
                let key = tmp[cursor2 as usize].clone();
                count1 = len1
                    - self.gallop_right_in(
                        &key,
                        Source::Items,
                        &[],
                        base1,
                        len1 as usize,
                        (len1 - 1) as usize,
                    ) as i64;
                if count1 != 0 {
                    dest -= count1;
                    cursor1 -= count1;
                    len1 -= count1;
                    self.items.copy_within_clone(
                        (cursor1 + 1) as usize,
                        (dest + 1) as usize,
                        count1 as usize,
                    );
                    if len1 == 0 {
                        break 'outer;
                    }
                }
                self.items[dest as usize] = tmp[cursor2 as usize].clone();
                dest -= 1;
                cursor2 -= 1;
                len2 -= 1;
                if len2 == 1 {
                    break 'outer;
                }
                let key = self.items[cursor1 as usize].clone();
                count2 = len2
                    - self.gallop_left_in(
                        &key,
                        Source::Tmp,
                        &tmp,
                        0,
                        len2 as usize,
                        (len2 - 1) as usize,
                    ) as i64;
                if count2 != 0 {
                    dest -= count2;
                    cursor2 -= count2;
                    len2 -= count2;
                    let from = (cursor2 + 1) as usize;
                    let to = (dest + 1) as usize;
                    self.items[to..to + count2 as usize]
                        .clone_from_slice(&tmp[from..from + count2 as usize]);
                    if len2 <= 1 {
                        break 'outer;
                    }
                }
                self.items[dest as usize] = self.items[cursor1 as usize].clone();
                dest -= 1;
                cursor1 -= 1;
                len1 -= 1;
                if len1 == 0 {
                    break 'outer;
                }
                min_gallop -= 1;
                if !(count1 >= MIN_GALLOP || count2 >= MIN_GALLOP) {
                    break;
                }
            }
            if min_gallop < 0 {
                min_gallop = 0;
            }
            min_gallop += 2;
        }
        self.min_gallop = if min_gallop < 1 { 1 } else { min_gallop };
        if len2 == 1 {
            dest -= len1;
            cursor1 -= len1;
            self.items.copy_within_clone(
                (cursor1 + 1) as usize,
                (dest + 1) as usize,
                len1 as usize,
            );
            self.items[dest as usize] = tmp[cursor2 as usize].clone();
            Ok(())
        } else if len2 == 0 {
            Err(JavaSortError::ContractViolation)
        } else {
            let start = (dest - (len2 - 1)) as usize;
            self.items[start..start + len2 as usize].clone_from_slice(&tmp[..len2 as usize]);
            Ok(())
        }
    }
}

#[derive(Clone, Copy)]
enum Source {
    Items,
    Tmp,
}

fn min_run_length(mut n: usize) -> usize {
    let mut r = 0;
    while n >= MIN_MERGE {
        r |= n & 1;
        n >>= 1;
    }
    n + r
}

trait CopyWithinClone {
    fn copy_within_clone(&mut self, source: usize, destination: usize, count: usize);
}

impl<T: Clone> CopyWithinClone for [T] {
    fn copy_within_clone(&mut self, source: usize, destination: usize, count: usize) {
        let block: Vec<T> = self[source..source + count].to_vec();
        self[destination..destination + count].clone_from_slice(&block);
    }
}
