import com.dabomstew.pkrandom.RomFunctions;
import com.dabomstew.pkrandom.newnds.NDSRom;
import cuecompressors.BLZCoder;

import java.io.ByteArrayOutputStream;
import java.io.PrintStream;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.Map;
import java.util.Random;
import java.util.concurrent.Callable;

public final class NdsVectors {
    private NdsVectors() {
    }

    private static final int RAM_OFFSET = 0x02000000;
    private static final int OVERLAY_COUNT = 3;

    private static final class FsEntry {
        final String name;
        final int directory;
        final int fileId;

        FsEntry(String name, int directory, int fileId) {
            this.name = name;
            this.directory = directory;
            this.fileId = fileId;
        }
    }

    private static byte[] fnt() {
        List<List<FsEntry>> subtables = new ArrayList<>();
        int[] firstIds = {3, 4, 6, 7, 9};
        int[] parents = {5, 0xF000, 0xF000, 0xF002, 0xF000};
        subtables.add(List.of(new FsEntry("data", 0xF001, -1), new FsEntry("a", 0xF002, -1), new FsEntry("root.bin", -1, 3), new FsEntry("ab", 0xF004, -1)));
        subtables.add(List.of(new FsEntry("foo.bin", -1, 4), new FsEntry("bar.txt", -1, 5)));
        subtables.add(List.of(new FsEntry("0", 0xF003, -1), new FsEntry("bc.bin", -1, 6)));
        subtables.add(List.of(new FsEntry("1", -1, 7), new FsEntry("2", -1, 8)));
        subtables.add(List.of(new FsEntry("c.bin", -1, 9)));
        ByteArrayOutputStream body = new ByteArrayOutputStream();
        int[] offsets = new int[subtables.size()];
        int mainSize = 8 * subtables.size();
        for (int i = 0; i < subtables.size(); i++) {
            offsets[i] = mainSize + body.size();
            for (FsEntry entry : subtables.get(i)) {
                byte[] name = entry.name.getBytes(StandardCharsets.US_ASCII);
                if (entry.directory >= 0) {
                    body.write(name.length | 0x80);
                    body.writeBytes(name);
                    body.write(entry.directory & 0xFF);
                    body.write(entry.directory >> 8);
                } else {
                    body.write(name.length);
                    body.writeBytes(name);
                }
            }
            body.write(0);
        }
        byte[] table = new byte[mainSize + body.size()];
        for (int i = 0; i < subtables.size(); i++) {
            Bin.writeInt(table, i * 8, offsets[i]);
            Bin.writeShort(table, i * 8 + 4, firstIds[i]);
            Bin.writeShort(table, i * 8 + 6, parents[i]);
        }
        Bin.put(table, mainSize, body.toByteArray());
        return table;
    }

    private static byte[] compressedArm9(Random random) {
        byte[] plain = new byte[0x40 + 0x5000];
        Bin.put(plain, 0, Bin.noise(random, 0x40));
        Bin.put(plain, 0x40, Bin.structured(random, 0x5000));
        Bin.writeInt(plain, 0x10, 0);
        for (int attempt = 0; attempt < 20; attempt++) {
            byte[] encoded = new BLZCoder(null).BLZ_EncodePub(plain, false, false, "arm9");
            int value = encoded.length + RAM_OFFSET;
            if (Bin.readInt(plain, 0x10) == value) {
                byte[] needle = new byte[4];
                Bin.writeInt(needle, 0, value);
                if (RomFunctions.search(encoded, needle).size() != 1) {
                    throw new IllegalStateException("size marker is not unique");
                }
                return encoded;
            }
            Bin.writeInt(plain, 0x10, value);
        }
        throw new IllegalStateException("arm9 size marker did not converge");
    }

    private static byte[] build(Random random, boolean compressArm9, boolean footer, byte[][] fileContents) {
        byte[] arm9 = compressArm9 ? compressedArm9(random) : Bin.noise(random, 0x900);
        if (!compressArm9) {
            arm9[arm9.length - 5] = 0;
        }
        byte[] arm7 = Bin.noise(random, 0x300);
        byte[] banner = Bin.noise(random, 0x840);
        byte[] fnt = fnt();
        int fileCount = fileContents.length;
        ByteArrayOutputStream rom = new ByteArrayOutputStream();
        rom.writeBytes(new byte[0x400]);
        int arm9Offset = rom.size();
        rom.writeBytes(arm9);
        byte[] tail = Bin.noise(random, 12);
        if (footer) {
            Bin.writeInt(tail, 0, 0xDEC00621);
        } else {
            tail[0] = 0x55;
        }
        rom.writeBytes(tail);
        rom.writeBytes(new byte[0x30]);
        int ovlOffset = rom.size();
        rom.writeBytes(new byte[OVERLAY_COUNT * 32]);
        rom.writeBytes(new byte[0x10]);
        int arm7Offset = rom.size();
        rom.writeBytes(arm7);
        int arm7OvlOffset = rom.size();
        byte[] arm7Ovl = Bin.noise(random, 32);
        rom.writeBytes(arm7Ovl);
        rom.writeBytes(new byte[0x44]);
        int bannerOffset = rom.size();
        rom.writeBytes(banner);
        int fntOffset = rom.size();
        rom.writeBytes(fnt);
        rom.writeBytes(new byte[0x8]);
        int fatOffset = rom.size();
        rom.writeBytes(new byte[fileCount * 8]);
        int[] starts = new int[fileCount];
        for (int id = fileCount - 1; id >= 0; id--) {
            rom.writeBytes(new byte[random.nextInt(9)]);
            starts[id] = rom.size();
            rom.writeBytes(fileContents[id]);
        }
        rom.writeBytes(new byte[5]);
        byte[] image = rom.toByteArray();
        for (int id = 0; id < fileCount; id++) {
            Bin.writeInt(image, fatOffset + id * 8, starts[id]);
            Bin.writeInt(image, fatOffset + id * 8 + 4, starts[id] + fileContents[id].length);
        }
        int[] flags = {1, 0, 1};
        int[] compressedSizes = {fileContents[0].length, fileContents[1].length, fileContents[2].length + 4};
        for (int i = 0; i < OVERLAY_COUNT; i++) {
            int entry = ovlOffset + i * 32;
            Bin.writeInt(image, entry, i);
            Bin.writeInt(image, entry + 4, 0x02100000 + i * 0x1000);
            Bin.writeInt(image, entry + 8, 0x800 + i);
            Bin.writeInt(image, entry + 12, 0x40 * i);
            Bin.writeInt(image, entry + 16, 0x02100010 + i);
            Bin.writeInt(image, entry + 20, 0x02100020 + i);
            Bin.writeInt(image, entry + 24, i);
            Bin.writeInt(image, entry + 28, compressedSizes[i] | (flags[i] << 24));
        }
        Bin.put(image, 0, Bin.ascii("RANDOMONTEST"));
        Bin.put(image, 0x0C, Bin.ascii("TEST"));
        image[0x1E] = 1;
        image[0x14] = 3;
        Bin.writeInt(image, 0x20, arm9Offset);
        Bin.writeInt(image, 0x24, RAM_OFFSET + 0x800);
        Bin.writeInt(image, 0x28, RAM_OFFSET);
        Bin.writeInt(image, 0x2C, arm9.length);
        Bin.writeInt(image, 0x30, arm7Offset);
        Bin.writeInt(image, 0x34, 0x02380000);
        Bin.writeInt(image, 0x38, 0x02380000);
        Bin.writeInt(image, 0x3C, arm7.length);
        Bin.writeInt(image, 0x40, fntOffset);
        Bin.writeInt(image, 0x44, fnt.length);
        Bin.writeInt(image, 0x48, fatOffset);
        Bin.writeInt(image, 0x4C, fileCount * 8);
        Bin.writeInt(image, 0x50, ovlOffset);
        Bin.writeInt(image, 0x54, OVERLAY_COUNT * 32);
        Bin.writeInt(image, 0x58, arm7OvlOffset);
        Bin.writeInt(image, 0x5C, arm7Ovl.length);
        Bin.writeInt(image, 0x68, bannerOffset);
        Bin.writeInt(image, 0x80, image.length);
        Bin.writeInt(image, 0x84, 0x200);
        return image;
    }

    private static byte[][] contents(Random random) {
        byte[][] files = new byte[10][];
        files[0] = new BLZCoder(null).BLZ_EncodePub(Bin.structured(random, 3000), false, false, "overlay");
        files[1] = Bin.noise(random, 0x120);
        files[2] = Bin.noise(random, 0x80);
        for (int id = 3; id < 10; id++) {
            files[id] = Bin.noise(random, 1 + random.nextInt(200));
        }
        return files;
    }

    private interface RomOperation {
        Object apply(NDSRom rom) throws Exception;
    }

    private static Map<String, Object> op(List<Object> log, NDSRom rom, Map<String, Object> description, RomOperation operation) {
        Callable<Object> call = () -> operation.apply(rom);
        description.put("outcome", Guard.run(call));
        log.add(description);
        return description;
    }

    private static Map<String, Object> describe(String name, Object... keyValues) {
        Map<String, Object> description = Json.object();
        description.put("op", name);
        for (int i = 0; i < keyValues.length; i += 2) {
            description.put((String) keyValues[i], keyValues[i + 1]);
        }
        return description;
    }

    public static Map<String, Object> vectors(Path workDir) throws Exception {
        List<Object> cases = new ArrayList<>();
        cases.add(runCase(workDir, "case_compressed", 0x1D5, true, true, true));
        cases.add(runCase(workDir, "case_plain", 0x2D5, false, false, false));
        cases.add(runCase(workDir, "case_footer", 0x3D5, false, true, true));
        Map<String, Object> root = Json.object();
        root.put("cases", cases);
        return root;
    }

    private static Map<String, Object> runCase(Path workDir, String name, long seed, boolean compressArm9, boolean footer, boolean withOperations) throws Exception {
        Random random = new Random(seed);
        byte[] image = build(random, compressArm9, footer, contents(random));
        Path romPath = workDir.resolve(name + ".nds");
        Files.write(romPath, image);
        Map<String, Object> vector = Json.object();
        vector.put("name", name);
        vector.put("file_name", romPath.getFileName().toString());
        vector.put("input", image);
        NDSRom rom = new NDSRom(romPath.toString());
        List<Object> log = new ArrayList<>();
        op(log, rom, describe("code"), NDSRom::getCode);
        op(log, rom, describe("version"), r -> (int) r.getVersion());
        if (withOperations) {
            op(log, rom, describe("read_file", "path", "a/bc.bin"), r -> r.getFile("a/bc.bin"));
            op(log, rom, describe("read_file", "path", "ab/c.bin"), r -> r.getFile("ab/c.bin"));
            op(log, rom, describe("read_file", "path", "a/bc.bin"), r -> r.getFile("a/bc.bin"));
            op(log, rom, describe("read_file", "path", "a/0/2"), r -> r.getFile("a/0/2"));
            op(log, rom, describe("read_file", "path", "nope"), r -> r.getFile("nope"));
            byte[] replacement = Bin.noise(random, 77);
            op(log, rom, describe("write_file", "path", "data/foo.bin", "data", replacement), r -> {
                r.writeFile("data/foo.bin", replacement);
                return true;
            });
            op(log, rom, describe("write_file", "path", "missing.bin", "data", replacement), r -> {
                r.writeFile("missing.bin", replacement);
                return true;
            });
            op(log, rom, describe("read_overlay", "number", 0), r -> r.getOverlay(0));
            byte[] overlay = Bin.noise(random, 50);
            op(log, rom, describe("write_overlay", "number", 1, "data", overlay), r -> {
                r.writeOverlay(1, overlay);
                return true;
            });
            op(log, rom, describe("read_overlay", "number", 2), r -> r.getOverlay(2));
            op(log, rom, describe("read_overlay", "number", 5), r -> r.getOverlay(5));
            op(log, rom, describe("write_overlay", "number", 3, "data", overlay), r -> {
                r.writeOverlay(3, overlay);
                return true;
            });
            op(log, rom, describe("overlay_address", "number", 1), r -> r.getOverlayAddress(1));
            op(log, rom, describe("overlay_address", "number", 9), r -> r.getOverlayAddress(9));
            Map<String, Object> arm9Read = op(log, rom, describe("read_arm9"), NDSRom::getARM9);
            Map<?, ?> arm9Outcome = (Map<?, ?>) arm9Read.get("outcome");
            if ("ok".equals(arm9Outcome.get("result"))) {
                byte[] arm9 = ((byte[]) arm9Outcome.get("value")).clone();
                for (int i = 0x30; i < arm9.length; i += 97) {
                    arm9[i] ^= 0x5A;
                }
                byte[] modified = compressArm9 ? arm9 : Arrays.copyOf(arm9, arm9.length + 0x123);
                op(log, rom, describe("write_arm9", "data", modified), r -> {
                    r.writeARM9(modified);
                    return true;
                });
                op(log, rom, describe("read_arm9"), NDSRom::getARM9);
            }
        }
        vector.put("operations", log);
        Path outPath = workDir.resolve(name + "_out.nds");
        Files.deleteIfExists(outPath);
        Map<String, Object> saved = Guard.run(() -> {
            rom.saveTo(outPath.toString());
            return Files.readAllBytes(outPath);
        });
        vector.put("saved", saved);
        ByteArrayOutputStream diagnostics = new ByteArrayOutputStream();
        rom.printRomDiagnostics(new PrintStream(diagnostics, true, StandardCharsets.UTF_8));
        List<Object> lines = new ArrayList<>(Arrays.asList(diagnostics.toString(StandardCharsets.UTF_8).split("\\R")));
        vector.put("diagnostics", lines);
        return vector;
    }
}
