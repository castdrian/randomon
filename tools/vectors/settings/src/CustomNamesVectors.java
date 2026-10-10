import com.dabomstew.pkrandom.CustomNamesSet;
import com.dabomstew.pkrandom.FileFunctions;

import java.io.ByteArrayInputStream;
import java.io.InputStream;
import java.lang.reflect.Method;
import java.nio.ByteBuffer;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;
import java.util.Random;

final class CustomNamesVectors {
    private static final String[] TEXT_POOL = {"a", "Z", "7", " ", " ", "\t", "\r", "\n", "\r\n", "é",
            "€", "😀", "\u0085", " ", " ", "﻿", "\u0000", "\u001f", " ", "\u000b",
            "\u000c", "　", "Bob", "&"};
    private static final String[] LONE_SURROGATES = {"\ud800", "\udfff"};
    private static final int[][] BYTE_PIECES = {{0x41}, {0x7A}, {0x20}, {0x09}, {0x0A}, {0x0D}, {0x00}, {0x0D, 0x0A},
            {0x80}, {0xBF}, {0xC0}, {0xC1}, {0xC2}, {0xDF}, {0xE0}, {0xE1}, {0xED}, {0xEF}, {0xF0}, {0xF3}, {0xF4},
            {0xF5}, {0xF8}, {0xFF}, {0xC3, 0xA9}, {0xE2, 0x82, 0xAC}, {0xE2, 0x80, 0xA8}, {0xC2, 0x85},
            {0xED, 0xA0, 0x80}, {0xED, 0x9F, 0xBF}, {0xE0, 0x80, 0x80}, {0xE0, 0xA0, 0x80}, {0xF0, 0x9F, 0x98, 0x80},
            {0xF0, 0x80, 0x80, 0x80}, {0xF4, 0x8F, 0xBF, 0xBF}, {0xF4, 0x90, 0x80, 0x80}, {0xEF, 0xBB, 0xBF},
            {0xC0, 0x80}, {0xE0, 0x9F}, {0xF0, 0x90}, {0xF0, 0x90, 0x80}};

    private CustomNamesVectors() {
    }

    static String generate(Path configDirectory) throws Exception {
        Random random = new Random(0xC0570AL);
        byte[] defaultNames = Files.readAllBytes(configDirectory.resolve("customnames.rncn"));
        CustomNamesSet parsedDefault = new CustomNamesSet(new ByteArrayInputStream(defaultNames));
        String defaultCase = Json.obj()
                .raw("bytes", Json.base64(defaultNames))
                .raw("parsed", describe(parsedDefault))
                .put("checksum", checksum(defaultNames))
                .raw("reserialized", Json.base64(parsedDefault.getBytes()))
                .toString();
        List<String> sets = new ArrayList<>();
        for (int i = 0; i < 60; i++) {
            sets.add(randomSetCase(random));
        }
        List<String> parseFuzz = new ArrayList<>();
        for (int i = 0; i < 300; i++) {
            byte[] bytes = fuzzedNamesFile(random);
            Json.Obj out = Json.obj().raw("bytes", Json.base64(bytes));
            try {
                out.raw("parsed", describe(new CustomNamesSet(new ByteArrayInputStream(bytes))));
            } catch (Throwable error) {
                out.put("error", Outcomes.errorName(error));
            }
            parseFuzz.add(out.toString());
        }
        List<String> checksums = new ArrayList<>();
        for (int i = 0; i < 400; i++) {
            byte[] bytes = trickyBytes(random, random.nextInt(60));
            checksums.add(Json.obj().raw("bytes", Json.base64(bytes)).put("checksum", checksum(bytes)).toString());
        }
        return Json.obj()
                .raw("default", defaultCase)
                .raw("sets", Json.array(sets))
                .raw("parse_fuzz", Json.array(parseFuzz))
                .raw("checksums", Json.array(checksums))
                .toString();
    }

    static String serializationVectors(Path configDirectory) throws Exception {
        Random random = new Random(0x5E9A7A7EL);
        byte[] defaultNames = Files.readAllBytes(configDirectory.resolve("customnames.rncn"));
        CustomNamesSet parsedDefault = new CustomNamesSet(new ByteArrayInputStream(defaultNames));
        List<String> sets = new ArrayList<>();
        for (int i = 0; i < 30; i++) {
            sets.add(randomSetCase(random));
        }
        return Json.obj()
                .put("line_separator", System.lineSeparator())
                .raw("default_reserialized", Json.base64(parsedDefault.getBytes()))
                .raw("sets", Json.array(sets))
                .toString();
    }

    private static String randomSetCase(Random random) throws Exception {
        List<List<String>> lists = new ArrayList<>();
        for (int list = 0; list < 5; list++) {
            List<String> names = new ArrayList<>();
            int count = random.nextInt(9);
            for (int i = 0; i < count; i++) {
                names.add(randomText(random, random.nextInt(12), false));
            }
            lists.add(names);
        }
        CustomNamesSet set = namesSet(lists);
        byte[] bytes = set.getBytes();
        return Json.obj()
                .raw("lists", describeLists(lists))
                .raw("bytes", Json.base64(bytes))
                .raw("parsed", describe(new CustomNamesSet(new ByteArrayInputStream(bytes))))
                .put("checksum", checksum(bytes))
                .toString();
    }

    private static CustomNamesSet namesSet(List<List<String>> lists) {
        CustomNamesSet set = new CustomNamesSet();
        set.setTrainerNames(lists.get(0));
        set.setTrainerClasses(lists.get(1));
        set.setDoublesTrainerNames(lists.get(2));
        set.setDoublesTrainerClasses(lists.get(3));
        set.setPokemonNicknames(lists.get(4));
        return set;
    }

    static byte[] randomNamesBytes(Random random) throws Exception {
        List<List<String>> lists = new ArrayList<>();
        for (int list = 0; list < 5; list++) {
            List<String> names = new ArrayList<>();
            int count = random.nextInt(6);
            for (int i = 0; i < count; i++) {
                names.add(randomText(random, 1 + random.nextInt(8), false));
            }
            lists.add(names);
        }
        return namesSet(lists).getBytes();
    }

    static String describe(CustomNamesSet names) {
        return Json.obj()
                .raw("trainer_names", Json.strings(names.getTrainerNames()))
                .raw("trainer_classes", Json.strings(names.getTrainerClasses()))
                .raw("doubles_trainer_names", Json.strings(names.getDoublesTrainerNames()))
                .raw("doubles_trainer_classes", Json.strings(names.getDoublesTrainerClasses()))
                .raw("pokemon_nicknames", Json.strings(names.getPokemonNicknames()))
                .toString();
    }

    private static String describeLists(List<List<String>> lists) {
        return Json.obj()
                .raw("trainer_names", Json.strings(lists.get(0)))
                .raw("trainer_classes", Json.strings(lists.get(1)))
                .raw("doubles_trainer_names", Json.strings(lists.get(2)))
                .raw("doubles_trainer_classes", Json.strings(lists.get(3)))
                .raw("pokemon_nicknames", Json.strings(lists.get(4)))
                .toString();
    }

    static String randomText(Random random, int pieces, boolean allowLoneSurrogates) {
        StringBuilder out = new StringBuilder();
        for (int i = 0; i < pieces; i++) {
            if (allowLoneSurrogates && random.nextInt(8) == 0) {
                out.append(LONE_SURROGATES[random.nextInt(LONE_SURROGATES.length)]);
            } else {
                out.append(TEXT_POOL[random.nextInt(TEXT_POOL.length)]);
            }
        }
        return out.toString();
    }

    static byte[] trickyBytes(Random random, int pieces) {
        List<Integer> bytes = new ArrayList<>();
        for (int i = 0; i < pieces; i++) {
            for (int value : BYTE_PIECES[random.nextInt(BYTE_PIECES.length)]) {
                bytes.add(value);
            }
        }
        byte[] out = new byte[bytes.size()];
        for (int i = 0; i < out.length; i++) {
            out[i] = (byte) (int) bytes.get(i);
        }
        return out;
    }

    private static byte[] fuzzedNamesFile(Random random) {
        List<byte[]> parts = new ArrayList<>();
        parts.add(new byte[] {(byte) (random.nextInt(10) == 0 ? random.nextInt(4) : 1)});
        int blocks = random.nextInt(7);
        for (int block = 0; block < blocks; block++) {
            byte[] content = trickyBytes(random, random.nextInt(10));
            int declared = content.length;
            int mode = random.nextInt(12);
            if (mode == 0) {
                declared = content.length + 1 + random.nextInt(40);
            } else if (mode == 1) {
                declared = -1 - random.nextInt(5);
            } else if (mode == 2) {
                declared = Math.max(0, content.length - 1 - random.nextInt(3));
            }
            parts.add(ByteBuffer.allocate(4).putInt(declared).array());
            parts.add(content);
        }
        if (random.nextInt(4) == 0) {
            parts.add(trickyBytes(random, random.nextInt(4)));
        }
        int total = 0;
        for (byte[] part : parts) {
            total += part.length;
        }
        ByteBuffer joined = ByteBuffer.allocate(total);
        for (byte[] part : parts) {
            joined.put(part);
        }
        return joined.array();
    }

    static int checksum(byte[] bytes) throws Exception {
        Method method = FileFunctions.class.getDeclaredMethod("getFileChecksum", InputStream.class);
        method.setAccessible(true);
        return (Integer) method.invoke(null, new ByteArrayInputStream(bytes));
    }
}
