import thenewpoketext.PokeTextData;
import thenewpoketext.TextToPoke;
import thenewpoketext.UnicodeParser;

import java.util.ArrayList;
import java.util.Arrays;
import java.util.Collections;
import java.util.List;
import java.util.Map;
import java.util.Random;

public final class PokeTextVectors {
    private static final int INLINE_LIMIT = 1 << 14;
    private static final int ROM_KEY = 0xD00E;

    private static final String[] FIXED_TOKENS = {
        "\\n", "\\p", "\\l", "\\and", "[PK]", "[MN]", " (224)", " (22", " (", "ᵉʳ", "ʳᵉ", "ᵉ", "ʳ",
        "'", "’", "=", "…", " ", "  ", "\\x0000", "\\x0001", "\\x01DE", "\\xFFFF", "\\xFFFE", "\\xF100",
        "\\v0100\\x0000", "\\v0101\\z0001", "\\v0102\\z0003\\z0004", "\\vFF01\\z0000\\z0000\\z0000",
        "\\z0005", "\\z0001\\z0002", "\\qrst", "\\rabc", "\\x-001", "\\x+0FF", "\\x-FFF", "\\x00ff",
        "\\x\uFF10\uFF10\uFF14\uFF21", "\\x\u0660\u0660\u0664\u0661", "\\x\u0966\uFF41\uFF46\u0F29",
        "\\v\uFF10\uFF11\uFF10\uFF12\\z\uFF41\uFF42\uFF43\uFF44", "\uD83D\uDE00", "\uD83D\uDE00x", "€",
        "ポケモン", "ぴかちゅう", "ＡＢＣ", "Ünïcödé", "\t", "\r", "~", "%", "&", "\u3000", "♂♀",
        "PkMn", "POKé", "Lv.", "No.", "\u2028", "\u0085"
    };

    private static final String[] ERROR_STRINGS = {
        "\\", "abc\\", "\\x", "\\x12", "\\xZZZZ", "\\x12G4", "\\v12", "\\vXYZW", "\\q", "\\q1", "\\z12",
        "\\z0001\\", "\\z0001\\zGGGG", "\\x-", "\\x+-12", "\\x--12", "\\x 123", "\\x12 3", "\\x\u0DE6\u0DE6\u0DE6\u0DE6",
        "\\x\uA9F0\uA9F0\uA9F0\uA9F0", "\\x\uFF27000", "ok\\an", "\\\\n"
    };

    private static final String[] LONE_SURROGATE_STRINGS = {
        "a\uD800b", "\uDC00", "\uD83D", "x\uDE00\uD83Dy", "\uD83D\\n", "\\x\uD800123", "[PK]\uDFFF[MN]"
    };

    private PokeTextVectors() {
    }

    public static Map<String, Object> vectors() {
        Random random = new Random(0x9E4);
        List<String> tableTexts = tableTexts();
        Map<String, Object> root = Json.object();
        root.put("table", table());
        root.put("hex_digits", hexDigits());
        root.put("encodes", encodes(random, tableTexts));
        root.put("decodes", decodes(random, tableTexts));
        root.put("encrypts", encrypts(random, tableTexts));
        return root;
    }

    private static List<String> tableTexts() {
        List<String> texts = new ArrayList<>();
        for (String text : UnicodeParser.tb) {
            if (text != null) {
                texts.add(text);
            }
        }
        return texts;
    }

    private static List<Integer> tableCodes() {
        List<Integer> codes = new ArrayList<>();
        for (int code = 0; code < UnicodeParser.tb.length; code++) {
            if (UnicodeParser.tb[code] != null) {
                codes.add(code);
            }
        }
        return codes;
    }

    private static Map<String, Object> table() {
        List<Object> byCode = new ArrayList<>();
        for (int code : tableCodes()) {
            byCode.add(List.of(code, units(UnicodeParser.tb[code])));
        }
        List<String> texts = new ArrayList<>(UnicodeParser.d.keySet());
        Collections.sort(texts);
        List<Object> byText = new ArrayList<>();
        for (String text : texts) {
            byText.add(List.of(units(text), UnicodeParser.d.get(text)));
        }
        Map<String, Object> table = Json.object();
        table.put("by_code", byCode);
        table.put("by_text", byText);
        return table;
    }

    private static List<Object> hexDigits() {
        List<Object> digits = new ArrayList<>();
        for (int unit = 0; unit < 0x10000; unit++) {
            int digit = Character.digit((char) unit, 16);
            if (digit >= 0) {
                digits.add(List.of(unit, digit));
            }
        }
        return digits;
    }

    private static List<Object> encodes(Random random, List<String> tableTexts) {
        List<Object> cases = new ArrayList<>();
        List<List<String>> fixed = List.of(
                List.of(), List.of(""), List.of("", ""), List.of("abcd"), List.of("abcde"),
                List.of("abcdefghij"), List.of("abcdefghi"), List.of("a"), List.of("\\x01FF"),
                List.of("\\x01FF\\x01FF\\x01FF\\x01FF\\x01FF"), List.of("\\xE000abcd"),
                List.of("Hello\\nWorld\\pBye\\l"), List.of(" (224)", " (224", "(224)"),
                Arrays.asList(FIXED_TOKENS), Arrays.asList(LONE_SURROGATE_STRINGS));
        for (List<String> strings : fixed) {
            cases.add(encodeCase(strings, false));
            cases.add(encodeCase(strings, true));
        }
        for (String text : FIXED_TOKENS) {
            cases.add(encodeCase(List.of(text), random.nextBoolean()));
        }
        for (String text : ERROR_STRINGS) {
            cases.add(encodeCase(List.of("ok", text), false));
            cases.add(encodeCase(List.of(text), true));
        }
        for (int i = 0; i < 160; i++) {
            int count = random.nextInt(5) == 0 ? 10 + random.nextInt(50) : random.nextInt(8);
            List<String> strings = new ArrayList<>();
            for (int j = 0; j < count; j++) {
                strings.add(randomString(random, tableTexts, random.nextInt(30)));
            }
            cases.add(encodeCase(strings, random.nextBoolean()));
        }
        for (int i = 0; i < 6; i++) {
            List<String> strings = new ArrayList<>();
            int count = 1 + random.nextInt(3);
            for (int j = 0; j < count; j++) {
                strings.add(randomString(random, tableTexts, 500 + random.nextInt(2500)));
            }
            cases.add(encodeCase(strings, i % 2 == 0));
        }
        List<String> many = new ArrayList<>();
        for (int j = 0; j < 300; j++) {
            many.add(randomString(random, tableTexts, random.nextInt(12)));
        }
        cases.add(encodeCase(many, false));
        cases.add(encodeCase(many, true));
        cases.add(emptyStringsCase(65535, false));
        cases.add(emptyStringsCase(65536, false));
        cases.add(emptyStringsCase(65537, true));
        return cases;
    }

    private static Map<String, Object> emptyStringsCase(int count, boolean compressed) {
        Map<String, Object> result = Json.object();
        result.put("strings", List.of());
        result.put("empty_count", count);
        putEncodings(result, Collections.nCopies(count, ""), compressed);
        return result;
    }

    private static Map<String, Object> encodeCase(List<String> strings, boolean compressed) {
        Map<String, Object> result = Json.object();
        result.put("strings", unitsList(strings));
        putEncodings(result, strings, compressed);
        return result;
    }

    private static void putEncodings(Map<String, Object> result, List<String> strings, boolean compressed) {
        result.put("compressed", compressed);
        result.put("make_file", Guard.run(() -> blob(TextToPoke.MakeFile(strings, compressed))));
        result.put("write", Guard.run(() -> blob(writeStrings(strings, compressed))));
        result.put("read_back", Guard.run(() -> decoded(writeStrings(strings, compressed))));
    }

    private static byte[] writeStrings(List<String> strings, boolean compressed) {
        byte[] raw = TextToPoke.MakeFile(strings, compressed);
        PokeTextData encrypt = new PokeTextData(raw);
        encrypt.SetKey(ROM_KEY);
        encrypt.encrypt();
        return encrypt.get();
    }

    private static List<Object> decodes(Random random, List<String> tableTexts) {
        List<Object> cases = new ArrayList<>();
        for (int i = 0; i < 40; i++) {
            byte[] file = validFile(random, tableTexts);
            int flips = 1 + random.nextInt(3);
            for (int j = 0; j < flips; j++) {
                file[random.nextInt(file.length)] ^= (byte) (1 << random.nextInt(8));
            }
            cases.add(decodeCase(file));
        }
        for (int i = 0; i < 30; i++) {
            byte[] file = validFile(random, tableTexts);
            int length = i < 6 ? i : random.nextInt(file.length);
            cases.add(decodeCase(Arrays.copyOf(file, length)));
        }
        for (int i = 0; i < 100; i++) {
            byte[] raw = crafted(random, tableTexts);
            byte[] file = raw;
            try {
                PokeTextData data = new PokeTextData(raw);
                data.encrypt();
                file = data.get();
            } catch (RuntimeException e) {
                file = raw;
            }
            cases.add(decodeCase(file));
        }
        for (int i = 0; i < 30; i++) {
            byte[] file = Bin.noise(random, random.nextInt(64));
            if (file.length >= 2) {
                Bin.writeShort(file, 0, random.nextInt(5));
            }
            cases.add(decodeCase(file));
        }
        for (int i = 0; i < 30; i++) {
            cases.add(decodeCase(mixedCompressionFile(random, tableTexts)));
        }
        return cases;
    }

    private static Map<String, Object> decodeCase(byte[] file) {
        Map<String, Object> result = Json.object();
        result.put("file", file);
        result.put("decoded", Guard.run(() -> decoded(file)));
        return result;
    }

    private static List<Object> encrypts(Random random, List<String> tableTexts) {
        List<Object> cases = new ArrayList<>();
        for (int i = 0; i < 80; i++) {
            byte[] raw = i < 4 ? Bin.noise(random, i) : crafted(random, tableTexts);
            int key = random.nextInt(3) == 0 ? ROM_KEY : random.nextInt(0x10000);
            Map<String, Object> result = Json.object();
            result.put("raw", raw);
            result.put("key", key);
            result.put("encrypted", Guard.run(() -> {
                PokeTextData data = new PokeTextData(raw);
                data.SetKey(key);
                data.encrypt();
                return blob(data.get());
            }));
            cases.add(result);
        }
        return cases;
    }

    private static byte[] validFile(Random random, List<String> tableTexts) {
        while (true) {
            List<String> strings = new ArrayList<>();
            int count = 1 + random.nextInt(6);
            for (int j = 0; j < count; j++) {
                strings.add(randomString(random, tableTexts, random.nextInt(20)));
            }
            try {
                return writeStrings(strings, random.nextBoolean());
            } catch (RuntimeException e) {
                continue;
            }
        }
    }

    private static byte[] mixedCompressionFile(Random random, List<String> tableTexts) {
        int count = 1 + random.nextInt(5);
        List<int[]> entries = new ArrayList<>();
        for (int i = 0; i < count; i++) {
            entries.add(entryWords(random, tableTexts, random.nextBoolean()));
        }
        byte[] raw = assemble(entries, random.nextInt(0x10000));
        PokeTextData data = new PokeTextData(raw);
        data.encrypt();
        return data.get();
    }

    private static int[] entryWords(Random random, List<String> tableTexts, boolean compressed) {
        while (true) {
            try {
                byte[] single = TextToPoke.MakeFile(List.of(randomString(random, tableTexts, random.nextInt(15))),
                        compressed);
                int[] words = new int[(single.length - 12) / 2];
                for (int i = 0; i < words.length; i++) {
                    words[i] = (single[12 + i * 2] & 0xFF) | ((single[13 + i * 2] & 0xFF) << 8);
                }
                return words;
            } catch (RuntimeException e) {
                continue;
            }
        }
    }

    private static byte[] assemble(List<int[]> entries, int key) {
        int base = 4 + entries.size() * 8;
        int total = base;
        for (int[] words : entries) {
            total += words.length * 2;
        }
        byte[] data = new byte[total];
        Bin.writeShort(data, 0, entries.size());
        Bin.writeShort(data, 2, key);
        int offset = base;
        for (int i = 0; i < entries.size(); i++) {
            int[] words = entries.get(i);
            Bin.writeInt(data, 4 + i * 8, offset);
            Bin.writeInt(data, 8 + i * 8, words.length);
            for (int word : words) {
                Bin.writeShort(data, offset, word);
                offset += 2;
            }
        }
        return data;
    }

    private static byte[] crafted(Random random, List<String> tableTexts) {
        int count = random.nextInt(7);
        int bodyWords = random.nextInt(48);
        int bodyStart = 4 + count * 8;
        byte[] data = new byte[bodyStart + bodyWords * 2];
        Bin.writeShort(data, 0, count);
        Bin.writeShort(data, 2, random.nextInt(0x10000));
        for (int i = 0; i < count; i++) {
            int pointer;
            int length;
            int kind = random.nextInt(24);
            if (kind < 18) {
                int startWord = random.nextInt(bodyWords + 1);
                int remaining = bodyWords - startWord;
                pointer = bodyStart + 2 * startWord;
                length = 1 + random.nextInt(Math.max(1, remaining)) + (random.nextInt(10) == 0 ? 1 : 0);
            } else if (kind == 18) {
                pointer = 2 * random.nextInt(3);
                length = 1 + random.nextInt(6);
            } else if (kind == 19) {
                pointer = random.nextInt();
                length = random.nextInt(4);
            } else if (kind < 22) {
                pointer = bodyStart;
                length = -random.nextInt(3);
            } else if (kind == 22) {
                pointer = bodyStart + 2 * random.nextInt(bodyWords + 1);
                length = random.nextBoolean() ? Integer.MAX_VALUE : 0x10000;
            } else {
                pointer = bodyStart + 1;
                length = 1 + random.nextInt(4);
            }
            Bin.writeInt(data, 4 + i * 8, pointer);
            Bin.writeInt(data, 8 + i * 8, length);
        }
        List<Integer> codes = tableCodes();
        int offset = bodyStart;
        while (offset < data.length) {
            int kind = random.nextInt(10);
            if (kind == 0 && data.length - offset >= 6) {
                int[] words = entryWords(random, tableTexts, random.nextBoolean());
                for (int k = 0; k < words.length && offset < data.length; k++) {
                    Bin.writeShort(data, offset, words[k]);
                    offset += 2;
                }
                continue;
            }
            int word;
            if (kind < 4) {
                word = codes.get(random.nextInt(codes.size()));
            } else if (kind == 4) {
                word = 0xFFFE;
            } else if (kind == 5) {
                word = 0xFFFF;
            } else if (kind == 6) {
                word = random.nextBoolean() ? 0xF100 : 0x1FF;
            } else if (kind == 7) {
                word = random.nextInt(4);
            } else {
                word = random.nextInt(0x10000);
            }
            Bin.writeShort(data, offset, word);
            offset += 2;
        }
        return data;
    }

    private static String randomString(Random random, List<String> tableTexts, int tokens) {
        StringBuilder builder = new StringBuilder();
        for (int i = 0; i < tokens; i++) {
            builder.append(randomToken(random, tableTexts));
        }
        return builder.toString();
    }

    private static String randomToken(Random random, List<String> tableTexts) {
        int kind = random.nextInt(20);
        if (kind < 6) {
            return tableTexts.get(random.nextInt(tableTexts.size()));
        } else if (kind < 10) {
            char c;
            do {
                c = (char) (0x20 + random.nextInt(0x5F));
            } while (c == '\\');
            return String.valueOf(c);
        } else if (kind < 12) {
            String hex = String.format("%04X", random.nextInt(0x10000));
            return "\\x" + (random.nextBoolean() ? hex : hex.toLowerCase());
        } else if (kind == 12) {
            StringBuilder builder = new StringBuilder(String.format("\\v%04X", random.nextInt(0x10000)));
            int arguments = random.nextInt(4);
            if (arguments == 0) {
                builder.append("\\x0000");
            }
            for (int i = 0; i < arguments; i++) {
                builder.append(String.format("\\z%04X", random.nextInt(0x10000)));
            }
            return builder.toString();
        } else if (kind < 15) {
            return FIXED_TOKENS[random.nextInt(FIXED_TOKENS.length)];
        } else if (kind < 18) {
            int[][] ranges = {{0x3040, 0x30FF}, {0x4E00, 0x4E80}, {0x00A0, 0x00FF}, {0xFF00, 0xFF60}, {0x2000, 0x2700}};
            int[] range = ranges[random.nextInt(ranges.length)];
            return String.valueOf((char) (range[0] + random.nextInt(range[1] - range[0] + 1)));
        } else if (kind == 18) {
            return new String(Character.toChars(0x10000 + random.nextInt(0x100000)));
        } else {
            return "\\n";
        }
    }

    private static Map<String, Object> blob(byte[] bytes) {
        Map<String, Object> result = Json.object();
        result.put("length", bytes.length);
        result.put("sha256", Bin.sha256(bytes));
        if (bytes.length <= INLINE_LIMIT) {
            result.put("bytes", bytes);
        }
        return result;
    }

    private static Map<String, Object> decoded(byte[] file) {
        PokeTextData data = new PokeTextData(file);
        data.decrypt();
        Map<String, Object> result = Json.object();
        result.put("strings", unitsList(data.strlist));
        result.put("compressed", data.compressFlag);
        return result;
    }

    private static List<Object> unitsList(List<String> strings) {
        List<Object> result = new ArrayList<>();
        for (String text : strings) {
            result.add(units(text));
        }
        return result;
    }

    private static byte[] units(String text) {
        byte[] bytes = new byte[text.length() * 2];
        for (int i = 0; i < text.length(); i++) {
            char c = text.charAt(i);
            bytes[i * 2] = (byte) c;
            bytes[i * 2 + 1] = (byte) (c >> 8);
        }
        return bytes;
    }
}
