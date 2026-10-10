import java.lang.reflect.Field;
import java.util.ArrayList;
import java.util.Collections;
import java.util.HashMap;
import java.util.HashSet;
import java.util.Iterator;
import java.util.LinkedHashMap;
import java.util.LinkedList;
import java.util.List;
import java.util.Map;
import java.util.Random;

final class CollectionVectors {
    private static final int[] SHUFFLE_SIZES = {0, 1, 2, 3, 4, 5, 6, 10, 31, 64, 100, 151, 500};
    private static final Field TABLE_FIELD = tableField();

    private CollectionVectors() {
    }

    private static Field tableField() {
        try {
            Field field = HashMap.class.getDeclaredField("table");
            field.setAccessible(true);
            return field;
        } catch (ReflectiveOperationException e) {
            throw new IllegalStateException(e);
        }
    }

    private static Object[] table(HashMap<?, ?> map) {
        try {
            return (Object[]) TABLE_FIELD.get(map);
        } catch (IllegalAccessException e) {
            throw new IllegalStateException(e);
        }
    }

    static int capacity(HashMap<?, ?> map) {
        Object[] table = table(map);
        return table == null ? 0 : table.length;
    }

    static boolean hasTreeBins(HashMap<?, ?> map) {
        Object[] table = table(map);
        if (table == null) {
            return false;
        }
        for (Object bin : table) {
            if (bin != null && bin.getClass().getName().endsWith("$TreeNode")) {
                return true;
            }
        }
        return false;
    }

    static Map<String, Object> buildShuffle() {
        List<Object> cases = new ArrayList<>();
        for (long seed : RandomVectors.seeds().subList(0, 12)) {
            for (int size : SHUFFLE_SIZES) {
                List<Integer> arrayList = sequence(size);
                List<Integer> linkedList = new LinkedList<>(sequence(size));
                Random arrayRandom = new Random(seed);
                Random linkedRandom = new Random(seed);
                Collections.shuffle(arrayList, arrayRandom);
                Collections.shuffle(linkedList, linkedRandom);
                int arrayNext = arrayRandom.nextInt();
                int linkedNext = linkedRandom.nextInt();
                if (!arrayList.equals(linkedList) || arrayNext != linkedNext) {
                    throw new IllegalStateException("RandomAccess and sequential shuffles diverged");
                }
                Map<String, Object> entry = new LinkedHashMap<>();
                entry.put("seed", seed);
                entry.put("size", size);
                entry.put("result", new ArrayList<Object>(arrayList));
                entry.put("next_int_after", arrayNext);
                cases.add(entry);
            }
        }
        Map<String, Object> root = new LinkedHashMap<>();
        root.put("cases", cases);
        return root;
    }

    private static List<Integer> sequence(int size) {
        List<Integer> list = new ArrayList<>();
        for (int i = 0; i < size; i++) {
            list.add(i);
        }
        return list;
    }

    interface KeySource<K> {
        K next(Random random);

        Object toJson(K key);
    }

    static final class MapScenario<K> {
        private final String name;
        private final String kind;
        private HashMap<K, Integer> map;
        private final KeySource<K> keys;
        private final List<Object> ops = new ArrayList<>();
        private final List<Object> snapshots = new ArrayList<>();

        MapScenario(String name, String kind, HashMap<K, Integer> initial, Object init, KeySource<K> keys) {
            this.name = name;
            this.kind = kind;
            this.map = initial;
            this.keys = keys;
            List<Object> initOp = new ArrayList<>();
            initOp.add("init");
            initOp.add(init);
            ops.add(initOp);
        }

        private void record(Object... parts) {
            List<Object> op = new ArrayList<>();
            Collections.addAll(op, parts);
            ops.add(op);
        }

        void put(K key, int value) {
            map.put(key, value);
            record("put", keys.toJson(key), value);
        }

        void remove(K key) {
            map.remove(key);
            record("remove", keys.toJson(key));
        }

        void computeIfAbsent(K key, int value) {
            map.computeIfAbsent(key, k -> value);
            record("compute_if_absent", keys.toJson(key), value);
        }

        void merge(K key, int value) {
            map.merge(key, value, Integer::sum);
            record("merge_sum", keys.toJson(key), value);
        }

        void putIfAbsent(K key, int value) {
            map.putIfAbsent(key, value);
            record("put_if_absent", keys.toJson(key), value);
        }

        void computeIncrementOrRemove(K key) {
            map.compute(key, (k, old) -> incrementOrRemove(old));
            record("compute_increment_or_remove", keys.toJson(key));
        }

        private static Integer incrementOrRemove(Integer old) {
            if (old == null) {
                return 1;
            }
            if (old % 2 == 0) {
                return null;
            }
            return old + 1;
        }

        boolean retainNotDivisibleByThree() {
            if (hasTreeBins(map)) {
                return false;
            }
            Iterator<Map.Entry<K, Integer>> iterator = map.entrySet().iterator();
            while (iterator.hasNext()) {
                if (iterator.next().getValue() % 3 == 0) {
                    iterator.remove();
                }
            }
            record("retain_not_divisible_by_three");
            return true;
        }

        void clear() {
            map.clear();
            record("clear");
        }

        void jdk8Copy() {
            int capacity = (int) ((float) map.size() / 0.75f + 1.0f);
            HashMap<K, Integer> copy = map.isEmpty() ? new HashMap<>() : new HashMap<>(capacity);
            for (Map.Entry<K, Integer> entry : map.entrySet()) {
                copy.put(entry.getKey(), entry.getValue());
            }
            map = copy;
            record("jdk8_copy");
        }

        void snapshot() {
            List<Object> keyList = new ArrayList<>();
            List<Object> valueList = new ArrayList<>();
            for (Map.Entry<K, Integer> entry : map.entrySet()) {
                keyList.add(keys.toJson(entry.getKey()));
                valueList.add(entry.getValue());
            }
            Map<String, Object> snapshot = new LinkedHashMap<>();
            snapshot.put("op_index", ops.size());
            snapshot.put("capacity", capacity(map));
            snapshot.put("tree_bins", hasTreeBins(map));
            snapshot.put("keys", keyList);
            snapshot.put("values", valueList);
            snapshots.add(snapshot);
            record("snapshot");
        }

        Map<String, Object> toJson() {
            Map<String, Object> json = new LinkedHashMap<>();
            json.put("name", name);
            json.put("kind", kind);
            json.put("ops", ops);
            json.put("snapshots", snapshots);
            return json;
        }

        void runRandomScript(Random script, int length, int snapshotEvery) {
            for (int i = 0; i < length; i++) {
                int roll = script.nextInt(100);
                K key = keys.next(script);
                int value = script.nextInt(1000);
                if (roll < 50) {
                    put(key, value);
                } else if (roll < 66) {
                    remove(key);
                } else if (roll < 72) {
                    computeIfAbsent(key, value);
                } else if (roll < 78) {
                    merge(key, value);
                } else if (roll < 84) {
                    putIfAbsent(key, value);
                } else if (roll < 88) {
                    computeIncrementOrRemove(key);
                } else if (roll < 90) {
                    retainNotDivisibleByThree();
                } else if (roll < 92) {
                    jdk8Copy();
                } else if (roll < 93) {
                    clear();
                } else {
                    put(key, value);
                }
                if ((i + 1) % snapshotEvery == 0) {
                    snapshot();
                }
            }
            snapshot();
        }
    }

    static final class IntKeys implements KeySource<Integer> {
        private final int mode;

        IntKeys(int mode) {
            this.mode = mode;
        }

        public Integer next(Random random) {
            switch (mode) {
                case 0:
                    return random.nextInt(60);
                case 1:
                    return random.nextInt();
                case 2:
                    return random.nextInt(48) * 64;
                case 3:
                    return random.nextInt(40) * 1024 + random.nextInt(2) * 65536;
                case 4:
                    return (random.nextInt(30) << 16) | random.nextInt(4);
                case 5:
                    return random.nextInt(2000) - 1000;
                default:
                    return 31 * (31 * (31 + random.nextInt(800)) + random.nextInt(800)) + random.nextInt(12);
            }
        }

        public Object toJson(Integer key) {
            return key;
        }
    }

    static final class StringKeys implements KeySource<String> {
        private static final String[] NAMES = {
            "Bulbasaur", "Nidoran♀", "Nidoran♂", "Farfetch'd", "Mr. Mime", "Flabébé",
            "Type: Null", "Ho-Oh", "Porygon-Z", "Jangmo-o", "Kommo-o", "😀", "￿", "",
            "", "a", "Aa", "BB", "AaAa", "BBBB", "AaBB", "BBAa", "Route 1", "Route 10", "Viridian Forest",
            "Mt. Moon", "Ten Carat Hill", "Poni Plains", "Akala Outskirts", "Route 16"
        };
        private final int mode;

        StringKeys(int mode) {
            this.mode = mode;
        }

        public String next(Random random) {
            switch (mode) {
                case 0:
                    return NAMES[random.nextInt(NAMES.length)];
                case 1:
                    return collidingString(random, 2 + random.nextInt(3));
                case 2: {
                    int length = random.nextInt(6);
                    StringBuilder sb = new StringBuilder();
                    for (int i = 0; i < length; i++) {
                        sb.append((char) ('a' + random.nextInt(4)));
                    }
                    return sb.toString();
                }
                default:
                    return "key" + random.nextInt(300);
            }
        }

        private static String collidingString(Random random, int blocks) {
            StringBuilder sb = new StringBuilder();
            for (int i = 0; i < blocks; i++) {
                sb.append(random.nextBoolean() ? "Aa" : "BB");
            }
            return sb.toString();
        }

        public Object toJson(String key) {
            return key;
        }
    }

    private static <K> HashMap<K, Integer> initialMap(Object init) {
        if (init instanceof Integer) {
            return new HashMap<>((Integer) init);
        }
        return new HashMap<>();
    }

    static Map<String, Object> buildHashMap() {
        List<Object> scenarios = new ArrayList<>();
        Object[] inits = {"default", 0, 1, 3, 16, 100};
        for (int mode = 0; mode <= 6; mode++) {
            for (int variant = 0; variant < 3; variant++) {
                Object init = inits[(mode + variant) % inits.length];
                MapScenario<Integer> scenario = new MapScenario<>(
                    "int_mode" + mode + "_variant" + variant, "int", initialMap(init), init, new IntKeys(mode));
                scenario.runRandomScript(new Random(1000L * mode + variant), 260, 40);
                scenarios.add(scenario.toJson());
            }
        }
        for (int mode = 0; mode <= 3; mode++) {
            for (int variant = 0; variant < 3; variant++) {
                Object init = inits[(mode + variant) % inits.length];
                MapScenario<String> scenario = new MapScenario<>(
                    "string_mode" + mode + "_variant" + variant, "string", initialMap(init), init,
                    new StringKeys(mode));
                scenario.runRandomScript(new Random(5000L * mode + variant), 220, 40);
                scenarios.add(scenario.toJson());
            }
        }
        scenarios.add(treeifyScenario());
        scenarios.add(sequentialScenario());
        scenarios.add(collidingStringTreeScenario());
        Map<String, Object> root = new LinkedHashMap<>();
        root.put("scenarios", scenarios);
        return root;
    }

    private static Map<String, Object> treeifyScenario() {
        MapScenario<Integer> scenario = new MapScenario<>("treeify_multiples_of_64", "int",
            new HashMap<>(), "default", new IntKeys(2));
        for (int i = 0; i < 40; i++) {
            scenario.put(i * 64, i);
            if (i % 4 == 3) {
                scenario.snapshot();
            }
        }
        for (int i = 0; i < 40; i += 3) {
            scenario.remove(i * 64);
            scenario.snapshot();
        }
        for (int i = 0; i < 200; i++) {
            scenario.put(i * 64 + 1, i);
        }
        scenario.snapshot();
        for (int i = 0; i < 200; i += 2) {
            scenario.remove(i * 64 + 1);
        }
        scenario.snapshot();
        for (int i = 300; i < 340; i++) {
            scenario.computeIfAbsent(i * 128, i);
        }
        scenario.snapshot();
        return scenario.toJson();
    }

    private static Map<String, Object> sequentialScenario() {
        MapScenario<Integer> scenario = new MapScenario<>("sequential_with_high_bits", "int",
            new HashMap<>(), "default", new IntKeys(1));
        for (int i = 0; i < 120; i++) {
            scenario.put(i * 0x10001, i);
        }
        scenario.snapshot();
        for (int i = 0; i < 120; i++) {
            scenario.put(-i * 7919, i);
        }
        scenario.snapshot();
        scenario.clear();
        scenario.snapshot();
        for (int i = 0; i < 30; i++) {
            scenario.put(i * 31, i);
        }
        scenario.snapshot();
        return scenario.toJson();
    }

    private static Map<String, Object> collidingStringTreeScenario() {
        MapScenario<String> scenario = new MapScenario<>("colliding_strings_tree", "string",
            new HashMap<>(), "default", new StringKeys(1));
        List<String> family = new ArrayList<>();
        for (int mask = 0; mask < 64; mask++) {
            StringBuilder sb = new StringBuilder();
            for (int bit = 5; bit >= 0; bit--) {
                sb.append((mask >> bit & 1) == 0 ? "Aa" : "BB");
            }
            family.add(sb.toString());
        }
        Collections.shuffle(family, new Random(77));
        for (int i = 0; i < 40; i++) {
            scenario.put("filler" + i, -i);
        }
        for (int i = 0; i < family.size(); i++) {
            scenario.put(family.get(i), i);
            if (i % 8 == 7) {
                scenario.snapshot();
            }
        }
        for (int i = 0; i < family.size(); i += 3) {
            scenario.remove(family.get(i));
        }
        scenario.snapshot();
        for (int i = 0; i < 80; i++) {
            scenario.put("more" + i, i);
        }
        scenario.snapshot();
        return scenario.toJson();
    }

    static Map<String, Object> buildHashSet() {
        List<Object> scenarios = new ArrayList<>();
        for (int variant = 0; variant < 6; variant++) {
            scenarios.add(setScenario(variant));
        }
        Map<String, Object> root = new LinkedHashMap<>();
        root.put("scenarios", scenarios);
        return root;
    }

    private static Map<String, Object> setScenario(int variant) {
        Random script = new Random(9000 + variant);
        IntKeys keys = new IntKeys(variant % 2 == 0 ? 6 : 0);
        HashSet<Integer> set = new HashSet<>();
        List<Object> ops = new ArrayList<>();
        List<Object> snapshots = new ArrayList<>();
        for (int i = 0; i < 160; i++) {
            int roll = script.nextInt(100);
            List<Object> op = new ArrayList<>();
            if (roll < 60) {
                int key = keys.next(script);
                set.add(key);
                op.add("add");
                op.add(key);
            } else if (roll < 75) {
                int key = keys.next(script);
                set.remove(key);
                op.add("remove");
                op.add(key);
            } else if (roll < 82) {
                List<Integer> others = randomKeys(script, keys, script.nextInt(set.size() + 3));
                jdk8RemoveAll(set, others);
                op.add("remove_all_list");
                op.add(new ArrayList<Object>(others));
            } else if (roll < 88) {
                List<Integer> elements = new ArrayList<>(set);
                elements.addAll(randomKeys(script, keys, script.nextInt(6)));
                Collections.shuffle(elements, script);
                set = new HashSet<>(Math.max((int) (elements.size() / .75f) + 1, 16));
                set.addAll(elements);
                op.add("from_collection");
                op.add(new ArrayList<Object>(elements));
            } else {
                int key = keys.next(script);
                set.add(key);
                op.add("add");
                op.add(key);
            }
            ops.add(op);
            if ((i + 1) % 20 == 0) {
                Map<String, Object> snapshot = new LinkedHashMap<>();
                snapshot.put("op_index", ops.size());
                snapshot.put("elements", new ArrayList<Object>(set));
                snapshots.add(snapshot);
                List<Object> marker = new ArrayList<>();
                marker.add("snapshot");
                ops.add(marker);
            }
        }
        Map<String, Object> json = new LinkedHashMap<>();
        json.put("name", "set_variant" + variant);
        json.put("ops", ops);
        json.put("snapshots", snapshots);
        return json;
    }

    private static List<Integer> randomKeys(Random script, IntKeys keys, int count) {
        List<Integer> result = new ArrayList<>();
        for (int i = 0; i < count; i++) {
            result.add(keys.next(script));
        }
        return result;
    }

    private static void jdk8RemoveAll(HashSet<Integer> set, List<Integer> others) {
        if (set.size() > others.size()) {
            for (Integer other : others) {
                set.remove(other);
            }
        } else {
            set.removeIf(others::contains);
        }
    }
}
