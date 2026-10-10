import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.Random;

final class RandomVectors {
    private static final int[] BOUNDS = {
        1, 2, 3, 5, 7, 10, 16, 17, 64, 100, 151, 255, 256, 386, 493, 649, 721, 807, 1000, 4096,
        1 << 30, (1 << 30) + 1, 0x55555556, Integer.MAX_VALUE - 1, Integer.MAX_VALUE
    };
    private static final int[] BYTE_LENGTHS = {0, 1, 2, 3, 4, 5, 7, 8, 9, 13, 16};

    private RandomVectors() {
    }

    static List<Long> seeds() {
        List<Long> seeds = new ArrayList<>();
        long[] fixed = {
            0L, 1L, -1L, 42L, 123456789L, 987654321987L, Long.MIN_VALUE, Long.MAX_VALUE,
            0x5DEECE66DL, 0xFFFFFFFFFFFFL, 0x1000000000000L, -0x5DEECE66DL, 281474976710655L, 31337L
        };
        for (long seed : fixed) {
            seeds.add(seed);
        }
        Random seedSource = new Random(20240601L);
        for (int i = 0; i < 10; i++) {
            seeds.add(seedSource.nextLong());
        }
        for (int i = 0; i < 6; i++) {
            long value = 0;
            byte[] bytes = new byte[6];
            seedSource.nextBytes(bytes);
            for (int b = 0; b < bytes.length; b++) {
                value |= ((long) bytes[b] & 0xffL) << (8 * b);
            }
            seeds.add(value);
        }
        return seeds;
    }

    static Map<String, Object> build() {
        List<Object> cases = new ArrayList<>();
        for (long seed : seeds()) {
            cases.add(caseFor(seed));
        }
        Map<String, Object> root = new LinkedHashMap<>();
        root.put("cases", cases);
        return root;
    }

    private static Map<String, Object> caseFor(long seed) {
        Map<String, Object> entry = new LinkedHashMap<>();
        entry.put("seed", seed);
        entry.put("next_int", nextInts(seed));
        entry.put("next_int_bounded", boundedInts(seed));
        entry.put("next_long", nextLongs(seed));
        entry.put("next_boolean", nextBooleans(seed));
        entry.put("next_float_bits", nextFloatBits(seed));
        entry.put("next_double_bits", nextDoubleBits(seed));
        entry.put("next_gaussian_bits", nextGaussianBits(seed));
        entry.put("next_bytes", nextBytes(seed));
        entry.put("mixed", mixed(seed));
        return entry;
    }

    private static List<Object> nextInts(long seed) {
        Random random = new Random(seed);
        List<Object> values = new ArrayList<>();
        for (int i = 0; i < 16; i++) {
            values.add(random.nextInt());
        }
        return values;
    }

    private static List<Object> boundedInts(long seed) {
        List<Object> perBound = new ArrayList<>();
        for (int bound : BOUNDS) {
            Random random = new Random(seed);
            List<Object> values = new ArrayList<>();
            for (int i = 0; i < 12; i++) {
                values.add(random.nextInt(bound));
            }
            Map<String, Object> item = new LinkedHashMap<>();
            item.put("bound", bound);
            item.put("values", values);
            item.put("next_int_after", random.nextInt());
            perBound.add(item);
        }
        return perBound;
    }

    private static List<Object> nextLongs(long seed) {
        Random random = new Random(seed);
        List<Object> values = new ArrayList<>();
        for (int i = 0; i < 8; i++) {
            values.add(random.nextLong());
        }
        return values;
    }

    private static List<Object> nextBooleans(long seed) {
        Random random = new Random(seed);
        List<Object> values = new ArrayList<>();
        for (int i = 0; i < 16; i++) {
            values.add(random.nextBoolean());
        }
        return values;
    }

    private static List<Object> nextFloatBits(long seed) {
        Random random = new Random(seed);
        List<Object> values = new ArrayList<>();
        for (int i = 0; i < 8; i++) {
            values.add(Float.floatToRawIntBits(random.nextFloat()));
        }
        return values;
    }

    private static List<Object> nextDoubleBits(long seed) {
        Random random = new Random(seed);
        List<Object> values = new ArrayList<>();
        for (int i = 0; i < 8; i++) {
            values.add(Double.doubleToRawLongBits(random.nextDouble()));
        }
        return values;
    }

    private static List<Object> nextGaussianBits(long seed) {
        Random random = new Random(seed);
        List<Object> values = new ArrayList<>();
        for (int i = 0; i < 25; i++) {
            values.add(Double.doubleToRawLongBits(random.nextGaussian()));
        }
        return values;
    }

    private static List<Object> nextBytes(long seed) {
        Random random = new Random(seed);
        List<Object> calls = new ArrayList<>();
        for (int length : BYTE_LENGTHS) {
            byte[] bytes = new byte[length];
            random.nextBytes(bytes);
            List<Object> values = new ArrayList<>();
            for (byte b : bytes) {
                values.add((int) b);
            }
            calls.add(values);
        }
        return calls;
    }

    private static List<Object> mixed(long seed) {
        Random script = new Random(seed ^ 0x9E3779B97F4A7C15L);
        Random random = new Random(seed);
        List<Object> steps = new ArrayList<>();
        for (int i = 0; i < 60; i++) {
            int op = script.nextInt(10);
            List<Object> step = new ArrayList<>();
            switch (op) {
                case 0:
                    step.add("next_int");
                    step.add(random.nextInt());
                    break;
                case 1: {
                    int bound = BOUNDS[script.nextInt(BOUNDS.length)];
                    step.add("next_int_bounded");
                    step.add(bound);
                    step.add(random.nextInt(bound));
                    break;
                }
                case 2:
                    step.add("next_long");
                    step.add(random.nextLong());
                    break;
                case 3:
                    step.add("next_boolean");
                    step.add(random.nextBoolean());
                    break;
                case 4:
                    step.add("next_float_bits");
                    step.add(Float.floatToRawIntBits(random.nextFloat()));
                    break;
                case 5:
                    step.add("next_double_bits");
                    step.add(Double.doubleToRawLongBits(random.nextDouble()));
                    break;
                case 6:
                case 7:
                    step.add("next_gaussian_bits");
                    step.add(Double.doubleToRawLongBits(random.nextGaussian()));
                    break;
                case 8: {
                    int length = script.nextInt(10);
                    byte[] bytes = new byte[length];
                    random.nextBytes(bytes);
                    List<Object> values = new ArrayList<>();
                    for (byte b : bytes) {
                        values.add((int) b);
                    }
                    step.add("next_bytes");
                    step.add(length);
                    step.add(values);
                    break;
                }
                default: {
                    long newSeed = script.nextLong();
                    random.setSeed(newSeed);
                    step.add("set_seed");
                    step.add(newSeed);
                    break;
                }
            }
            steps.add(step);
        }
        return steps;
    }
}
