# Java iteration order and number formatting in UPR ZX 4.6.1

This note records which `HashMap`/`HashSet` instances in upstream UPR ZX 4.6.1 (`upstream/src`, commit `7f00eb8`) are iterated in a way that can change the output ROM or the log. It also explains what `randomon_java` emulates, and which Java runtime behaviour it targets. UPR 4.6.1 is built for and normally run on Java 8, so JDK 8 is the reference everywhere below.

## Key types and their hash codes

| Key type | `hashCode` | Deterministic across runs? |
|---|---|---|
| `Integer`, `Short`, `Byte`, `Character` | the value | yes |
| `Long` | `(int)(v ^ (v >>> 32))` | yes |
| `String` | `31*h + c` over UTF-16 code units | yes |
| `Boolean` | 1231 / 1237 | yes |
| `Pokemon` | `31 + number` (`Comparable` by `number`) | yes |
| `Trainer` | `31 + index` | yes |
| `Evolution` | from `from.number`, `to.number`, `type.ordinal()` | yes |
| `AbstractRomHandler.EvolutionPair` | from both `Pokemon` hashes | yes |
| `EvolutionUpdate` | not overridden (identity) | no |
| `Move` | not overridden (identity) | no |
| every enum (`Type`, `MoveCategory`, `EvolutionType`, ...) | `Object.hashCode` (identity) | no |

HotSpot hands out identity hashes from a thread-local xorshift generator. Their values depend on the JVM build, how many threads started first, and every earlier identity-hash request on that thread. A GUI run and a CLI run of the same seed can therefore differ, and so can two machines. This cannot be emulated, so any upstream collection whose order rests on an identity hash is nondeterministic in UPR itself.

`EnumMap` and `EnumSet` iterate in ordinal order. `TreeMap` and `TreeSet` iterate in sorted order. `LinkedHashMap` and `LinkedHashSet` iterate in insertion order. All of these are deterministic. `JavaEnumMap` and `JavaEnumSet` cover the first two; the port should use `BTreeMap` for the sorted ones and `indexmap` for the linked ones.

## Collections whose iteration order affects output

| Location | Collection | Key type | How order reaches the output | Deterministic? |
|---|---|---|---|---|
| `romhandlers/Gen4RomHandler.java:5806`, `Gen5RomHandler.java:4301`, `Gen6RomHandler.java:4227`, `Gen7RomHandler.java:3775` (`getSensibleHeldItemsFor`) | `byType`, the `HashMap` returned by `pokemon/Effectiveness.java:57` (`against`) | `Type` | The `entrySet()` order decides the order of weakness-reducing berries in the candidate list. `AbstractRomHandler.java:2041` then picks from that list with `random.nextInt(size)`, so the order changes which item is held, the ROM bytes and the log. This happens whenever "sensible held items" is on and the Pokémon has two or more weaknesses. | **no** (identity hash) |
| `romhandlers/AbstractRomHandler.java:3754` (`createSetsOfMoves`) | `validTypeMoves` (`HashMap`, filled at 3430 and 3610) | `Type` | Its `keySet()` loop sums `totalAvgPower` as a `double`. Adding in a different order can change the last bit of the sum, and that sum sets the `minAvg`/`maxAvg` thresholds used by the RNG loops that follow. This is rare but possible. | **no** (identity hash) |
| `Gen4RomHandler.java:4284` and `4418`, `Gen5RomHandler.java:3119` and `3232`, `Gen6RomHandler.java:3099` and `3213`, `Gen7RomHandler.java:2735` and `2869` (`removeImpossibleEvolutions`, `removeTimeBasedEvolutions`) | `extraEvolutions` (`HashSet`, reused with `clear()`, so capacity stays 16 or larger) | `Evolution` | `pkmn.evolutionsFrom.addAll(extraEvolutions)` and the loop into `ev.to.evolutionsTo` set the order of the evolution lists. Those lists are later written to the evolution NARC/GARC and the log. This matters when one species gains two or more extra evolutions. | yes, with exact `HashSet` emulation. Hashes exceed 65535, so the `h ^ (h >>> 16)` spread matters. |
| `Gen7RomHandler.java:1587` (`getAreaNameFromZones`) | `uniqueZoneNames` (`HashSet`) | `String` | `String.join(" / ", set)` becomes the area name. That name goes into `EncounterSet.displayName` and the log. | yes, with exact `HashSet` emulation |

## Iterated, but the order does not change output

Each of these is deterministic when emulated exactly. A faithful port should still use `JavaHashMap`/`JavaHashSet` for them, because doing so is cheap and keeps the code close to upstream.

| Location | Collection | Key type | Why the order is harmless |
|---|---|---|---|
| `AbstractRomHandler.java:55` | `placementHistory` | `Pokemon` | Only used for integer averages and sums, for `contains`, and for stdout debug output. |
| `AbstractRomHandler.java:56` | `itemPlacementHistory` | `Integer` | Same as above. |
| `AbstractRomHandler.java:4086`, iterated in Gen3 3557, Gen4 5589/5607 and Gen5 1999/2012 (`applyCorrectStaticMusic`) | `specialMusicStaticChanges` | `Integer` | Each key rewrites only its own slots, and slots already written are skipped. One exception needs checking during the Gen 5 port: the BW2 Kyurem branch also writes `i-0xFE` to `i-0xF8`, and if that range overlaps another key's slot the order would matter. An exact `JavaHashMap` removes the risk. |
| `AbstractRomHandler.java:5587` (`relatedPokemon`) | `related` | `Pokemon` | First-match `break`, but the result is only "does any element exceed the limit". |
| `Gen6RomHandler.java:83`, `Gen7RomHandler.java:84` | `RomEntry.linkedStaticOffsets` | `Integer` | Keys and values are unique and each base gets exactly one link. |
| `Gen4RomHandler.java:3798` | `textFiles` | `Integer` | Each element edits a separate text file, and the edit is idempotent. |
| `Gen3RomHandler.java:1695` | `battleTrappers` | `Pokemon` | Only `addAll`-ed into `EncounterSet.bannedPokemon`, which is itself only used for membership and `removeAll`. |
| `pptxt/PPTxtHandler.java:22` | `pokeToText` key set | `String` | Joined into a regex alternation. No token is a prefix of another, so the order cannot change any match. |
| `newnds/NDSRom.java`, `ctr/NCCH.java` diagnostics maps | `files`, `fileDiagnostics`, ... | `String`, `Integer` | Only iterated for diagnostics, and sorted before printing. ROM, NARC, GARC and RomFS writing all go through `TreeMap`s or index loops. |
| `pokemon/MoveSynergy.java:1040` | `Effectiveness.against` result | `Type` | Streamed into a set that is only used for `contains`. |

Every other `HashMap`/`HashSet` in upstream is only used for lookups (`get`, `containsKey`, `contains`), for `size()`, or as the argument to `List.removeAll`/`retainAll`, which keep the list's own order. That includes all the static tables in `constants/*.java`, the `RomEntry` ini maps, the text tables, the CRC32 maps, the `Move` and `Type` sets in the TM and tutor code, and `EncounterSet.bannedPokemon`. Every `getMovesLearnt`, `getEggMoves`, `getTMHMCompatibility`, `getMoveTutorCompatibility` and `getShopItems` return value is a `TreeMap`. The `EvolutionUpdate` collections are `TreeSet`s. One more caution: `Gen3RomHandler.setEggMoves` (Gen3 2263) writes in `keySet()` order of whatever map it is given. Callers pass `TreeMap`s today, so the port must keep those maps sorted.

### What the port should do with the nondeterministic cases

Java itself cannot reproduce these two cases, so no implementation can match "UPR's output" there. The natural choice is to iterate in `Type` ordinal order, which is also insertion order. Use `JavaEnumMap`, or a `Vec` indexed by ordinal. Parity tests should avoid settings that hit `getSensibleHeldItemsFor` on Gen 4–7 when a Pokémon has several weaknesses, or should accept a diff there. The `createSetsOfMoves` sum only differs in rare last-bit cases.

## What `JavaHashMap` emulates

`JavaHashMap<K, V>` and `JavaHashSet<E>` copy JDK 8's `java.util.HashMap`:

- **Hashing and sizing:** the `h ^ (h >>> 16)` spread, power-of-two tables allocated lazily on first insert, the 0.75 load factor, `tableSizeFor`, the `threshold` arithmetic in `resize()` including the `HashMap(0)`/`HashMap(1)` edge cases, and `clear()` keeping the table length.
- **Bin order:** tail insertion for `put`/`putIfAbsent`, and head insertion for `computeIfAbsent`/`compute`/`merge`. Those three also check for a resize *before* inserting instead of after, and treeify one node earlier.
- **Resizing:** the lo/hi split that keeps order on resize.
- **Tree bins:** a bin with 9 or more entries becomes a tree bin once the table has at least 64 slots. Tree bins keep a linked `next` order, and that order changes: `treeify`, `putTreeVal` and `removeTreeNode` move the red-black root to the front of the bin, and `putTreeVal` links a new node right after its tree parent. To reproduce this exactly, the crate ports `TreeNode` (rotations, `balanceInsertion`, `balanceDeletion`, `split`, `untreeify`). Ties between keys with equal hashes are broken by `JavaHash::java_compare_to` (`Comparable`), which is how `String` collisions are ordered. For keys that are not comparable, Java falls back to `System.identityHashCode`, which is nondeterministic. The crate always picks "left" in that case. None of the upstream key types reach that fallback, because equal hashes imply equal keys for `Integer`, `Pokemon` and `Trainer`, and `String` is comparable.

### JDK 8 behaviour that differs from the JDK 20 used to generate the vectors

The vectors come from JDK 20, which is the installed JDK. Where JDK 20's `HashMap` differs from JDK 8's, the Java harness spells out the JDK 8 algorithm explicitly, and the Rust code follows JDK 8:

1. **`new HashMap<>(map)`, `putAll` into an empty map, and `HashMap.clone()`:** JDK 8 sizes the table with `tableSizeFor((int)((float)s / 0.75f + 1.0f))`. JDK 19 and later use `ceil(s / 0.75)`, which produces a smaller table in some cases. For example, 12 entries give a 32-slot table on JDK 8 and 16 slots on JDK 19+. Use `from_entries_like_copy_constructor`, `put_all` and `java_clone` for these.
2. **`new HashSet<>(collection)`:** JDK 8 uses `new HashMap<>(Math.max((int)(n / .75f) + 1, 16))`. JDK 19 and later use `HashMap.newHashMap(Math.max(n, 12))`. Use `JavaHashSet::from_collection`.
3. **`Iterator.remove` in a tree bin:** JDK 8 untreeifies a tree bin that has become small, even during iterator removal. JDK 10 and later skip that step when `movable == false` (JDK-8186171). `retain`, `remove_via_iterator`, `remove_all_list`/`remove_all_set` (the iterate-and-remove branch) and `retain_all` use the JDK 8 behaviour. The vectors only test iterator removal when no tree bins are present, because JDK 20 cannot produce JDK 8 reference output for that case. No upstream collection that is iterated in an order-sensitive way ever gets a tree bin.

`AbstractSet.removeAll` is reproduced with its size heuristic. When the set is larger than the argument, it calls `remove` for each argument element. Otherwise it iterates the set with `Iterator.remove`.

## Number formatting

UPR 4.6.1 targets Java 8, so `randomon_java` implements JDK 8 behaviour, not the shortest-digit `Double.toString` that JDK 19 introduced (Raffaello Giulietti, JDK-4511638).

- **`Double.toString`/`Float.toString`** (`double_to_string`, `float_to_string`) port JDK 8's `FloatingDecimal.BinaryToASCIIBuffer.dtoa`: the `developLongDigits` fast path, `estimateDecExp`, the int, long and big-integer digit loops with their asymmetric `high` tests, the "at least two digits in E-form" hack, the final round-up, and the `toJavaFormatString` layout. The output is often not the shortest. For example, JDK 8 prints `2e23` as `1.9999999999999998E23`, `1e23` as `9.999999999999999E22`, and `2.82879384806159E17` as `2.82879384806159008E17`. JDK 19+ prints `2.0E23`, `1.0E23` and `2.82879384806159E17`. In the vector sample, 22 doubles and 71 floats print differently between the two algorithms.
- **`String.format("%.Nf")`** (`java_format`, `format_fixed`) ports JDK 8's `Formatter` together with `FormattedFloatingDecimal`. It takes the digits from the same `dtoa` in non-compatible mode, which always produces at least two digits, and then rounds those *decimal digits* HALF_UP (`applyPrecision`). It does not round the exact binary value. That is why Java prints `%.2f` of `1.005` as `1.01` and `%.0f` of `0.5` as `1`, where C would give `1.00` and `0`. `-0.0` and small negative values keep their sign (`-0.00`). JDK 21 changed `Formatter` to the new algorithm (JDK-8300869), but JDK 20 still uses the JDK 8 path.
- **Other conversions:** `%d`, `%s`, `%x`/`%X`, `%o`, `%c`, `%b`, `%h`, `%%` and `%n` are implemented with the flags `-`, `0`, `+`, space, `,`, `(` and `#`, and with explicit (`1$`) and relative (`<`) argument indices. Padding counts UTF-16 code units, as Java does, so names like `Nidoran♀` and `Flabébé` line up the way they do in UPR's log. Negative `byte`, `short`, `int` and `long` values are printed in `%x` as two's complement at their own width. `%s` applied to a `double` or `float` uses the JDK 8 `toString` above. `%e`, `%g`, `%a` and `%t` are rejected, because UPR does not use them.
- **Locale:** Java formats with the default locale. The crate always formats as `en-US`: ASCII digits, `.` as the decimal separator and `,` for grouping. UPR's log only uses `%.00f`, which prints no decimal separator, so only a locale with non-ASCII digits could make a difference.
- **`Math.round(double)`/`Math.round(float)`** (`math::round_double`, `math::round_float`) port the JDK 8 bit-twiddling version, so `Math.round(0.49999999999999994)` is `0`. JDK 7 computed `floor(x + 0.5)` instead. Rust's `as` casts already match Java's `(int)` and `(long)` casts, which saturate and send NaN to 0.
- **`StrictMath.log`** is a port of fdlibm `__ieee754_log`. It is checked bit for bit against 2,300 inputs. `StrictMath.sqrt` is IEEE-exact, which is the same as Rust's `f64::sqrt`. `next_gaussian` uses both functions, so it matches bit for bit on every platform.

### How the reference output was produced

Only JDK 20 is installed. JDK 20 still contains the pre-JDK 19 algorithm as `jdk.internal.math.FloatingDecimal.toJavaFormatString`, which is the JDK 8 `dtoa`, including the `isCompatibleFormat` flag. Its `Formatter` still calls `FormattedFloatingDecimal` with that legacy `dtoa`. The harness calls `FloatingDecimal` through `--add-exports java.base/jdk.internal.math=ALL-UNNAMED`. It records `Double.toString` as well wherever the result differs, and it leaves out of the `%s` format vectors any value whose output differs between JDK 8 and JDK 20. One assumption remains: that JDK 20's legacy `FloatingDecimal` and `FormattedFloatingDecimal` are unchanged from JDK 8u. Both were rewritten for JDK 8 (JDK-7032154 and JDK-7131459) and look unchanged since then, but no JDK 8 runtime was available to confirm it. Rerunning the Java harness on a JDK 9–20 runtime regenerates every vector. It cannot run on JDK 8 itself, because there the class is `sun.misc.FloatingDecimal`.
