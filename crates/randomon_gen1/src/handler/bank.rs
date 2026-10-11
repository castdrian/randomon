use randomon_core::{Movesets, RandomizationError, Result, RomHandler};
use randomon_data::constants::gen1;
use randomon_data::{MoveLearnt, PokemonId, Trainer};
use randomon_gb::{
    BankPacker, Placement, Region, bank_end, bank_of, calculate_offset, make_gb_pointer,
    merge_regions, read_word, write_word,
};

use super::Gen1RomHandler;

const SPECIAL_FORMAT: u8 = 0xFF;
const MAX_PARTY_SIZE: usize = 6;
const LEVEL_ONE_MOVE_SLOTS: usize = 4;

#[derive(Clone, Debug, Default)]
pub(super) struct TrainerLayout {
    pub class_regions: Vec<Option<Region>>,
    pub party_sizes: Vec<usize>,
    pub special_formats: Vec<bool>,
    pub species_offsets: Vec<Vec<i32>>,
    pub class_of_position: Vec<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct PinnedSlot {
    pub original: i32,
    pub position: usize,
    pub slot: usize,
}

impl TrainerLayout {
    pub fn pin(&self, offsets: impl IntoIterator<Item = i32>) -> Vec<PinnedSlot> {
        let mut pinned = Vec::new();
        for original in offsets {
            for (position, species) in self.species_offsets.iter().enumerate() {
                if let Some(slot) = species.iter().position(|&offset| offset == original) {
                    pinned.push(PinnedSlot {
                        original,
                        position,
                        slot,
                    });
                }
            }
        }
        pinned
    }
}

fn byte_at(rom: &[u8], offset: i32) -> Option<u8> {
    usize::try_from(offset)
        .ok()
        .and_then(|index| rom.get(index).copied())
}

fn party_end(rom: &[u8], mut offset: i32) -> Option<(i32, Vec<i32>)> {
    let special = byte_at(rom, offset)? == SPECIAL_FORMAT;
    let (stride, species_delta) = if special { (2, 1) } else { (1, 0) };
    offset += 1;
    let mut species = Vec::new();
    while byte_at(rom, offset)? != 0 {
        species.push(offset + species_delta);
        offset += stride;
    }
    Some((offset + 1, species))
}

pub(super) fn measure_trainer_layout(rom: &[u8], table: i32, counts: &[i32]) -> TrainerLayout {
    let mut layout = TrainerLayout::default();
    for class in 1..=gen1::TRAINER_CLASS_COUNT {
        let count = counts.get(class as usize).copied().unwrap_or(0);
        let pointer_offset = table + (class - 1) * 2;
        if byte_at(rom, pointer_offset + 1).is_none() {
            return TrainerLayout::default();
        }
        let start = calculate_offset(bank_of(table), read_word(rom, pointer_offset));
        let mut offset = start;
        for _ in 0..count {
            let special = byte_at(rom, offset) == Some(SPECIAL_FORMAT);
            let Some((next, species)) = party_end(rom, offset) else {
                return TrainerLayout::default();
            };
            layout.party_sizes.push(species.len());
            layout.special_formats.push(special);
            layout.class_of_position.push(class as usize);
            layout.species_offsets.push(species);
            offset = next;
        }
        layout
            .class_regions
            .push((count > 0).then(|| Region::new(start, offset - start)));
    }
    layout
}

fn overlaps(first: Region, second: Region) -> bool {
    first.offset < second.end() && second.offset < first.end()
}

struct ClassBlob {
    bytes: Vec<u8>,
    species_offsets: Vec<Vec<i32>>,
}

fn class_blob(trainers: &[Trainer], internal_of: &dyn Fn(PokemonId) -> u8) -> ClassBlob {
    let mut blob = Vec::new();
    let mut species_offsets = Vec::with_capacity(trainers.len());
    for trainer in trainers {
        let party = &trainer.pokemon[..trainer.pokemon.len().min(MAX_PARTY_SIZE)];
        let level_of = |level: i32| level.clamp(1, 100) as u8;
        let shared_level = party
            .first()
            .map(|first| level_of(first.level))
            .filter(|&level| party.iter().all(|tp| level_of(tp.level) == level))
            .filter(|_| trainer.poketype & 1 == 0);
        match shared_level {
            Some(level) => {
                blob.push(level);
                let mut offsets = Vec::with_capacity(party.len());
                for tp in party {
                    offsets.push(blob.len() as i32);
                    blob.push(internal_of(tp.pokemon));
                }
                species_offsets.push(offsets);
            }
            None if party.is_empty() => {
                blob.push(1);
                species_offsets.push(Vec::new());
            }
            None => {
                blob.push(SPECIAL_FORMAT);
                let mut offsets = Vec::with_capacity(party.len());
                for tp in party {
                    blob.push(level_of(tp.level));
                    offsets.push(blob.len() as i32);
                    blob.push(internal_of(tp.pokemon));
                }
                species_offsets.push(offsets);
            }
        }
        blob.push(0);
    }
    ClassBlob {
        bytes: blob,
        species_offsets,
    }
}

fn trim_rank(trainer: &Trainer) -> u8 {
    if trainer.is_boss() {
        2
    } else if trainer.is_important() {
        1
    } else {
        0
    }
}

fn trim_one_extra(trainers: &mut [Trainer], original_sizes: &[usize]) -> bool {
    let candidate = trainers
        .iter()
        .enumerate()
        .filter_map(|(position, trainer)| {
            let original = original_sizes.get(position).copied().unwrap_or(1).max(1);
            let extra = trainer.pokemon.len().saturating_sub(original);
            (extra > 0).then_some((trim_rank(trainer), std::cmp::Reverse(extra), position))
        })
        .min();
    match candidate {
        Some((_, _, position)) => {
            let party = &mut trainers[position].pokemon;
            party.remove(party.len().saturating_sub(2));
            true
        }
        None => false,
    }
}

fn moveset_placement(preference: &[usize]) -> Placement<'_> {
    Placement {
        preference,
        share_identical: true,
        share_leading_zero: true,
    }
}

fn trainer_placement(preference: &[usize]) -> Placement<'_> {
    Placement {
        preference,
        ..Placement::default()
    }
}

struct BankLayout {
    regions: Vec<Region>,
    moveset_regions: Vec<usize>,
    class_regions: Vec<Option<usize>>,
    moveset_spill: Vec<usize>,
    trainer_spill: Vec<usize>,
}

struct PackedBank {
    packer: BankPacker,
    moveset_pointers: Vec<i32>,
    class_pointers: Vec<i32>,
}

impl Gen1RomHandler {
    fn moveset_table(&self) -> i32 {
        self.value("PokemonMovesetsTableOffset")
    }

    fn moveset_regions(&self) -> Vec<Region> {
        let table = self.moveset_table();
        let count = self.value("InternalPokemonCount");
        let main = Region::new(table + count * 2, self.value("PokemonMovesetsDataSize"));
        let mut regions = vec![main];
        let extra = self.value("PokemonMovesetsExtraSpaceOffset");
        if extra != 0 && bank_of(extra) == bank_of(table) {
            regions.push(Region::new(extra, bank_end(extra) - extra));
        }
        regions
    }

    fn pinned_classes(&self) -> Vec<usize> {
        self.pinned_slots
            .iter()
            .filter_map(|pinned| {
                self.trainer_layout
                    .class_of_position
                    .get(pinned.position)
                    .copied()
            })
            .collect()
    }

    fn bank_layout(&self, compact: bool) -> BankLayout {
        let moveset_bank = bank_of(self.moveset_table());
        let trainer_bank = bank_of(self.value("TrainerDataTableOffset"));
        let mut regions: Vec<Region> = self
            .moveset_regions()
            .into_iter()
            .filter(|region| bank_of(region.offset) == moveset_bank)
            .collect();
        let moveset_regions: Vec<usize> = (0..regions.len()).collect();
        let pinned = self.pinned_classes();
        let mut class_regions = Vec::with_capacity(self.trainer_layout.class_regions.len());
        let mut shared = Vec::new();
        for (index, extent) in self.trainer_layout.class_regions.iter().enumerate() {
            let usable = extent.filter(|&region| {
                bank_of(region.offset) == trainer_bank
                    && !regions.iter().any(|&other| overlaps(region, other))
            });
            let keeps_own = !compact || pinned.contains(&(index + 1));
            match usable {
                Some(region) if keeps_own => {
                    regions.push(region);
                    class_regions.push(Some(regions.len() - 1));
                }
                Some(region) => {
                    shared.push(region);
                    class_regions.push(None);
                }
                None => class_regions.push(None),
            }
        }
        let shared_start = regions.len();
        regions.extend(merge_regions(shared));
        let mut trainer_regions: Vec<usize> = (shared_start..regions.len()).collect();
        trainer_regions.extend(class_regions.iter().flatten().copied());
        let in_bank = |indices: &[usize], bank: i32| -> Vec<usize> {
            indices
                .iter()
                .copied()
                .filter(|&index| bank_of(regions[index].offset) == bank)
                .collect()
        };
        let moveset_spill = [
            in_bank(&moveset_regions, moveset_bank),
            in_bank(&trainer_regions, moveset_bank),
        ]
        .concat();
        let trainer_spill = [
            in_bank(&trainer_regions, trainer_bank),
            in_bank(&moveset_regions, trainer_bank),
        ]
        .concat();
        BankLayout {
            regions,
            moveset_regions,
            class_regions,
            moveset_spill,
            trainer_spill,
        }
    }

    fn evolution_and_move_blob(&self, number: i32, moves: &[MoveLearnt]) -> Vec<u8> {
        let mut blob = self.evolution_bytes(PokemonId::from_index(number as usize));
        blob.push(0);
        let level_one = moves
            .iter()
            .take(LEVEL_ONE_MOVE_SLOTS)
            .take_while(|learnt| learnt.level == 1)
            .count();
        for learnt in &moves[level_one..] {
            blob.push(learnt.level.clamp(1, 100) as u8);
            blob.push(self.move_num_to_rom[learnt.move_number as usize] as u8);
        }
        blob.push(0);
        blob
    }

    fn write_level_one_moves(&mut self, number: i32, moves: &[MoveLearnt]) {
        let stats_offset = self.base_stats_offset(number);
        let level_one: Vec<u8> = moves
            .iter()
            .take(LEVEL_ONE_MOVE_SLOTS)
            .take_while(|learnt| learnt.level == 1)
            .map(|learnt| self.move_num_to_rom[learnt.move_number as usize] as u8)
            .collect();
        for slot in 0..LEVEL_ONE_MOVE_SLOTS {
            let value = level_one.get(slot).copied().unwrap_or(0);
            self.set_byte(
                stats_offset + gen1::BS_LEVEL1_MOVES_OFFSET + slot as i32,
                value,
            );
        }
    }

    fn moveset_blobs(&mut self, movesets: &Movesets) -> Vec<Vec<u8>> {
        let current = self.get_moves_learnt();
        let count = self.value("InternalPokemonCount");
        let mut blobs = Vec::with_capacity(count as usize);
        for internal in 1..=count {
            let number = self.poke_rby_to_num[internal as usize];
            if number == 0 {
                blobs.push(vec![0, 0]);
                continue;
            }
            let moves = movesets
                .get(&number)
                .or_else(|| current.get(&number))
                .cloned()
                .unwrap_or_default();
            self.write_level_one_moves(number, &moves);
            blobs.push(self.evolution_and_move_blob(number, &moves));
        }
        blobs
    }

    fn class_blobs(&self, trainers: &[Trainer]) -> Result<Vec<ClassBlob>> {
        let counts = self.entry.array("TrainerDataClassCounts");
        let internal_of = |pokemon: PokemonId| self.internal_of(pokemon);
        let mut blobs = Vec::new();
        let mut position = 0;
        for class in 1..=gen1::TRAINER_CLASS_COUNT {
            let count = counts.get(class as usize).copied().unwrap_or(0).max(0) as usize;
            let members = trainers
                .get(position..position + count)
                .ok_or(RandomizationError::IndexOutOfBounds("trainers"))?;
            blobs.push(class_blob(members, &internal_of));
            position += count;
        }
        Ok(blobs)
    }

    fn try_pack(
        &self,
        moveset_blobs: &[Vec<u8>],
        class_blobs: &[ClassBlob],
        compact: bool,
    ) -> std::result::Result<PackedBank, i32> {
        let layout = self.bank_layout(compact);
        let mut packer = BankPacker::new(&layout.regions, 0);
        let mut moveset_pointers: Vec<Option<i32>> = moveset_blobs
            .iter()
            .map(|blob| packer.place(blob, moveset_placement(&layout.moveset_regions)))
            .collect();
        let mut class_pointers: Vec<Option<i32>> = class_blobs
            .iter()
            .enumerate()
            .map(|(class, blob)| {
                let own = layout.class_regions.get(class).copied().flatten();
                match own {
                    _ if blob.bytes.is_empty() => Some(-1),
                    Some(region) => packer.place(&blob.bytes, trainer_placement(&[region])),
                    None => None,
                }
            })
            .collect();
        let mut deficit = 0;
        for (pointer, blob) in moveset_pointers.iter_mut().zip(moveset_blobs) {
            if pointer.is_none() {
                *pointer = packer.place(blob, moveset_placement(&layout.moveset_spill));
                deficit += if pointer.is_none() {
                    blob.len() as i32
                } else {
                    0
                };
            }
        }
        for (pointer, blob) in class_pointers.iter_mut().zip(class_blobs) {
            if pointer.is_none() {
                *pointer = packer.place(&blob.bytes, trainer_placement(&layout.trainer_spill));
                deficit += if pointer.is_none() {
                    blob.bytes.len() as i32
                } else {
                    0
                };
            }
        }
        if deficit > 0 {
            return Err(deficit);
        }
        Ok(PackedBank {
            packer,
            moveset_pointers: moveset_pointers.into_iter().flatten().collect(),
            class_pointers: class_pointers.into_iter().flatten().collect(),
        })
    }

    fn write_packed(&mut self, packed: &PackedBank) {
        packed.packer.write_into(&mut self.rom);
        let moveset_table = self.moveset_table();
        for (index, &offset) in packed.moveset_pointers.iter().enumerate() {
            write_word(
                &mut self.rom,
                moveset_table + index as i32 * 2,
                make_gb_pointer(offset),
            );
        }
        let trainer_table = self.value("TrainerDataTableOffset");
        let mut previous = packed
            .class_pointers
            .iter()
            .copied()
            .find(|&offset| offset >= 0)
            .unwrap_or(0);
        for (index, &offset) in packed.class_pointers.iter().enumerate() {
            let offset = if offset < 0 { previous } else { offset };
            previous = offset;
            write_word(
                &mut self.rom,
                trainer_table + index as i32 * 2,
                make_gb_pointer(offset),
            );
        }
    }

    fn relocate_pinned(&mut self, packed: &PackedBank, class_blobs: &[ClassBlob]) {
        let mut locations = Vec::new();
        for (class, blob) in class_blobs.iter().enumerate() {
            let base = packed.class_pointers.get(class).copied().unwrap_or(-1);
            for offsets in &blob.species_offsets {
                locations.push(
                    offsets
                        .iter()
                        .map(|&relative| base + relative)
                        .collect::<Vec<_>>(),
                );
            }
        }
        for pinned in self.pinned_slots.clone() {
            let Some(offsets) = locations.get(pinned.position) else {
                continue;
            };
            let Some(&offset) = offsets.get(pinned.slot).or_else(|| offsets.last()) else {
                continue;
            };
            if offset >= 0 {
                self.relocated_offsets.insert(pinned.original, offset);
            }
        }
    }

    pub(super) fn resolve_offset(&self, offset: i32) -> i32 {
        self.relocated_offsets
            .get(&offset)
            .copied()
            .unwrap_or(offset)
    }

    fn freeze_pinned_classes(&self, trainers: &mut [Trainer]) {
        let layout = &self.trainer_layout;
        let pinned_classes: Vec<usize> = self
            .pinned_slots
            .iter()
            .filter_map(|pinned| layout.class_of_position.get(pinned.position).copied())
            .collect();
        for (position, trainer) in trainers.iter_mut().enumerate() {
            let Some(class) = layout.class_of_position.get(position) else {
                continue;
            };
            if !pinned_classes.contains(class) {
                continue;
            }
            let original = layout.party_sizes[position].max(1);
            while trainer.pokemon.len() > original {
                let index = trainer.pokemon.len().saturating_sub(2);
                trainer.pokemon.remove(index);
            }
            if layout.special_formats[position] {
                trainer.poketype |= 1;
            } else if let Some(level) = trainer.pokemon.iter().map(|tp| tp.level).max() {
                trainer.poketype &= !1;
                for tp in &mut trainer.pokemon {
                    tp.level = level;
                }
            }
        }
    }

    pub(super) fn repack_bank(&mut self, movesets: &Movesets, trainers: &[Trainer]) -> Result<()> {
        let moveset_blobs = self.moveset_blobs(movesets);
        let mut trainers = trainers.to_vec();
        for trainer in &mut trainers {
            trainer.pokemon.truncate(MAX_PARTY_SIZE);
        }
        self.freeze_pinned_classes(&mut trainers);
        let original_sizes = self.trainer_layout.party_sizes.clone();
        loop {
            let class_blobs = self.class_blobs(&trainers)?;
            let packed = self
                .try_pack(&moveset_blobs, &class_blobs, false)
                .or_else(|_| self.try_pack(&moveset_blobs, &class_blobs, true));
            match packed {
                Ok(packed) => {
                    self.write_packed(&packed);
                    self.relocate_pinned(&packed, &class_blobs);
                    return Ok(());
                }
                Err(deficit) => {
                    for _ in 0..deficit.max(1) {
                        if !trim_one_extra(&mut trainers, &original_sizes) {
                            return Err(RandomizationError::randomization(
                                "Unable to fit trainers, moves and evolutions into the ROM",
                            ));
                        }
                    }
                }
            }
        }
    }
}
