import java.util.List;
import java.util.Map;

final class Json {
    private Json() {
    }

    static String write(Object value) {
        StringBuilder out = new StringBuilder();
        append(out, value, 0);
        out.append('\n');
        return out.toString();
    }

    private static void append(StringBuilder out, Object value, int depth) {
        if (value == null) {
            out.append("null");
        } else if (value instanceof String) {
            appendString(out, (String) value);
        } else if (value instanceof Boolean || value instanceof Integer || value instanceof Long
                || value instanceof Short || value instanceof Byte) {
            out.append(value);
        } else if (value instanceof Map) {
            appendObject(out, (Map<?, ?>) value, depth);
        } else if (value instanceof List) {
            appendArray(out, (List<?>) value, depth);
        } else {
            throw new IllegalArgumentException("unsupported json value " + value.getClass());
        }
    }

    private static boolean isScalarList(List<?> list) {
        for (Object item : list) {
            if (item instanceof Map || item instanceof List) {
                return false;
            }
        }
        return true;
    }

    private static void appendArray(StringBuilder out, List<?> list, int depth) {
        if (isScalarList(list)) {
            out.append('[');
            for (int i = 0; i < list.size(); i++) {
                if (i > 0) {
                    out.append(',');
                }
                append(out, list.get(i), depth + 1);
            }
            out.append(']');
            return;
        }
        out.append("[\n");
        for (int i = 0; i < list.size(); i++) {
            indent(out, depth + 1);
            append(out, list.get(i), depth + 1);
            out.append(i + 1 < list.size() ? ",\n" : "\n");
        }
        indent(out, depth);
        out.append(']');
    }

    private static void appendObject(StringBuilder out, Map<?, ?> map, int depth) {
        out.append("{\n");
        int i = 0;
        for (Map.Entry<?, ?> entry : map.entrySet()) {
            indent(out, depth + 1);
            appendString(out, entry.getKey().toString());
            out.append(": ");
            append(out, entry.getValue(), depth + 1);
            out.append(++i < map.size() ? ",\n" : "\n");
        }
        indent(out, depth);
        out.append('}');
    }

    private static void indent(StringBuilder out, int depth) {
        for (int i = 0; i < depth; i++) {
            out.append(' ');
        }
    }

    private static void appendString(StringBuilder out, String value) {
        out.append('"');
        for (int i = 0; i < value.length(); i++) {
            char c = value.charAt(i);
            if (c == '"' || c == '\\') {
                out.append('\\').append(c);
            } else if (c >= 0x20 && c < 0x7F) {
                out.append(c);
            } else {
                out.append(String.format("\\u%04x", (int) c));
            }
        }
        out.append('"');
    }
}
