import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.Random;

import jdk.internal.math.FloatingDecimal;

final class NumberVectors {
    private NumberVectors() {
    }

    static String jdk8DoubleToString(double value) {
        return FloatingDecimal.toJavaFormatString(value);
    }

    static String jdk8FloatToString(float value) {
        return FloatingDecimal.toJavaFormatString(value);
    }

    static List<Double> sampleDoubles() {
        List<Double> values = new ArrayList<>();
        double[] fixed = {
            0.0, -0.0, 1.0, -1.0, 0.1, 0.2, 0.3, 0.5, 1.5, 2.5, 100.0, 99.6078431372549, 1e-3, 9.999e-4,
            1e-4, 1e7, 9999999.0, 9999999.999, 1e7 + 0.5, 1e8, 1e16, 1e21, 1e22, 1e23, 2e23, 5e-324,
            Double.MIN_NORMAL, Double.MAX_VALUE, Math.PI, Math.E, 2.82879384806159E17, 1.18575755E-316,
            Double.NaN, Double.POSITIVE_INFINITY, Double.NEGATIVE_INFINITY, 0.001, 0.0011, 123456.789,
            4.35, 0.15, 1.005, 2.675, 1.0 / 3.0, 2.0 / 3.0, 255.0 / 2.55, 100.0 / 255.0 * 100.0
        };
        for (double value : fixed) {
            values.add(value);
        }
        for (int exponent = -30; exponent <= 30; exponent++) {
            values.add(Math.pow(10, exponent));
            values.add(Math.pow(2, exponent * 3));
        }
        for (int hitratio = 0; hitratio <= 255; hitratio++) {
            values.add(hitratio / 255.0 * 100.0);
            values.add(hitratio * 100.0 / 255.0);
        }
        Random random = new Random(424242L);
        for (int i = 0; i < 600; i++) {
            values.add(Double.longBitsToDouble(random.nextLong()));
        }
        for (int i = 0; i < 400; i++) {
            values.add(random.nextInt(2000000) / Math.pow(10, random.nextInt(9)));
        }
        for (int i = 0; i < 300; i++) {
            values.add(random.nextDouble() * Math.pow(10, random.nextInt(40) - 20));
        }
        for (int i = 0; i < 100; i++) {
            values.add(Double.longBitsToDouble(random.nextLong() & 0x000FFFFFFFFFFFFFL));
        }
        return values;
    }

    static List<Float> sampleFloats() {
        List<Float> values = new ArrayList<>();
        float[] fixed = {
            0.0f, -0.0f, 1.0f, 0.1f, 0.5f, 1.0e23f, 2.0e-44f, 1.4e-45f, 3.4028235e38f, 1.0e-36f, 8.41e21f,
            3.3554432e7f, 1e7f, 9999999f, 1e-3f, 0.001f, Float.NaN, Float.POSITIVE_INFINITY,
            Float.NEGATIVE_INFINITY, 1.17549435E-38f, 16777216f, 16777217f, 0.3f, 2.5f
        };
        for (float value : fixed) {
            values.add(value);
        }
        Random random = new Random(171717L);
        for (int i = 0; i < 600; i++) {
            values.add(Float.intBitsToFloat(random.nextInt()));
        }
        for (int i = 0; i < 300; i++) {
            values.add((float) (random.nextInt(2000000) / Math.pow(10, random.nextInt(9))));
        }
        for (int i = 0; i < 100; i++) {
            values.add(Float.intBitsToFloat(random.nextInt() & 0x007FFFFF));
        }
        return values;
    }

    static Map<String, Object> buildDoubleToString() {
        List<Object> cases = new ArrayList<>();
        int differsFromJdk19 = 0;
        for (double value : sampleDoubles()) {
            List<Object> item = new ArrayList<>();
            String jdk8 = jdk8DoubleToString(value);
            item.add(Double.doubleToRawLongBits(value));
            item.add(jdk8);
            if (!jdk8.equals(Double.toString(value))) {
                differsFromJdk19++;
                item.add(Double.toString(value));
            }
            cases.add(item);
        }
        List<Object> floatCases = new ArrayList<>();
        int floatsDiffer = 0;
        for (float value : sampleFloats()) {
            List<Object> item = new ArrayList<>();
            String jdk8 = jdk8FloatToString(value);
            item.add(Float.floatToRawIntBits(value));
            item.add(jdk8);
            if (!jdk8.equals(Float.toString(value))) {
                floatsDiffer++;
                item.add(Float.toString(value));
            }
            floatCases.add(item);
        }
        Map<String, Object> root = new LinkedHashMap<>();
        root.put("doubles_differing_from_jdk19_plus", differsFromJdk19);
        root.put("floats_differing_from_jdk19_plus", floatsDiffer);
        root.put("doubles", cases);
        root.put("floats", floatCases);
        return root;
    }

    static Map<String, Object> buildRound() {
        List<Object> doubles = new ArrayList<>();
        List<Double> samples = sampleDoubles();
        double[] edges = {
            0.49999999999999994, -0.49999999999999994, 0.5, -0.5, 1.5, -1.5, 2.5, -2.5, 4503599627370495.5,
            4503599627370496.0, 9.223372036854776E18, -9.223372036854776E18, 1e19, -1e19
        };
        for (double edge : edges) {
            samples.add(edge);
        }
        for (double value : samples) {
            List<Object> item = new ArrayList<>();
            item.add(Double.doubleToRawLongBits(value));
            item.add(Math.round(value));
            item.add((int) value);
            item.add((long) value);
            doubles.add(item);
        }
        List<Object> floats = new ArrayList<>();
        List<Float> floatSamples = sampleFloats();
        float[] floatEdges = {0.49999997f, -0.49999997f, 0.5f, -0.5f, 2.5f, -2.5f, 8388607.5f, 3e9f, -3e9f};
        for (float edge : floatEdges) {
            floatSamples.add(edge);
        }
        for (float value : floatSamples) {
            List<Object> item = new ArrayList<>();
            item.add(Float.floatToRawIntBits(value));
            item.add(Math.round(value));
            item.add((int) value);
            item.add((long) value);
            floats.add(item);
        }
        Map<String, Object> root = new LinkedHashMap<>();
        root.put("doubles", doubles);
        root.put("floats", floats);
        return root;
    }

    static Map<String, Object> buildStrictMath() {
        List<Object> logs = new ArrayList<>();
        List<Double> inputs = new ArrayList<>();
        double[] fixed = {0.0, -0.0, 1.0, -1.0, 2.0, 0.5, Math.E, 1e-310, 4.9e-324, Double.MAX_VALUE,
            Double.NaN, Double.POSITIVE_INFINITY, Double.NEGATIVE_INFINITY, 1.0000001, 0.9999999,
            1.0 + Math.ulp(1.0), 1.0 - Math.ulp(1.0) / 2};
        for (double value : fixed) {
            inputs.add(value);
        }
        Random random = new Random(55555L);
        for (int i = 0; i < 1500; i++) {
            inputs.add(random.nextDouble());
        }
        for (int i = 0; i < 500; i++) {
            inputs.add(Math.abs(Double.longBitsToDouble(random.nextLong())));
        }
        for (int i = 0; i < 300; i++) {
            inputs.add(1.0 + (random.nextDouble() - 0.5) * 1e-5);
        }
        for (double input : inputs) {
            List<Object> item = new ArrayList<>();
            item.add(Double.doubleToRawLongBits(input));
            item.add(Double.doubleToRawLongBits(StrictMath.log(input)));
            logs.add(item);
        }
        Map<String, Object> root = new LinkedHashMap<>();
        root.put("log", logs);
        return root;
    }

    static Map<String, Object> buildHashCodes() {
        List<Object> strings = new ArrayList<>();
        String[] fixed = {
            "", "a", "Aa", "BB", "Bulbasaur", "Nidoran♀", "Flabébé", "😀", "￿￿",
            "polygenelubricants", "The quick brown fox jumps over the lazy dog", "Pokémon", "ピカチュウ"
        };
        for (String value : fixed) {
            strings.add(stringCase(value));
        }
        Random random = new Random(8080L);
        for (int i = 0; i < 200; i++) {
            int length = random.nextInt(24);
            StringBuilder sb = new StringBuilder();
            for (int c = 0; c < length; c++) {
                int kind = random.nextInt(10);
                if (kind < 7) {
                    sb.append((char) (32 + random.nextInt(95)));
                } else if (kind < 9) {
                    sb.append((char) (0xA0 + random.nextInt(0xD000)));
                } else {
                    sb.appendCodePoint(0x10000 + random.nextInt(0x10000));
                }
            }
            strings.add(stringCase(sb.toString()));
        }
        List<Object> longs = new ArrayList<>();
        for (int i = 0; i < 100; i++) {
            long value = i < 4 ? new long[] {0L, -1L, Long.MIN_VALUE, Long.MAX_VALUE}[i] : random.nextLong();
            List<Object> item = new ArrayList<>();
            item.add(value);
            item.add(Long.hashCode(value));
            longs.add(item);
        }
        List<Object> compares = new ArrayList<>();
        for (int i = 0; i + 1 < strings.size(); i += 2) {
            String left = (String) ((List<?>) strings.get(i)).get(0);
            String right = (String) ((List<?>) strings.get(i + 1)).get(0);
            List<Object> item = new ArrayList<>();
            item.add(left);
            item.add(right);
            item.add(Integer.signum(left.compareTo(right)));
            compares.add(item);
        }
        Map<String, Object> root = new LinkedHashMap<>();
        root.put("strings", strings);
        root.put("longs", longs);
        root.put("string_compare_signum", compares);
        return root;
    }

    private static List<Object> stringCase(String value) {
        List<Object> item = new ArrayList<>();
        item.add(value);
        item.add(value.hashCode());
        return item;
    }
}
