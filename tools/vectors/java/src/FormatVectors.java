import java.util.ArrayList;
import java.util.Arrays;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.Random;

final class FormatVectors {
    private static final String[] FLOAT_FORMATS = {
        "%.0f", "%.00f", "%.1f", "%.2f", "%.3f", "%f", "%10.2f", "%-10.1f|", "%010.3f", "%,.2f", "%+.1f",
        "%(.2f", "% .2f", "%#.0f", "%.12f", "%.17f"
    };
    private static final String[] INT_FORMATS = {
        "%d", "%3d", "%-3d|", "%-2d|", "%03d", "%02d", "%04d", "%5d", "%-5d|", "%05d", "%,d", "%+d", "%(d",
        "% d", "%,015d", "%x", "%X", "%08X", "%02X", "%08x", "%#x", "%#010X", "%o", "%s", "%10s|", "%c"
    };
    private static final String[] STRING_FORMATS = {"%s", "%-15s|", "%15s|", "%-10s|", "%.3s", "%10.3s|", "%S", "%-11s|"};

    private FormatVectors() {
    }

    private static List<Object> arg(String type, Object value) {
        return new ArrayList<>(Arrays.asList(type, value));
    }

    private static Object actual(List<Object> argument) {
        String type = (String) argument.get(0);
        Object value = argument.get(1);
        switch (type) {
            case "int":
                return ((Number) value).intValue();
            case "long":
                return ((Number) value).longValue();
            case "short":
                return ((Number) value).shortValue();
            case "byte":
                return ((Number) value).byteValue();
            case "double_bits":
                return Double.longBitsToDouble(((Number) value).longValue());
            case "float_bits":
                return Float.intBitsToFloat(((Number) value).intValue());
            case "char":
                return ((String) value).charAt(0);
            case "bool":
                return value;
            case "null":
                return null;
            default:
                return value;
        }
    }

    private static boolean addCase(List<Object> cases, String format, List<List<Object>> args) {
        Object[] actualArgs = new Object[args.size()];
        for (int i = 0; i < args.size(); i++) {
            Object value = actual(args.get(i));
            actualArgs[i] = value;
            if (format.contains("s") && printsDifferentlySinceJdk19(value)) {
                return false;
            }
        }
        String expected;
        try {
            expected = String.format(Locale.US, format, actualArgs);
        } catch (RuntimeException e) {
            return false;
        }
        Map<String, Object> item = new LinkedHashMap<>();
        item.put("format", format);
        item.put("args", new ArrayList<Object>(args));
        item.put("expected", expected);
        cases.add(item);
        return true;
    }

    private static boolean printsDifferentlySinceJdk19(Object value) {
        if (value instanceof Double) {
            return !NumberVectors.jdk8DoubleToString((Double) value).equals(Double.toString((Double) value));
        }
        if (value instanceof Float) {
            return !NumberVectors.jdk8FloatToString((Float) value).equals(Float.toString((Float) value));
        }
        return false;
    }

    static Map<String, Object> build() {
        List<Object> cases = new ArrayList<>();
        addUprCases(cases);
        addFloatCases(cases);
        addIntCases(cases);
        addStringCases(cases);
        addMiscCases(cases);
        Map<String, Object> root = new LinkedHashMap<>();
        root.put("cases", cases);
        return root;
    }

    private static void addUprCases(List<Object> cases) {
        addCase(cases, "%-10s -> %-10s", Arrays.asList(arg("string", "Tackle"), arg("string", "Hyper Beam")));
        addCase(cases, "TM%02d %s", Arrays.asList(arg("int", 7), arg("string", "Hail")));
        addCase(cases, "%03d %s", Arrays.asList(arg("int", 29), arg("string", "Nidoran♀")));
        addCase(cases, "HP   %-3d", Arrays.asList(arg("int", 45)));
        addCase(cases, "%.00f%% accuracy", Arrays.asList(arg("double_bits", Double.doubleToRawLongBits(99.6078431372549))));
        addCase(cases, "%.00f%% accuracy", Arrays.asList(arg("double_bits", Double.doubleToRawLongBits(50.5))));
        addCase(cases, "%-15s -> %-15s", Arrays.asList(arg("string", "Flabébé"), arg("string", "Floette")));
        addCase(cases, "%3d|%-10s|%-17s|%4d|%4d|%4d|%4d|%4d", Arrays.asList(arg("int", 1), arg("string", "Bulbasaur"),
            arg("string", "GRASS/POISON"), arg("int", 45), arg("int", 49), arg("int", 49), arg("int", 45), arg("int", 65)));
        addCase(cases, "|TM%02d %12s ", Arrays.asList(arg("int", 3), arg("string", "Water Pulse")));
        addCase(cases, "| %16s ", Arrays.asList(arg("string", "-")));
        addCase(cases, "%3d|%-15s|%-8s|%5d|%4d|%3d", Arrays.asList(arg("int", 33), arg("string", "Tackle"),
            arg("string", "NORMAL"), arg("int", 40), arg("int", 100), arg("int", 35)));
        addCase(cases, "- %5s", Arrays.asList(arg("string", "Potion")));
        addCase(cases, "%d%%: %s", Arrays.asList(arg("int", 30), arg("string", "Oran Berry, Sitrus Berry")));
        addCase(cases, "@%X", Arrays.asList(arg("int", 0x3A2B1C)));
        addCase(cases, "%08X", Arrays.asList(arg("long", 0xDEADBEEFL)));
        addCase(cases, "%08X", Arrays.asList(arg("int", -559038737)));
        addCase(cases, "overlay_%04d", Arrays.asList(arg("int", 36)));
        addCase(cases, "Trade %-11s -> %-11s the %-11s        ->      %-11s -> %-15s the %s", Arrays.asList(
            arg("string", "Abra"), arg("string", "Mr. Mime"), arg("string", "MARCEL"), arg("string", "Pikachu"),
            arg("string", "Kadabra"), arg("string", "Bob")));
        addCase(cases, "Level %s-%s", Arrays.asList(arg("int", 5), arg("int", 50)));
        addCase(cases, "(%s => %s)", Arrays.asList(arg("string", "BROCK"), arg("string", "GARY")));
    }

    private static void addFloatCases(List<Object> cases) {
        List<Double> samples = new ArrayList<>(NumberVectors.sampleDoubles().subList(0, 120));
        for (int hitratio = 0; hitratio <= 255; hitratio += 3) {
            samples.add(hitratio / 255.0 * 100.0);
        }
        Random random = new Random(31415L);
        for (int i = 0; i < 300; i++) {
            samples.add((random.nextInt(200001) - 100000) / Math.pow(10, random.nextInt(6)));
        }
        for (int i = 0; i < 150; i++) {
            samples.add(random.nextInt(100000) / 1000.0 + 0.0005);
        }
        int formatIndex = 0;
        for (double value : samples) {
            if (Math.abs(value) > 1e30 && !Double.isInfinite(value)) {
                continue;
            }
            for (int repeat = 0; repeat < 3; repeat++) {
                String format = FLOAT_FORMATS[formatIndex++ % FLOAT_FORMATS.length];
                addCase(cases, format, Arrays.asList(arg("double_bits", Double.doubleToRawLongBits(value))));
            }
        }
        for (float value : NumberVectors.sampleFloats().subList(0, 60)) {
            addCase(cases, "%.2f", Arrays.asList(arg("float_bits", Float.floatToRawIntBits(value))));
        }
    }

    private static void addIntCases(List<Object> cases) {
        int[] ints = {0, 1, -1, 7, 42, -42, 999, 1000, -1000, 123456789, Integer.MAX_VALUE, Integer.MIN_VALUE, 65, 0x2640};
        long[] longs = {0L, -1L, 1234567890123L, Long.MIN_VALUE, Long.MAX_VALUE};
        for (String format : INT_FORMATS) {
            for (int value : ints) {
                addCase(cases, format, Arrays.asList(arg("int", value)));
            }
            for (long value : longs) {
                addCase(cases, format, Arrays.asList(arg("long", value)));
            }
            addCase(cases, format, Arrays.asList(arg("byte", -2)));
            addCase(cases, format, Arrays.asList(arg("short", -300)));
        }
    }

    private static void addStringCases(List<Object> cases) {
        String[] strings = {"", "a", "Bulbasaur", "Nidoran♂", "Flabébé", "😀 smile", "Type: Null", "ho-oh"};
        for (String format : STRING_FORMATS) {
            for (String value : strings) {
                addCase(cases, format, Arrays.asList(arg("string", value)));
            }
            addCase(cases, format, Arrays.asList(arg("null", null)));
            addCase(cases, format, Arrays.asList(arg("bool", true)));
            addCase(cases, format, Arrays.asList(arg("double_bits", Double.doubleToRawLongBits(1.5))));
            addCase(cases, format, Arrays.asList(arg("float_bits", Float.floatToRawIntBits(0.1f))));
            addCase(cases, format, Arrays.asList(arg("char", "x")));
        }
    }

    private static void addMiscCases(List<Object> cases) {
        addCase(cases, "%1$s %1$s %2$s", Arrays.asList(arg("string", "a"), arg("string", "b")));
        addCase(cases, "%s %<s %s", Arrays.asList(arg("string", "a"), arg("string", "b")));
        addCase(cases, "%b %b %b %B", Arrays.asList(arg("bool", false), arg("null", null), arg("int", 0), arg("bool", true)));
        addCase(cases, "%c%c%c", Arrays.asList(arg("char", "z"), arg("int", 0x41), arg("int", 0x1F600)));
        addCase(cases, "100%% done%n", new ArrayList<>());
        addCase(cases, "%5%|%-5%|", new ArrayList<>());
        addCase(cases, "%h", Arrays.asList(arg("string", "Bulbasaur")));
        addCase(cases, "[%8.3f] [%-8.3f] [%08.3f]", Arrays.asList(arg("double_bits", Double.doubleToRawLongBits(-3.14159)),
            arg("double_bits", Double.doubleToRawLongBits(2.71828)), arg("double_bits", Double.doubleToRawLongBits(-1.0005))));
        addCase(cases, "%.2f %.2f %.1f", Arrays.asList(arg("double_bits", Double.doubleToRawLongBits(-0.0)),
            arg("double_bits", Double.doubleToRawLongBits(-0.001)), arg("double_bits", Double.doubleToRawLongBits(0.05))));
        addCase(cases, "%f %f %f", Arrays.asList(arg("double_bits", Double.doubleToRawLongBits(Double.NaN)),
            arg("double_bits", Double.doubleToRawLongBits(Double.NEGATIVE_INFINITY)),
            arg("double_bits", Double.doubleToRawLongBits(Double.POSITIVE_INFINITY))));
        addCase(cases, "%10f|%-10f|%(f", Arrays.asList(arg("double_bits", Double.doubleToRawLongBits(Double.NaN)),
            arg("double_bits", Double.doubleToRawLongBits(Double.NEGATIVE_INFINITY)),
            arg("double_bits", Double.doubleToRawLongBits(Double.NEGATIVE_INFINITY))));
    }
}
