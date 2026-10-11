# Engine parity notes

Decisions taken while porting `Randomizer.java`, `AbstractRomHandler.java` and the model package from UPR ZX 4.6.1 (`upstream/`, commit `7f00eb8`).

## Object identity

- Pokémon, moves, evolutions and mega evolutions live in arenas inside `randomon_data::GameData`. They are referenced by `PokemonId`, `MoveId`, `EvolutionId` and `MegaEvolutionId`. For Pokémon and moves the arena index is the species or move number, so ordering ids gives the same order as `Pokemon.compareTo` and `TreeMap<Pokemon, …>`, and `PokemonId::java_hash_code` returns `31 + number`, like `Pokemon.hashCode`.
- `Pokemon.evolutionsFrom` and `evolutionsTo` are list ids into a shared list arena. That keeps `copyBaseFormeEvolutions` behaving like Java, where two Pokémon share the same list object.
- `Evolution` equality follows `Evolution.equals` (from, to and type). `List.remove(tempEvo)` and `contains` use that equality. Temporary evolutions reuse one arena slot.
- Getters that build fresh objects in Java (`getTrainers`, `getEncounters`, `getMovesLearnt`, `getTMHMCompatibility`, `getStaticPokemon`, `getIngameTrades`) return owned values. Every caller writes them back with the matching setter, as upstream does.
- `getAllowedItems` and `getNonBadItems` return `&mut ItemList`, because later generations mutate the shared static list (`randomizeShopItems` bans items in place and later steps see the ban).

## Nondeterministic upstream collections

- `createSetsOfMoves` sums `avgTypePower` over a `HashMap<Type, …>` whose order depends on identity hashes. Randomon sums in `Type` ordinal order. The sum can differ from a given JVM run in its last bit, which could move the `minAvg`/`maxAvg` thresholds in rare cases. No difference showed up in the Gen 1 grid.
- `Effectiveness.against` returns a `HashMap<Type, …>`. Randomon returns the entries in ordinal order. In UPR this order only matters for Gen 4–7 `getSensibleHeldItemsFor`.

## Logging

- The GUI calls `romHandler.setLog(log)` before randomizing, so handler messages such as `--Updating Type Effectiveness--` appear in the log. The CLI does not. `RandomizationOptions::handler_logging` selects the behaviour and defaults to the GUI's.
- `Time elapsed: …ms` is real wall-clock time on both sides. The parity harness masks that one line.
- The settings line uses the checksum of the default `customnames.rncn`, like `Settings.toString`. A different checksum can be passed through `RandomizationOptions::custom_names_checksum`.

## Upstream failures that Randomon fixes

Byte parity is no longer a goal. Where UPR crashes, hangs or runs out of memory, Randomon now produces a sensible result instead:

- Gen 1 and Gen 2 trainer data that grows (extra Pokémon, held items, custom moves) is repacked into the bank's original trainer extents plus known free space (Gen 1 shares bank space with movesets and evolutions; Gen 2 uses the free tail of the trainer bank). Classes that still fit stay in place. If the data cannot fit, added Pokémon are trimmed first (regular trainers before important ones and bosses), then custom moves and held items that were not originally present. Trainer slots that other tables point into (Yellow's rival starter) are kept in place.
- Gen 1 has no held items, double battles or per-trainer moves, and Gen 2 has no double battles, so `tweak_for_rom` turns those options off for those games. Better trainer movesets uses Gen 1's special stat when the ratio is computed.
- Evolution walks (`final_evos`, relatives, pre-evolution chains, illegal chains, copy-up) are cycle-safe, so "random every level" works. Random evolutions relax force-change, the stage limit and then the growth curve before giving a Pokémon no evolution, and Pokémon outside the original pool no longer raise `originalEvos.get`.
- Similar-strength searches widen in bounded steps until the whole pool is covered, then fall back. Every random retry loop is bounded, and empty pools fall back to the full pool or keep the original Pokémon.
- Handler getters that fail record a deferred error that the randomizer reports before saving, instead of panicking.

## What every generation handler implements

The trait in `handler.rs` splits into two parts.

- **Generation-agnostic default methods.** These are UPR's shared randomization logic in `engine/*`: stats, types, abilities, evolutions, starters, wild Pokémon (including the ORAS DexNav collapse), trainers, trainer movesets with `MoveSynergy`, move data, movesets, TMs and tutors, statics and totems, names, items, shops, pickup items, trades and misc-tweak ordering.
- **Required methods.** These are the generation-specific ROM access: loading and saving, the Pokémon, move, trainer, encounter, static, trade, TM/HM, tutor, item, shop and text getters and setters, evolution clean-up (`remove_impossible_evolutions`, `make_evolutions_easier`, `remove_time_based_evolutions`), intro Pokémon, misc tweaks and diagnostics. A handler overrides a default method wherever the upstream generation handler overrides the `AbstractRomHandler` method. Examples are `type_in_game`, `has_rival_final_battle`, `get_game_breaking_moves`, `misc_tweaks_available` and `apply_misc_tweak`.

## 3DS (Gen 6 and Gen 7)

- `getSensibleHeldItemsFor` walks `Effectiveness.against`, a `HashMap<Type, …>` keyed by identity-hashed enums, so the order of weakness berries in the candidate list differs between JVM runs. Randomon walks the types in ordinal order. The parity grids leave "sensible held items for trainers" off; every other trainer held item option is covered.
- 3DS handlers read the ROM lazily from a file (`RomHandlerFactory::create_from_file`), the same way UPR's `NCCH` reads through a `RandomAccessFile`. Nothing reads a whole multi-gigabyte image into memory. A full `.cxi` save streams through `RomHandler::save_rom_to_path`, and LayeredFS output (`RomOutput::Directory`) lists the changed RomFS files and `code.bin` under the title ID, plus a `<titleId>/romfs/` entry for the folder UPR always creates. `write_directory_output` purges the title folder first, like `NCCH.saveAsLayeredFS`.
- A game update is loaded after the base ROM and the handler reloads all of its data, like `loadGameUpdate` calling `loadedROM` again. Files the update contains are read from the update, writes go to the base ROM and to the update copy, and LayeredFS output comes from the base ROM. The CLI forces save-as-directory when `-u` is given, as `CliRandomizer` does.
- N3DS text (`randomon_formats::ctr::text`) is ported with UPR's quirks: unknown text variables are printed as hex but parsed back as decimal, and when an entry cannot be encoded (`IOException`) the whole text file is left unchanged, as `setStrings` catches and prints the exception.
- UPR's `NCCH` extracts files into `tmp_<rom name>` next to the jar, so two UPR processes on the same ROM share and corrupt each other's files. The parity harness gives every Java worker its own copy of the compiled classes, which moves that folder, and runs one JVM per job. Gen 7 needs `-Xmx4g`; with 2 GB UPR runs out of heap while rebuilding the encounter GARC.
- Edits to RomFS files that exist only in a loaded game update are written to the LayeredFS output too.
