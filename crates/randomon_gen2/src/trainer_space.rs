use randomon_core::{Movesets, RandomizationError, Result};
use randomon_data::constants::gb;
use randomon_data::{Trainer, TrainerPokemon, moves_at_level};
use randomon_gb::{
    BankPacker, Placement, Region, bank_of, calculate_offset, make_gb_pointer, merge_regions,
    read_word, uniform_tail,
};

use crate::handler::Gen2RomHandler;

const POKEMON_TERMINATOR: u8 = 0xFF;
const ITEMS_FLAG: i32 = 2;
const MOVES_FLAG: i32 = 1;
const MAX_PARTY_SIZE: usize = 6;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct TrainerShape {
    pub party_size: usize,
    pub data_type: i32,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct TrainerSpace {
    pub class_regions: Vec<Option<Region>>,
    pub tail: Option<Region>,
    pub filler: u8,
    pub shapes: Vec<TrainerShape>,
}

fn byte_at(rom: &[u8], offset: i32) -> Option<u8> {
    usize::try_from(offset)
        .ok()
        .and_then(|index| rom.get(index).copied())
}

fn entry_length(data_type: i32) -> i32 {
    let mut length = 2;
    if data_type & ITEMS_FLAG != 0 {
        length += 1;
    }
    if data_type & MOVES_FLAG != 0 {
        length += 4;
    }
    length
}

fn trainer_end(rom: &[u8], mut offset: i32) -> Option<(i32, TrainerShape)> {
    while byte_at(rom, offset)? != gb::STRING_TERMINATOR {
        offset += 1;
    }
    offset += 1;
    let data_type = i32::from(byte_at(rom, offset)?);
    offset += 1;
    let mut party_size = 0;
    while byte_at(rom, offset)? != POKEMON_TERMINATOR {
        offset += entry_length(data_type);
        party_size += 1;
    }
    Some((
        offset + 1,
        TrainerShape {
            party_size,
            data_type,
        },
    ))
}

pub(crate) fn measure_trainer_space(rom: &[u8], table: i32, limits: &[i32]) -> TrainerSpace {
    let mut space = TrainerSpace::default();
    let mut data_end = 0;
    for (class, &limit) in limits.iter().enumerate() {
        let pointer_offset = table + class as i32 * 2;
        if byte_at(rom, pointer_offset + 1).is_none() {
            return TrainerSpace::default();
        }
        let start = calculate_offset(bank_of(table), read_word(rom, pointer_offset));
        let mut offset = start;
        for _ in 0..limit {
            let Some((next, shape)) = trainer_end(rom, offset) else {
                return TrainerSpace::default();
            };
            space.shapes.push(shape);
            offset = next;
        }
        let used = limit > 0 && offset > start;
        space
            .class_regions
            .push(used.then(|| Region::new(start, offset - start)));
        if used {
            data_end = data_end.max(offset);
        }
    }
    if let Some((tail, filler)) =
        uniform_tail(rom, data_end).filter(|_| bank_of(data_end) == bank_of(table))
    {
        space.tail = Some(tail);
        space.filler = filler;
    } else {
        space.filler = POKEMON_TERMINATOR;
    }
    space
}

fn shrink_rank(trainer: &Trainer) -> u8 {
    if trainer.is_boss() {
        2
    } else if trainer.is_important() {
        1
    } else {
        0
    }
}

fn party_bytes(trainer: &Trainer) -> i32 {
    trainer.pokemon.len().min(MAX_PARTY_SIZE) as i32 * entry_length(trainer.poketype)
}

fn shrink_one(trainers: &mut [Trainer], shapes: &[TrainerShape]) -> Option<i32> {
    let original = |position: usize| shapes.get(position).copied().unwrap_or_default();
    let extra_pokemon = trainers
        .iter()
        .enumerate()
        .filter_map(|(position, trainer)| {
            let extra = trainer
                .pokemon
                .len()
                .saturating_sub(original(position).party_size.max(1));
            (extra > 0).then_some((shrink_rank(trainer), std::cmp::Reverse(extra), position))
        })
        .min();
    if let Some((_, _, position)) = extra_pokemon {
        let before = party_bytes(&trainers[position]);
        let party = &mut trainers[position].pokemon;
        party.remove(party.len().saturating_sub(2));
        return Some(before - party_bytes(&trainers[position]));
    }
    for flag in [MOVES_FLAG, ITEMS_FLAG] {
        let added = trainers
            .iter()
            .enumerate()
            .filter(|(position, trainer)| {
                trainer.poketype & flag != 0 && original(*position).data_type & flag == 0
            })
            .map(|(position, trainer)| (shrink_rank(trainer), position))
            .min();
        if let Some((_, position)) = added {
            let before = party_bytes(&trainers[position]);
            trainers[position].poketype &= !flag;
            return Some(before - party_bytes(&trainers[position]));
        }
    }
    None
}

impl Gen2RomHandler {
    fn trainer_entry(
        &self,
        trainer: &Trainer,
        tp: &TrainerPokemon,
        movesets: &Movesets,
        out: &mut Vec<u8>,
    ) -> Result<()> {
        out.push(tp.level.clamp(1, 100) as u8);
        let number = self.number_of(tp.pokemon);
        out.push(number);
        if trainer.poketype & ITEMS_FLAG != 0 {
            out.push(tp.held_item.clamp(0, 0xFF) as u8);
        }
        if trainer.poketype & MOVES_FLAG != 0 {
            let moves = if tp.reset_moves {
                let moveset = movesets
                    .get(&i32::from(number))
                    .ok_or(RandomizationError::NullPointer("movesets.get"))?;
                moves_at_level(moveset, tp.level, 0)
            } else {
                tp.moves
            };
            out.extend(moves.iter().map(|&mv| mv.clamp(0, 0xFF) as u8));
        }
        Ok(())
    }

    fn class_blobs(&self, trainers: &[Trainer], movesets: &Movesets) -> Result<Vec<Vec<u8>>> {
        let mut blobs = Vec::new();
        let mut members = trainers.iter();
        for &limit in self.array("TrainerDataClassCounts") {
            let mut blob = Vec::new();
            for _ in 0..limit {
                let trainer = members
                    .next()
                    .ok_or(RandomizationError::IndexOutOfBounds("trainers"))?;
                blob.extend(self.translate(trainer.name.as_deref().unwrap_or_default()));
                blob.push(gb::STRING_TERMINATOR);
                blob.push((trainer.poketype & (ITEMS_FLAG | MOVES_FLAG)) as u8);
                for tp in trainer.pokemon.iter().take(MAX_PARTY_SIZE) {
                    self.trainer_entry(trainer, tp, movesets, &mut blob)?;
                }
                blob.push(POKEMON_TERMINATOR);
            }
            blobs.push(blob);
        }
        Ok(blobs)
    }

    fn pack_trainer_classes(
        &self,
        blobs: &[Vec<u8>],
        compact: bool,
    ) -> std::result::Result<(BankPacker, Vec<i32>), i32> {
        let space = &self.trainer_space;
        let mut regions: Vec<Region> = Vec::new();
        let mut own: Vec<Option<usize>> = Vec::new();
        let mut spill: Vec<usize> = Vec::new();
        if compact {
            let all = space
                .class_regions
                .iter()
                .flatten()
                .copied()
                .chain(space.tail);
            regions = merge_regions(all.collect());
            own = vec![None; space.class_regions.len()];
            spill = (0..regions.len()).collect();
        } else {
            for region in &space.class_regions {
                own.push(region.map(|region| {
                    regions.push(region);
                    regions.len() - 1
                }));
            }
            if let Some(tail) = space.tail {
                regions.push(tail);
                spill.push(regions.len() - 1);
            }
            spill.extend(own.iter().flatten().copied());
        }
        let mut packer = BankPacker::new(&regions, space.filler);
        let mut pointers: Vec<Option<i32>> = blobs
            .iter()
            .enumerate()
            .map(|(class, blob)| match own.get(class).copied().flatten() {
                _ if blob.is_empty() => Some(-1),
                Some(region) => packer.place(blob, placement(&[region])),
                None => None,
            })
            .collect();
        let mut deficit = 0;
        for (pointer, blob) in pointers.iter_mut().zip(blobs) {
            if pointer.is_none() {
                *pointer = packer.place(blob, placement(&spill));
                deficit += if pointer.is_none() {
                    blob.len() as i32
                } else {
                    0
                };
            }
        }
        if deficit > 0 {
            return Err(deficit);
        }
        Ok((packer, pointers.into_iter().flatten().collect()))
    }

    fn write_class_pointers(&mut self, pointers: &[i32]) {
        let table = self.value("TrainerDataTableOffset");
        let mut previous = pointers
            .iter()
            .copied()
            .find(|&offset| offset >= 0)
            .unwrap_or(0);
        for (class, &offset) in pointers.iter().enumerate() {
            let offset = if offset < 0 { previous } else { offset };
            previous = offset;
            self.set_word(table + class as i32 * 2, make_gb_pointer(offset));
        }
    }

    pub(crate) fn store_trainers(&mut self, trainers: &[Trainer]) -> Result<()> {
        let movesets = self.read_moves_learnt();
        let mut trainers = trainers.to_vec();
        for trainer in &mut trainers {
            trainer.pokemon.truncate(MAX_PARTY_SIZE);
        }
        let shapes = self.trainer_space.shapes.clone();
        loop {
            let blobs = self.class_blobs(&trainers, &movesets)?;
            let packed = self
                .pack_trainer_classes(&blobs, false)
                .or_else(|_| self.pack_trainer_classes(&blobs, true));
            match packed {
                Ok((packer, pointers)) => {
                    packer.write_into(&mut self.rom);
                    self.write_class_pointers(&pointers);
                    return Ok(());
                }
                Err(deficit) => {
                    let mut saved = 0;
                    while saved < deficit.max(1) {
                        saved += shrink_one(&mut trainers, &shapes).ok_or_else(|| {
                            RandomizationError::randomization(
                                "Unable to fit the trainer data into the ROM",
                            )
                        })?;
                    }
                }
            }
        }
    }
}

fn placement(preference: &[usize]) -> Placement<'_> {
    Placement {
        preference,
        ..Placement::default()
    }
}
