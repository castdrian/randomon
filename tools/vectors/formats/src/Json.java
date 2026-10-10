import java.util.Base64;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

public final class Json {
    private Json() {
    }

    public static Map<String, Object> object() {
        return new LinkedHashMap<>();
    }

    public static String write(Object value) {
        StringBuilder builder = new StringBuilder();
        append(builder, value, 0);
        builder.append('\n');
        return builder.toString();
    }

    private static void append(StringBuilder builder, Object value, int depth) {
        if (value == null) {
            builder.append("null");
        } else if (value instanceof String text) {
            appendString(builder, text);
        } else if (value instanceof byte[] bytes) {
            appendString(builder, Base64.getEncoder().encodeToString(bytes));
        } else if (value instanceof Boolean || value instanceof Integer || value instanceof Long) {
            builder.append(value);
        } else if (value instanceof Map<?, ?> map) {
            appendMap(builder, map, depth);
        } else if (value instanceof List<?> list) {
            appendList(builder, list, depth);
        } else {
            throw new IllegalArgumentException("unsupported JSON value " + value.getClass());
        }
    }

    private static void appendMap(StringBuilder builder, Map<?, ?> map, int depth) {
        builder.append('{');
        boolean first = true;
        for (Map.Entry<?, ?> entry : map.entrySet()) {
            if (!first) {
                builder.append(',');
            }
            first = false;
            newline(builder, depth + 1);
            appendString(builder, entry.getKey().toString());
            builder.append(':');
            append(builder, entry.getValue(), depth + 1);
        }
        if (!map.isEmpty()) {
            newline(builder, depth);
        }
        builder.append('}');
    }

    private static void appendList(StringBuilder builder, List<?> list, int depth) {
        builder.append('[');
        boolean first = true;
        for (Object item : list) {
            if (!first) {
                builder.append(',');
            }
            first = false;
            if (depth < 2) {
                newline(builder, depth + 1);
            }
            append(builder, item, depth + 1);
        }
        if (!list.isEmpty() && depth < 2) {
            newline(builder, depth);
        }
        builder.append(']');
    }

    private static void newline(StringBuilder builder, int depth) {
        builder.append('\n');
        for (int i = 0; i < depth; i++) {
            builder.append(' ');
        }
    }

    private static void appendString(StringBuilder builder, String text) {
        builder.append('"');
        for (int i = 0; i < text.length(); i++) {
            char c = text.charAt(i);
            switch (c) {
                case '"' -> builder.append("\\\"");
                case '\\' -> builder.append("\\\\");
                case '\n' -> builder.append("\\n");
                case '\r' -> builder.append("\\r");
                case '\t' -> builder.append("\\t");
                default -> {
                    if (c < 0x20 || c > 0x7E) {
                        builder.append(String.format("\\u%04x", (int) c));
                    } else {
                        builder.append(c);
                    }
                }
            }
        }
        builder.append('"');
    }
}
