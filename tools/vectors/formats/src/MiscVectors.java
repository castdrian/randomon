import com.dabomstew.pkrandom.FileFunctions;
import com.dabomstew.pkrandom.RomFunctions;
import com.dabomstew.pkrandom.ctr.AMX;
import com.dabomstew.pkrandom.ctr.BFLIM;
import com.dabomstew.pkrandom.ctr.Mini;
import com.dabomstew.pkrandom.ctr.SMDH;
import com.dabomstew.pkrandom.newnds.CRC16;
import com.dabomstew.pkrandom.newnds.NARCArchive;

import java.awt.image.BufferedImage;
import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.Map;
import java.util.Random;
import java.util.stream.Stream;

public final class MiscVectors {
    private MiscVectors() {
    }

    public static Map<String, Object> search() {
        Random random = new Random(0x5EA);
        List<Object> searches = new ArrayList<>();
        for (int i = 0; i < 120; i++) {
            byte[] haystack = Bin.lowEntropy(random, 20 + random.nextInt(1500), 2 + random.nextInt(3));
            byte[] needle;
            if (random.nextBoolean()) {
                int length = 2 + random.nextInt(7);
                int start = random.nextInt(haystack.length - length);
                needle = Arrays.copyOfRange(haystack, start, start + length);
            } else {
                needle = Bin.lowEntropy(random, 2 + random.nextInt(5), 3);
            }
            int begin = random.nextInt(4) == 0 ? random.nextInt(haystack.length) : 0;
            int end = random.nextInt(4) == 0 ? begin + random.nextInt(haystack.length - begin + 1) : haystack.length;
            Map<String, Object> vector = Json.object();
            vector.put("haystack", haystack);
            vector.put("needle", needle);
            vector.put("begin", begin);
            vector.put("end", end);
            vector.put("found", new ArrayList<Object>(RomFunctions.search(haystack, begin, end, needle)));
            searches.add(vector);
        }
        List<Object> freeSpace = new ArrayList<>();
        for (int i = 0; i < 60; i++) {
            byte[] rom = Bin.lowEntropy(random, 200 + random.nextInt(800), 3);
            for (int run = 0; run < 3; run++) {
                int start = random.nextInt(rom.length - 40);
                Arrays.fill(rom, start, start + 5 + random.nextInt(30), (byte) 0xFF);
            }
            int amount = 1 + random.nextInt(20);
            int offset = random.nextInt(rom.length / 2);
            boolean aligned = random.nextBoolean();
            Map<String, Object> vector = Json.object();
            vector.put("rom", rom);
            vector.put("amount", amount);
            vector.put("offset", offset);
            vector.put("aligned", aligned);
            vector.put("result", RomFunctions.freeSpaceFinder(rom, (byte) 0xFF, amount, offset, aligned));
            freeSpace.add(vector);
        }
        Map<String, Object> root = Json.object();
        root.put("search", searches);
        root.put("free_space", freeSpace);
        return root;
    }

    public static Map<String, Object> ips(Path upstreamPatches, Path testPatchRoot) throws IOException {
        List<Object> upstream = new ArrayList<>();
        List<Path> patchFiles;
        try (Stream<Path> walk = Files.walk(upstreamPatches)) {
            patchFiles = walk.filter(p -> p.toString().endsWith(".ips")).sorted().toList();
        }
        for (Path patchFile : patchFiles) {
            String relative = upstreamPatches.relativize(patchFile).toString().replace('\\', '/');
            String name = relative.substring(0, relative.length() - 4);
            byte[] patch = Files.readAllBytes(patchFile);
            int size = Bin.align(maximumPatchEnd(patch) + 0x100, 0x10000);
            byte[] rom = syntheticRom(size);
            Map<String, Object> vector = Json.object();
            vector.put("name", name);
            vector.put("rom_size", size);
            Map<String, Object> outcome = Guard.run(() -> {
                FileFunctions.applyPatch(rom, name);
                return Bin.sha256(rom);
            });
            vector.put("outcome", outcome);
            upstream.add(vector);
        }
        List<Object> synthetic = new ArrayList<>();
        Path patchDir = testPatchRoot.resolve("com/dabomstew/pkrandom/patches");
        Files.createDirectories(patchDir);
        List<byte[]> patches = syntheticPatches();
        for (int i = 0; i < patches.size(); i++) {
            String name = "vector_" + i;
            byte[] patch = patches.get(i);
            Files.write(patchDir.resolve(name + ".ips"), patch);
            byte[] rom = syntheticRom(64);
            Map<String, Object> vector = Json.object();
            vector.put("patch", patch);
            vector.put("rom", rom.clone());
            vector.put("outcome", Guard.run(() -> {
                FileFunctions.applyPatch(rom, name);
                return rom;
            }));
            synthetic.add(vector);
        }
        Map<String, Object> root = Json.object();
        root.put("upstream", upstream);
        root.put("synthetic", synthetic);
        return root;
    }

    static byte[] syntheticRom(int size) {
        byte[] rom = new byte[size];
        for (int i = 0; i < size; i++) {
            rom[i] = (byte) ((i * 31) ^ (i >>> 9));
        }
        return rom;
    }

    private static int maximumPatchEnd(byte[] patch) {
        int offset = 5;
        int maximum = 0;
        while (offset + 2 < patch.length) {
            int writeOffset = ((patch[offset] & 0xFF) << 16) | ((patch[offset + 1] & 0xFF) << 8) | (patch[offset + 2] & 0xFF);
            if (writeOffset == 0x454f46) {
                break;
            }
            offset += 3;
            int size = ((patch[offset] & 0xFF) << 8) | (patch[offset + 1] & 0xFF);
            offset += 2;
            if (size == 0) {
                int rleSize = ((patch[offset] & 0xFF) << 8) | (patch[offset + 1] & 0xFF);
                maximum = Math.max(maximum, writeOffset + rleSize);
                offset += 3;
            } else {
                maximum = Math.max(maximum, writeOffset + size);
                offset += size;
            }
        }
        return maximum;
    }

    private static List<byte[]> syntheticPatches() {
        List<byte[]> patches = new ArrayList<>();
        patches.add(ipsBytes("PATCH", new int[][] {{3, 4}, {20, 0, 6, 0x7A}, {60, 2}}, true));
        patches.add(ipsBytes("PATCX", new int[][] {{3, 4}}, true));
        patches.add(Bin.ascii("PATCH"));
        patches.add(ipsBytes("PATCH", new int[][] {{60, 0, 10, 0x11}}, true));
        patches.add(ipsBytes("PATCH", new int[][] {{62, 5}}, true));
        patches.add(ipsBytes("PATCH", new int[][] {{1, 2}, {8, 3}}, false));
        byte[] truncated = ipsBytes("PATCH", new int[][] {{4, 9}}, false);
        patches.add(Arrays.copyOf(truncated, truncated.length - 3));
        patches.add(Arrays.copyOf(ipsBytes("PATCH", new int[][] {{1, 2}}, false), 9));
        patches.add(Arrays.copyOf(ipsBytes("PATCH", new int[][] {{1, 0, 4, 1}}, false), 11));
        patches.add(Arrays.copyOf(ipsBytes("PATCH", new int[][] {{1, 0, 4, 1}}, false), 12));
        patches.add(ipsBytes("PATCH", new int[][] {{0, 64}}, true));
        return patches;
    }

    private static byte[] ipsBytes(String magic, int[][] records, boolean terminated) {
        ByteArrayOutputStream out = new ByteArrayOutputStream();
        out.writeBytes(Bin.ascii(magic));
        for (int[] record : records) {
            out.write(record[0] >> 16);
            out.write(record[0] >> 8);
            out.write(record[0]);
            out.write(record[1] >> 8);
            out.write(record[1]);
            if (record[1] == 0) {
                out.write(record[2] >> 8);
                out.write(record[2]);
                out.write(record[3]);
            } else {
                for (int i = 0; i < record[1]; i++) {
                    out.write(0xA0 + i);
                }
            }
        }
        if (terminated) {
            out.writeBytes(Bin.ascii("EOF"));
        }
        return out.toByteArray();
    }

    public static Map<String, Object> crc16() {
        Random random = new Random(0xC16);
        List<Object> cases = new ArrayList<>();
        for (int i = 0; i < 30; i++) {
            byte[] data = Bin.noise(random, random.nextInt(600));
            Map<String, Object> vector = Json.object();
            vector.put("data", data);
            vector.put("crc", CRC16.calculate(data, 0, data.length) & 0xFFFF);
            cases.add(vector);
        }
        Map<String, Object> root = Json.object();
        root.put("cases", cases);
        return root;
    }

    public static Map<String, Object> timsort() {
        Random random = new Random(0x7150);
        List<Object> cases = new ArrayList<>();
        int[] sizes = {0, 1, 2, 5, 31, 32, 33, 64, 65, 100, 257, 500, 1000, 2000};
        for (int size : sizes) {
            for (int distribution = 0; distribution < 5; distribution++) {
                long[] values = new long[size];
                for (int i = 0; i < size; i++) {
                    values[i] = switch (distribution) {
                        case 0 -> random.nextInt(1000);
                        case 1 -> (random.nextLong() >>> 30);
                        case 2 -> i * 16L + (random.nextInt(10) == 0 ? -random.nextInt(4000) : 0);
                        case 3 -> random.nextInt(4);
                        default -> (long) i * (1L << 26) * (random.nextInt(3) == 0 ? -1 : 1);
                    };
                }
                Map<String, Object> vector = Json.object();
                List<Object> valueList = new ArrayList<>();
                for (long value : values) {
                    valueList.add(value);
                }
                vector.put("values", valueList);
                vector.put("outcome", Guard.run(() -> {
                    List<long[]> items = new ArrayList<>();
                    for (int i = 0; i < values.length; i++) {
                        items.add(new long[] {i, values[i]});
                    }
                    items.sort((left, right) -> (int) (left[1] - right[1]));
                    List<Object> order = new ArrayList<>();
                    for (long[] item : items) {
                        order.add((int) item[0]);
                    }
                    return order;
                }));
                cases.add(vector);
            }
        }
        Map<String, Object> root = Json.object();
        root.put("cases", cases);
        return root;
    }

    public static Map<String, Object> jdk() {
        Map<String, Object> root = Json.object();
        String trimmed = new String(new byte[8], StandardCharsets.UTF_8).trim();
        root.put("trimmed_empty_is_literal", trimmed == "");
        root.put("java_version", System.getProperty("java.version"));
        List<Object> utf16 = new ArrayList<>();
        byte[][] samples = {
            {0x00, (byte) 0xD8, 0x61, 0x00, 0x62, 0x00},
            {0x00, (byte) 0xD8},
            {0x00, (byte) 0xD8, 0x61},
            {0x41, 0x00, 0x00, (byte) 0xD8, 0x61},
            {0x00, (byte) 0xDC, 0x61, 0x00},
            {0x61, 0x00, 0x62},
            {0x3D, (byte) 0xD8, 0x00, (byte) 0xDE, 0x21, 0x00},
            {0x00, (byte) 0xD8, 0x00, (byte) 0xD8, 0x00, (byte) 0xDC},
            {0x20, 0x00, 0x00, 0x00, 0x41, 0x00, 0x20, 0x00},
        };
        for (byte[] sample : samples) {
            Map<String, Object> vector = Json.object();
            vector.put("input", sample);
            String decoded = new String(sample, StandardCharsets.UTF_16LE);
            vector.put("decoded", decoded.getBytes(StandardCharsets.UTF_16LE));
            vector.put("trimmed", decoded.trim().getBytes(StandardCharsets.UTF_16LE));
            utf16.add(vector);
        }
        root.put("utf16", utf16);
        return root;
    }

    public static Map<String, Object> mini() {
        Random random = new Random(0x3141);
        List<Object> packs = new ArrayList<>();
        for (int i = 0; i < 12; i++) {
            int count = random.nextInt(9);
            byte[][] files = new byte[count][];
            List<Object> fileList = new ArrayList<>();
            for (int f = 0; f < count; f++) {
                files[f] = Bin.noise(random, random.nextInt(23));
                fileList.add(files[f]);
            }
            String identifier = i % 2 == 0 ? "WD" : "EA";
            Map<String, Object> vector = Json.object();
            vector.put("identifier", identifier);
            vector.put("files", fileList);
            vector.put("outcome", Guard.run(() -> Mini.PackMini(files, identifier)));
            packs.add(vector);
        }
        List<Object> unpacks = new ArrayList<>();
        byte[] valid = packMini(new byte[][] {{1, 2, 3}, {4}, {}}, "WD");
        List<byte[]> inputs = List.of(valid, Arrays.copyOf(valid, 10), new byte[] {'W', 'D'}, new byte[] {'E', 'A', 1, 0, 8, 0, 0, 0, 4, 0, 0, 0}, new byte[] {(byte) 0xC3, 'D', 0, 0});
        for (byte[] input : inputs) {
            for (String identifier : new String[] {"WD", "EA"}) {
                Map<String, Object> vector = Json.object();
                vector.put("input", input);
                vector.put("identifier", identifier);
                vector.put("outcome", Guard.run(() -> {
                    byte[][] unpacked = Mini.UnpackMini(input, identifier);
                    if (unpacked == null) {
                        return null;
                    }
                    List<Object> list = new ArrayList<>();
                    list.addAll(Arrays.asList((Object[]) unpacked));
                    return list;
                }));
                unpacks.add(vector);
            }
        }
        Map<String, Object> root = Json.object();
        root.put("pack", packs);
        root.put("unpack", unpacks);
        return root;
    }

    private static byte[] packMini(byte[][] files, String identifier) {
        try {
            return Mini.PackMini(files, identifier);
        } catch (IOException e) {
            throw new RuntimeException(e);
        }
    }

    public static byte[] syntheticSmdh(Random random, boolean valid) {
        byte[] data = new byte[0x36C0];
        Bin.put(data, 0, Bin.ascii(valid ? "SMDH" : "HDMS"));
        String[] samples = {"  Pokemon Alpha Sapphire  ", "Pokémon", "こんにちは", "Nintendo", "", " x ", "😀 emoji"};
        for (int i = 0; i < 12; i++) {
            byte[] shortBytes = Bin.utf16(samples[random.nextInt(samples.length)]);
            byte[] longBytes = Bin.utf16(samples[random.nextInt(samples.length)] + " long");
            byte[] publisher = Bin.utf16(samples[random.nextInt(samples.length)]);
            Bin.put(data, 0x08 + 0x200 * i, Arrays.copyOf(shortBytes, Math.min(shortBytes.length, 0x80)));
            Bin.put(data, 0x88 + 0x200 * i, Arrays.copyOf(longBytes, Math.min(longBytes.length, 0x100)));
            Bin.put(data, 0x188 + 0x200 * i, Arrays.copyOf(publisher, Math.min(publisher.length, 0x80)));
        }
        data[0x88 + 0x200 * 3 + 0x20] = (byte) 0x00;
        data[0x88 + 0x200 * 3 + 0x21] = (byte) 0xD8;
        for (int i = 0x2040; i < data.length; i++) {
            data[i] = (byte) random.nextInt(256);
        }
        return data;
    }

    public static Map<String, Object> smdh() {
        Random random = new Random(0x53D4);
        List<Object> cases = new ArrayList<>();
        String[][] operations = {
            {"ORAS randomizer seed: 123456789", "Universal Pokemon Randomizer ZX"},
            {"XY randomizer seed: -42", "Universal Pokemon Randomizer ZX"},
            {"A description that is definitely far too long to fit into sixty four characters", "Short"},
            {"Fits", "A publisher name that is definitely too long for the sixty four character limit"},
        };
        for (int i = 0; i < operations.length; i++) {
            byte[] input = syntheticSmdh(random, i != 3);
            String description = operations[i][0];
            String publisher = operations[i][1];
            Map<String, Object> vector = Json.object();
            vector.put("input", input);
            vector.put("description", description);
            vector.put("publisher", publisher);
            vector.put("outcome", Guard.run(() -> {
                SMDH smdh = new SMDH(input.clone());
                smdh.setAllDescriptions(description);
                smdh.setAllPublishers(publisher);
                return smdh.getBytes();
            }));
            cases.add(vector);
        }
        Map<String, Object> root = Json.object();
        root.put("cases", cases);
        return root;
    }

    public static Map<String, Object> amx() {
        Random random = new Random(0xA3);
        List<Object> cases = new ArrayList<>();
        for (int i = 0; i < 20; i++) {
            int count = 1 + random.nextInt(60);
            int[] instructions = new int[count];
            for (int k = 0; k < count; k++) {
                instructions[k] = switch (random.nextInt(5)) {
                    case 0 -> random.nextInt(64);
                    case 1 -> -random.nextInt(64);
                    case 2 -> random.nextInt();
                    case 3 -> random.nextInt(1 << 14) - (1 << 13);
                    default -> new int[] {0, -1, Integer.MIN_VALUE, Integer.MAX_VALUE, 63, 64, -64, -65}[random.nextInt(8)];
                };
            }
            byte[] script = buildAmx(random, instructions, i % 3 == 0);
            Map<String, Object> vector = Json.object();
            vector.put("input", script);
            vector.put("outcome", amxOutcome(() -> new AMX(script)));
            cases.add(vector);
        }
        List<Object> embedded = new ArrayList<>();
        ByteArrayOutputStream blob = new ByteArrayOutputStream();
        blob.writeBytes(Bin.noise(random, 13));
        blob.writeBytes(buildAmx(random, new int[] {1, 2, 3, -4}, false));
        blob.writeBytes(Bin.noise(random, 7));
        blob.writeBytes(buildAmx(random, new int[] {100000, -100000, 5}, true));
        blob.writeBytes(Bin.noise(random, 9));
        byte[] blobBytes = blob.toByteArray();
        for (int scriptNumber = 0; scriptNumber < 3; scriptNumber++) {
            int number = scriptNumber;
            Map<String, Object> vector = Json.object();
            vector.put("input", blobBytes);
            vector.put("script_number", number);
            vector.put("outcome", amxOutcome(() -> new AMX(blobBytes, number)));
            embedded.add(vector);
        }
        byte[] broken = buildAmx(random, new int[] {1, 2}, false);
        Bin.writeInt(broken, 0x14, Bin.readInt(broken, 0xC) + 6);
        Map<String, Object> brokenVector = Json.object();
        brokenVector.put("input", broken);
        brokenVector.put("outcome", amxOutcome(() -> new AMX(broken)));
        cases.add(brokenVector);
        Map<String, Object> root = Json.object();
        root.put("cases", cases);
        root.put("embedded", embedded);
        return root;
    }

    private interface AmxFactory {
        AMX create() throws Exception;
    }

    private static Map<String, Object> amxOutcome(AmxFactory factory) {
        return Guard.run(() -> {
            AMX amx = factory.create();
            Map<String, Object> result = Json.object();
            result.put("dec_data", amx.decData);
            result.put("script_offset", amx.scriptOffset);
            result.put("bytes", amx.decData == null ? null : Guard.run(amx::getBytes));
            return result;
        });
    }

    private static byte[] buildAmx(Random random, int[] instructions, boolean nonCanonical) {
        ByteArrayOutputStream compressed = new ByteArrayOutputStream();
        for (int instruction : instructions) {
            List<Integer> groups = new ArrayList<>();
            long value = Integer.toUnsignedLong(instruction);
            boolean negative = instruction < 0;
            long shadow = Integer.toUnsignedLong(negative ? ~instruction : instruction);
            do {
                groups.add((int) (value & 0x7F));
                value >>= 7;
                shadow >>= 7;
            } while (shadow != 0);
            if (groups.size() < 5 && ((groups.get(groups.size() - 1) & 0x40) != 0) != negative) {
                groups.add(negative ? 0x7F : 0x00);
            }
            if (nonCanonical && groups.size() < 5 && random.nextBoolean()) {
                groups.add(negative ? 0x7F : 0x00);
            }
            for (int g = groups.size() - 1; g >= 0; g--) {
                compressed.write(groups.get(g) | (g > 0 ? 0x80 : 0));
            }
        }
        byte[] extra = Bin.noise(random, 4 * random.nextInt(5));
        byte[] payload = compressed.toByteArray();
        int scriptInstrStart = 0x1C + extra.length;
        int length = scriptInstrStart + payload.length;
        byte[] script = new byte[length];
        Bin.writeInt(script, 0, length);
        Bin.writeInt(script, 4, 0x0A0AF1E0);
        Bin.writeShort(script, 8, random.nextInt(0x10000));
        Bin.writeShort(script, 0xA, random.nextInt(0x100));
        Bin.writeInt(script, 0xC, scriptInstrStart);
        Bin.writeInt(script, 0x10, scriptInstrStart + random.nextInt(64));
        Bin.writeInt(script, 0x14, scriptInstrStart + instructions.length * 4);
        Bin.writeInt(script, 0x18, random.nextInt(0x10000));
        Bin.put(script, 0x1C, extra);
        Bin.put(script, scriptInstrStart, payload);
        return script;
    }

    public static Map<String, Object> bflim() {
        Random random = new Random(0xBF1);
        List<Object> cases = new ArrayList<>();
        int[][] shapes = {{8, 8, 7}, {16, 8, 8}, {8, 24, 7}, {32, 16, 8}, {8, 8, 5}};
        for (int[] shape : shapes) {
            int width = shape[0];
            int height = shape[1];
            int format = shape[2];
            byte[] imageData = Bin.noise(random, width * height * 2);
            byte[] file = new byte[imageData.length + 0x28];
            Bin.put(file, 0, imageData);
            int header = imageData.length;
            Bin.writeIntBigEndian(file, header, 0x464C494D);
            file[header + 4] = (byte) 0xFF;
            file[header + 5] = (byte) 0xFE;
            Bin.writeShort(file, header + 6, 0x14);
            Bin.writeInt(file, header + 8, 0x02020000);
            Bin.writeInt(file, header + 12, file.length);
            int imag = header + 0x14;
            Bin.writeIntBigEndian(file, imag, 0x696D6167);
            Bin.writeInt(file, imag + 4, 0x10);
            Bin.writeShort(file, imag + 8, width);
            Bin.writeShort(file, imag + 10, height);
            Bin.writeShort(file, imag + 12, 0x80);
            file[imag + 14] = (byte) format;
            file[imag + 15] = 0;
            Bin.writeInt(file, imag + 16, imageData.length);
            Map<String, Object> vector = Json.object();
            vector.put("input", file);
            vector.put("outcome", Guard.run(() -> {
                BufferedImage image = new BFLIM(file).getImage();
                byte[] pixels = new byte[image.getWidth() * image.getHeight() * 4];
                int index = 0;
                for (int y = 0; y < image.getHeight(); y++) {
                    for (int x = 0; x < image.getWidth(); x++) {
                        Bin.writeIntBigEndian(pixels, index, image.getRGB(x, y));
                        index += 4;
                    }
                }
                return pixels;
            }));
            cases.add(vector);
        }
        Map<String, Object> root = Json.object();
        root.put("cases", cases);
        return root;
    }

    public static Map<String, Object> narc() throws IOException {
        Random random = new Random(0x4A2C);
        List<Object> cases = new ArrayList<>();
        NARCArchive fresh = new NARCArchive();
        for (int i = 0; i < 7; i++) {
            fresh.files.add(Bin.noise(random, random.nextInt(30)));
        }
        byte[] freshBytes = fresh.getBytes();
        cases.add(narcVector(freshBytes));
        cases.add(narcVector(new NARCArchive().getBytes()));
        byte[][] realNames = {Bin.ascii("alpha"), "b\u00e9".getBytes(StandardCharsets.ISO_8859_1), Bin.ascii("c"), Bin.ascii("delta")};
        byte[][] tameNames = {{1, 1}, {(byte) 0x81}, {1}, {}};
        cases.add(narcVector(narcWithFilenames(random, realNames)));
        cases.add(narcVector(narcWithFilenames(random, tameNames)));
        byte[] understated = narcWithFilenames(random, tameNames);
        byte[] grown = Arrays.copyOf(understated, understated.length + 24);
        cases.add(narcVector(grown));
        byte[] missing = freshBytes.clone();
        missing[0x10] = 'X';
        cases.add(narcVector(missing));
        Map<String, Object> root = Json.object();
        root.put("cases", cases);
        return root;
    }

    private static Map<String, Object> narcVector(byte[] input) {
        Map<String, Object> vector = Json.object();
        vector.put("input", input);
        vector.put("outcome", Guard.run(() -> {
            NARCArchive narc = new NARCArchive(input);
            Map<String, Object> result = Json.object();
            result.put("files", new ArrayList<Object>(narc.files));
            result.put("bytes", narc.getBytes());
            return result;
        }));
        return vector;
    }

    private static byte[] narcWithFilenames(Random random, byte[][] names) {
        List<byte[]> files = new ArrayList<>();
        for (int i = 0; i < 4; i++) {
            files.add(Bin.noise(random, 3 + random.nextInt(20)));
        }
        ByteArrayOutputStream fimg = new ByteArrayOutputStream();
        byte[] fatb = new byte[12 + files.size() * 8];
        Bin.put(fatb, 0, Bin.ascii("BTAF"));
        Bin.writeInt(fatb, 4, fatb.length);
        Bin.writeInt(fatb, 8, files.size());
        for (int i = 0; i < files.size(); i++) {
            Bin.writeInt(fatb, 12 + i * 8, fimg.size());
            fimg.writeBytes(files.get(i));
            Bin.writeInt(fatb, 16 + i * 8, fimg.size());
            while (fimg.size() % 4 != 0) {
                fimg.write(0xFF);
            }
        }
        ByteArrayOutputStream fntbBody = new ByteArrayOutputStream();
        fntbBody.writeBytes(new byte[] {8, 0, 0, 0, 0, 0, 1, 0});
        for (byte[] encoded : names) {
            fntbBody.write(encoded.length);
            fntbBody.writeBytes(encoded);
        }
        fntbBody.write(0);
        byte[] fntbPayload = fntbBody.toByteArray();
        byte[] fntb = new byte[8 + Bin.align(fntbPayload.length, 4)];
        Bin.put(fntb, 0, Bin.ascii("BTNF"));
        Bin.writeInt(fntb, 4, fntb.length);
        Bin.put(fntb, 8, fntbPayload);
        byte[] fimgPayload = fimg.toByteArray();
        byte[] fimgFrame = new byte[8 + fimgPayload.length];
        Bin.put(fimgFrame, 0, Bin.ascii("GMIF"));
        Bin.writeInt(fimgFrame, 4, fimgFrame.length);
        Bin.put(fimgFrame, 8, fimgPayload);
        int total = 16 + fntb.length + fatb.length + fimgFrame.length;
        byte[] narc = new byte[total];
        Bin.put(narc, 0, Bin.ascii("NARC"));
        Bin.writeShort(narc, 4, 0xFFFE);
        Bin.writeShort(narc, 6, 0x0100);
        Bin.writeInt(narc, 8, total);
        Bin.writeShort(narc, 12, 0x10);
        Bin.writeShort(narc, 14, 3);
        Bin.put(narc, 16, fntb);
        Bin.put(narc, 16 + fntb.length, fatb);
        Bin.put(narc, 16 + fntb.length + fatb.length, fimgFrame);
        return narc;
    }
}
