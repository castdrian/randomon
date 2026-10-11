import pptxt.PPTxtHandler;

import java.io.ByteArrayOutputStream;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.Random;

public final class PpTxtVectors {
    private static final String[] TOKENS = {
        "Pokémon", "×", "÷", "…", "♂", "♀", "★", "♪", "\\[angry]",
        "\\[:)]", "\\[zzz2]", "\\[PK]", "\\[MN]", "\\xF000", "\\xBD02", "\\x0000", "\\xFFFE", "\\x00E9",
        "Hello", " ", "!", "?", "123", "ポケモン", "{", "\\a{", "$1", "\u0015", "￱"
    };

    private PpTxtVectors() {
    }

    public static Map<String, Object> vectors() {
        Random random = new Random(0x5A7);
        List<Object> cases = new ArrayList<>();
        for (int index = 0; index < 120; index++) {
            cases.add(caseFor(random, index));
        }
        Map<String, Object> root = Json.object();
        root.put("cases", cases);
        return root;
    }

    private static byte[] units(String text) {
        return text.getBytes(StandardCharsets.UTF_16LE);
    }

    private static List<Object> unitList(List<String> strings) {
        List<Object> list = new ArrayList<>();
        for (String string : strings) {
            list.add(units(string));
        }
        return list;
    }

    private static String randomText(Random random) {
        StringBuilder builder = new StringBuilder();
        int tokens = random.nextInt(8);
        for (int token = 0; token < tokens; token++) {
            builder.append(TOKENS[random.nextInt(TOKENS.length)]);
        }
        return builder.toString();
    }

    private static List<Integer> plainWords(Random random) {
        List<Integer> words = new ArrayList<>();
        int kind = random.nextInt(4);
        int length = random.nextInt(12);
        for (int word = 0; word < length; word++) {
            if (kind == 0) {
                int value = random.nextInt(0x10000);
                words.add(Character.isSurrogate((char) value) ? value - 0x1000 : value);
            } else if (kind == 1) {
                words.add(0x20 + random.nextInt(0x5F));
            } else if (kind == 2) {
                int[] specials = {0xF000, 0xFFFE, 0x2467, 0x21D2, 0x2486, 0x00E9, 0x0001, 0x0014, 0xFFF0, 0xFFF1};
                words.add(specials[random.nextInt(specials.length)]);
            } else {
                words.add(0x3041 + random.nextInt(0x50));
            }
        }
        words.add(0xFFFF);
        return words;
    }

    private static void writeWord(ByteArrayOutputStream out, int value) {
        out.write(value & 0xFF);
        out.write((value >> 8) & 0xFF);
    }

    private static void writeLong(ByteArrayOutputStream out, int value) {
        writeWord(out, value & 0xFFFF);
        writeWord(out, (value >>> 16) & 0xFFFF);
    }

    private static byte[] buildFile(Random random, int entries, boolean secondSection) {
        List<List<Integer>> encrypted = new ArrayList<>();
        int dataSize = 0;
        for (int entry = 0; entry < entries; entry++) {
            List<Integer> plain = plainWords(random);
            int key = random.nextInt(0x10000);
            List<Integer> words = new ArrayList<>();
            for (int value : plain) {
                words.add((value ^ key) & 0xFFFF);
                key = ((key << 3) | (key >>> 13)) & 0xFFFF;
            }
            encrypted.add(words);
            dataSize += words.size() * 2;
        }
        int sectionOffset = secondSection ? 0x14 : 0x10;
        int tableSize = 4 + 8 * entries;
        int sectionSize = tableSize + dataSize;
        ByteArrayOutputStream out = new ByteArrayOutputStream();
        writeWord(out, secondSection ? 2 : 1);
        writeWord(out, entries);
        writeLong(out, sectionSize);
        writeLong(out, 0);
        writeLong(out, sectionOffset);
        if (secondSection) {
            writeLong(out, sectionOffset + sectionSize);
        }
        writeLong(out, sectionSize);
        int offset = tableSize;
        for (List<Integer> words : encrypted) {
            writeLong(out, offset);
            writeWord(out, words.size());
            writeWord(out, random.nextInt(0x10000));
            offset += words.size() * 2;
        }
        for (List<Integer> words : encrypted) {
            for (int word : words) {
                writeWord(out, word);
            }
        }
        if (secondSection) {
            writeLong(out, 8);
            writeLong(out, random.nextInt());
        }
        return out.toByteArray();
    }

    private static Map<String, Object> save(byte[] file, List<String> strings) {
        Map<String, Object> outcome = Json.object();
        try {
            outcome.put("bytes", PPTxtHandler.saveEntry(file, strings));
        } catch (RuntimeException error) {
            outcome.put("error", error.getClass().getName());
        }
        return outcome;
    }

    private static Map<String, Object> caseFor(Random random, int index) {
        byte[] file = buildFile(random, 1 + random.nextInt(9), index % 5 == 4);
        List<String> decoded = PPTxtHandler.readTexts(file);
        List<String> changed = new ArrayList<>();
        for (String text : decoded) {
            changed.add(random.nextInt(3) == 0 ? text : randomText(random));
        }
        if (index % 7 == 6) {
            changed.add(randomText(random));
        }
        List<String> changedCopy = new ArrayList<>(changed);
        Map<String, Object> result = Json.object();
        result.put("file", file);
        result.put("decoded", unitList(decoded));
        result.put("same", save(file, new ArrayList<>(decoded)));
        result.put("changed", unitList(changedCopy));
        result.put("changed_output", save(file, changed));
        return result;
    }
}
