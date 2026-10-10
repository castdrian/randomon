import java.util.ArrayList;
import java.util.Base64;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

final class Json {
    private Json() {
    }

    static String str(String value) {
        if (value == null) {
            return "null";
        }
        StringBuilder out = new StringBuilder("\"");
        for (int i = 0; i < value.length(); i++) {
            char c = value.charAt(i);
            if (c == '"') {
                out.append("\\\"");
            } else if (c == '\\') {
                out.append("\\\\");
            } else if (c < 0x20 || c >= 0x7F) {
                out.append(String.format("\\u%04x", (int) c));
            } else {
                out.append(c);
            }
        }
        return out.append('"').toString();
    }

    static String units(String value) {
        List<String> items = new ArrayList<>();
        for (int i = 0; i < value.length(); i++) {
            items.add(Integer.toString(value.charAt(i)));
        }
        return "[" + String.join(",", items) + "]";
    }

    static String base64(byte[] value) {
        return str(Base64.getEncoder().encodeToString(value));
    }

    static String strings(List<String> values) {
        List<String> items = new ArrayList<>();
        for (String value : values) {
            items.add(str(value));
        }
        return "[" + String.join(",", items) + "]";
    }

    static String ints(int[] values) {
        List<String> items = new ArrayList<>();
        for (int value : values) {
            items.add(Integer.toString(value));
        }
        return "[" + String.join(",", items) + "]";
    }

    static String array(List<String> rawItems) {
        return "[\n" + String.join(",\n", rawItems) + "\n]";
    }

    static Obj obj() {
        return new Obj();
    }

    static final class Obj {
        private final Map<String, String> entries = new LinkedHashMap<>();

        Obj raw(String key, String rawJson) {
            entries.put(key, rawJson);
            return this;
        }

        Obj put(String key, String value) {
            return raw(key, str(value));
        }

        Obj put(String key, boolean value) {
            return raw(key, Boolean.toString(value));
        }

        Obj put(String key, long value) {
            return raw(key, Long.toString(value));
        }

        @Override
        public String toString() {
            List<String> parts = new ArrayList<>();
            for (Map.Entry<String, String> entry : entries.entrySet()) {
                parts.add(str(entry.getKey()) + ":" + entry.getValue());
            }
            return "{" + String.join(",", parts) + "}";
        }
    }
}
