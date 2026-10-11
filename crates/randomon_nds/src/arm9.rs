use crate::data::{find, read_full_int, write_full_int};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Arm9Extension {
    extended: bool,
}

impl Arm9Extension {
    pub fn is_extended(&self) -> bool {
        self.extended
    }

    pub fn extend(
        &mut self,
        arm9: Vec<u8>,
        extend_by: i32,
        prefix: &str,
        arm9_offset: i32,
    ) -> Vec<u8> {
        if self.extended {
            return arm9;
        }
        let extended = extend_arm9(&arm9, extend_by, prefix, arm9_offset);
        self.extended = true;
        extended
    }
}

pub fn extend_arm9(arm9: &[u8], extend_by: i32, prefix: &str, arm9_offset: i32) -> Vec<u8> {
    let pointers_offset = find(arm9, prefix) + prefix.len() as i32 / 2;
    let old_destination_pointers = read_full_int(arm9, pointers_offset).wrapping_sub(arm9_offset);
    let itcm_source = read_full_int(arm9, pointers_offset + 8).wrapping_sub(arm9_offset);
    let itcm_size_offset = old_destination_pointers + 4;
    let old_itcm_size = read_full_int(arm9, itcm_size_offset);
    let old_dtcm_offset = itcm_source + old_itcm_size;
    let mut extended = arm9.to_vec();
    extended.resize(arm9.len() + extend_by as usize, 0);
    write_full_int(
        &mut extended,
        pointers_offset,
        old_destination_pointers + extend_by + arm9_offset,
    );
    let extended_end = extended.len() as i32 + arm9_offset;
    write_full_int(&mut extended, pointers_offset + 4, extended_end);
    write_full_int(&mut extended, itcm_size_offset, old_itcm_size + extend_by);
    let start = old_dtcm_offset as usize;
    extended.copy_within(start..arm9.len(), start + extend_by as usize);
    extended
}
