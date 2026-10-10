use crate::random::JavaRng;

pub fn shuffle<T>(list: &mut [T], rnd: &mut impl JavaRng) {
    let size = i32::try_from(list.len()).expect("Java lists hold at most i32::MAX elements");
    for i in (2..=size).rev() {
        let j = rnd.next_int_bounded(i);
        list.swap((i - 1) as usize, j as usize);
    }
}
