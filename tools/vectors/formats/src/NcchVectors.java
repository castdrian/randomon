import com.dabomstew.pkrandom.ctr.NCCH;
import com.dabomstew.pkrandom.romhandlers.Abstract3DSRomHandler;
import cuecompressors.BLZCoder;

import java.io.ByteArrayOutputStream;
import java.io.PrintStream;
import java.lang.reflect.Method;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Collections;
import java.util.List;
import java.util.Map;
import java.util.Random;
import java.util.stream.Stream;

public final class NcchVectors {
    private NcchVectors() {
    }

    private static final int UNIT = 0x200;

    private static final class Options {
        boolean codeCompressed;
        boolean noCrypto;
        boolean logo;
        boolean plain;
        boolean battleCro;
        boolean shuffleData;
        boolean iconFirst;
        boolean validRomfs = true;
        int level3Log2 = 9;
        int level1Log2 = -1;
        int level2Log2 = -1;
        String container = "ncch";
    }

    private static final class RomfsNode {
        final String name;
        final List<RomfsNode> children = new ArrayList<>();
        final List<String> files = new ArrayList<>();
        int offset;

        RomfsNode(String name) {
            this.name = name;
        }
    }

    private static final class FileRecord {
        final String name;
        final RomfsNode parent;
        final byte[] data;
        int metadataOffset;
        long dataOffset;

        FileRecord(String name, RomfsNode parent, byte[] data) {
            this.name = name;
            this.parent = parent;
            this.data = data;
        }
    }

    private static int dirEntrySize(RomfsNode node) {
        return 0x18 + Bin.align(Bin.utf16(node.name).length, 4);
    }

    private static int fileEntrySize(String name) {
        return 0x20 + Bin.align(Bin.utf16(name).length, 4);
    }

    private static List<RomfsNode> preorder(RomfsNode root) {
        List<RomfsNode> nodes = new ArrayList<>();
        nodes.add(root);
        for (RomfsNode child : root.children) {
            nodes.addAll(preorder(child));
        }
        return nodes;
    }

    private static byte[] romfs(Random random, Options options, List<FileRecord> records) {
        RomfsNode root = new RomfsNode("");
        RomfsNode a = new RomfsNode("a");
        RomfsNode a0 = new RomfsNode("0");
        RomfsNode a1 = new RomfsNode("1");
        RomfsNode sound = new RomfsNode("sound");
        root.children.add(a);
        root.children.add(sound);
        a.children.add(a0);
        a.children.add(a1);
        if (options.battleCro) {
            root.files.add("DllBattle.cro");
        }
        root.files.add("zz.bin");
        a0.files.addAll(List.of("0", "1", "2"));
        a1.files.add("0");
        sound.files.add("bgm.bcsar");

        List<RomfsNode> nodes = preorder(root);
        int dirOffset = 0;
        for (RomfsNode node : nodes) {
            node.offset = dirOffset;
            dirOffset += dirEntrySize(node);
        }
        int dirTableSize = dirOffset;
        int fileOffset = 0;
        for (RomfsNode node : nodes) {
            for (String name : node.files) {
                int size = 0x80 + random.nextInt(options.level3Log2 == 9 ? 900 : 3000);
                FileRecord record = new FileRecord(name, node, Bin.noise(random, size));
                record.metadataOffset = fileOffset;
                fileOffset += fileEntrySize(name);
                records.add(record);
            }
        }
        int fileTableSize = fileOffset;
        List<FileRecord> dataOrder = new ArrayList<>(records);
        if (options.shuffleData) {
            Collections.shuffle(dataOrder, random);
        }
        long cursor = 0;
        for (FileRecord record : dataOrder) {
            record.dataOffset = cursor;
            cursor = Bin.align((int) (cursor + record.data.length), 16);
        }
        long dataSize = cursor;

        byte[] dirTable = new byte[dirTableSize];
        for (RomfsNode node : nodes) {
            RomfsNode parent = nodes.stream().filter(p -> p.children.contains(node)).findFirst().orElse(root);
            int siblingIndex = parent.children.indexOf(node);
            int sibling = node == root || siblingIndex + 1 >= parent.children.size() ? -1 : parent.children.get(siblingIndex + 1).offset;
            int child = node.children.isEmpty() ? -1 : node.children.get(0).offset;
            int firstFile = -1;
            for (FileRecord record : records) {
                if (record.parent == node) {
                    firstFile = record.metadataOffset;
                    break;
                }
            }
            byte[] name = Bin.utf16(node.name);
            Bin.writeInt(dirTable, node.offset, parent.offset);
            Bin.writeInt(dirTable, node.offset + 4, sibling);
            Bin.writeInt(dirTable, node.offset + 8, child);
            Bin.writeInt(dirTable, node.offset + 12, firstFile);
            Bin.writeInt(dirTable, node.offset + 16, -1);
            Bin.writeInt(dirTable, node.offset + 20, name.length);
            Bin.put(dirTable, node.offset + 0x18, name);
        }
        byte[] fileTable = new byte[fileTableSize];
        for (int i = 0; i < records.size(); i++) {
            FileRecord record = records.get(i);
            FileRecord next = i + 1 < records.size() && records.get(i + 1).parent == record.parent ? records.get(i + 1) : null;
            byte[] name = Bin.utf16(record.name);
            int at = record.metadataOffset;
            Bin.writeInt(fileTable, at, record.parent.offset);
            Bin.writeInt(fileTable, at + 4, next == null ? -1 : next.metadataOffset);
            Bin.writeLong(fileTable, at + 8, record.dataOffset);
            Bin.writeLong(fileTable, at + 16, record.data.length);
            Bin.writeInt(fileTable, at + 24, -1);
            Bin.writeInt(fileTable, at + 28, name.length);
            Bin.put(fileTable, at + 0x20, name);
        }

        int dirHashSize = 4 * 3;
        int fileHashSize = 4 * 5;
        int headerEnd = 0x28;
        int dirHashOffset = headerEnd;
        int dirMetaOffset = dirHashOffset + dirHashSize;
        int fileHashOffset = dirMetaOffset + dirTableSize;
        int fileMetaOffset = fileHashOffset + fileHashSize;
        int fileDataOffset = Bin.align(fileMetaOffset + fileTableSize, 16);
        int level3Size = (int) (fileDataOffset + dataSize);
        byte[] level3 = new byte[level3Size];
        Bin.writeInt(level3, 0, 0x28);
        Bin.writeInt(level3, 4, dirHashOffset);
        Bin.writeInt(level3, 8, dirHashSize);
        Bin.writeInt(level3, 12, dirMetaOffset);
        Bin.writeInt(level3, 16, dirTableSize);
        Bin.writeInt(level3, 20, fileHashOffset);
        Bin.writeInt(level3, 24, fileHashSize);
        Bin.writeInt(level3, 28, fileMetaOffset);
        Bin.writeInt(level3, 32, fileTableSize);
        Bin.writeInt(level3, 36, fileDataOffset);
        Bin.put(level3, dirHashOffset, Bin.noise(random, dirHashSize));
        Bin.put(level3, dirMetaOffset, dirTable);
        Bin.put(level3, fileHashOffset, Bin.noise(random, fileHashSize));
        Bin.put(level3, fileMetaOffset, fileTable);
        for (FileRecord record : records) {
            Bin.put(level3, (int) (fileDataOffset + record.dataOffset), record.data);
        }

        int blockSize = 1 << options.level3Log2;
        int masterHashSize = 0x20;
        int level3Start = Bin.align(0x60 + masterHashSize, blockSize);
        int total = Bin.align(level3Start + level3Size, blockSize) + 2 * blockSize;
        byte[] romfs = new byte[total];
        if (options.validRomfs) {
            Bin.writeIntBigEndian(romfs, 0, 0x49564643);
            Bin.writeIntBigEndian(romfs, 4, 0x00000100);
        } else {
            Bin.put(romfs, 0, Bin.noise(random, 8));
        }
        Bin.writeInt(romfs, 8, masterHashSize);
        Bin.writeLong(romfs, 0x0C, 0);
        Bin.writeLong(romfs, 0x14, 0x20);
        Bin.writeInt(romfs, 0x1C, options.level1Log2 < 0 ? options.level3Log2 : options.level1Log2);
        Bin.writeLong(romfs, 0x24, 0x200);
        Bin.writeLong(romfs, 0x2C, 0x20);
        Bin.writeInt(romfs, 0x34, options.level2Log2 < 0 ? options.level3Log2 : options.level2Log2);
        Bin.writeLong(romfs, 0x3C, 0x1000);
        Bin.writeLong(romfs, 0x44, level3Size);
        Bin.writeInt(romfs, 0x4C, options.level3Log2);
        Bin.writeInt(romfs, 0x54, 0x5C);
        Bin.put(romfs, 0x60, Bin.noise(random, masterHashSize));
        Bin.put(romfs, level3Start, level3);
        Bin.put(romfs, total - blockSize, Bin.noise(random, blockSize));
        return romfs;
    }

    private static byte[] exefs(Random random, Options options, byte[] code, byte[] icon) {
        byte[] banner = Bin.noise(random, 0x2A0);
        List<Object[]> entries = new ArrayList<>();
        if (options.iconFirst) {
            entries.add(new Object[] {"icon", icon});
            entries.add(new Object[] {".code", code});
            entries.add(new Object[] {"banner", banner});
        } else {
            entries.add(new Object[] {".code", code});
            entries.add(new Object[] {"banner", banner});
            entries.add(new Object[] {"icon", icon});
        }
        ByteArrayOutputStream body = new ByteArrayOutputStream();
        byte[] header = new byte[0x200];
        for (int i = 0; i < entries.size(); i++) {
            String name = (String) entries.get(i)[0];
            byte[] data = (byte[]) entries.get(i)[1];
            int offset = body.size();
            Bin.put(header, i * 16, name.getBytes(StandardCharsets.US_ASCII));
            Bin.writeInt(header, i * 16 + 8, offset);
            Bin.writeInt(header, i * 16 + 12, data.length);
            body.writeBytes(data);
            while (body.size() % UNIT != 0) {
                body.write(0);
            }
            Bin.put(header, 0x200 - (i + 1) * 0x20, Bin.noise(random, 0x20));
        }
        ByteArrayOutputStream exefs = new ByteArrayOutputStream();
        exefs.writeBytes(header);
        exefs.writeBytes(body.toByteArray());
        return exefs.toByteArray();
    }

    private static byte[] ncch(Random random, Options options, List<FileRecord> records, byte[] plainCode) {
        byte[] code = options.codeCompressed ? new BLZCoder(null).BLZ_EncodePub(plainCode, false, true, ".code") : plainCode;
        byte[] icon = MiscVectors.syntheticSmdh(random, true);
        byte[] exefs = exefs(random, options, code, icon);
        byte[] romfs = romfs(random, options, records);
        ByteArrayOutputStream out = new ByteArrayOutputStream();
        byte[] header = new byte[0xA00];
        Bin.put(header, 0, Bin.noise(random, 0x100));
        Bin.put(header, 0x100, Bin.ascii("NCCH"));
        Bin.writeLong(header, 0x108, 0x000400000011C500L);
        Bin.put(header, 0x110, Bin.ascii("01"));
        Bin.writeShort(header, 0x112, 2);
        Bin.writeLong(header, 0x118, 0x000400000011C500L);
        Bin.put(header, 0x150, Bin.ascii("CTR-P-ECLA"));
        Bin.writeInt(header, 0x180, 0x400);
        byte[] flags = Bin.noise(random, 8);
        flags[7] = (byte) (options.noCrypto ? 4 : 0);
        Bin.put(header, 0x188, flags);
        Bin.put(header, 0x200, Bin.noise(random, 0x800));
        header[0x20D] = (byte) ((header[0x20D] & ~1) | (options.codeCompressed ? 1 : 0));
        out.writeBytes(header);
        if (options.logo) {
            Bin.writeInt(header, 0x198, out.size() / UNIT);
            Bin.writeInt(header, 0x19C, 1);
            out.writeBytes(Bin.noise(random, UNIT));
        }
        if (options.plain) {
            Bin.writeInt(header, 0x190, out.size() / UNIT);
            Bin.writeInt(header, 0x194, 1);
            out.writeBytes(Bin.noise(random, UNIT));
        }
        Bin.writeInt(header, 0x1A0, out.size() / UNIT);
        Bin.writeInt(header, 0x1A4, exefs.length / UNIT);
        Bin.writeInt(header, 0x1A8, 1);
        out.writeBytes(exefs);
        out.writeBytes(new byte[UNIT * 3]);
        Bin.writeInt(header, 0x1B0, out.size() / UNIT);
        Bin.writeInt(header, 0x1B4, Bin.align(romfs.length, UNIT) / UNIT);
        Bin.writeInt(header, 0x1B8, 1);
        out.writeBytes(romfs);
        while (out.size() % UNIT != 0) {
            out.write(0);
        }
        byte[] image = out.toByteArray();
        Bin.writeInt(header, 0x104, image.length / UNIT);
        System.arraycopy(header, 0, image, 0, header.length);
        return image;
    }

    private static byte[] wrap(Random random, Options options, byte[] ncch) {
        if (options.container.equals("cci")) {
            byte[] image = new byte[0x4000 + ncch.length];
            Bin.put(image, 0, Bin.noise(random, 0x100));
            Bin.put(image, 0x100, Bin.ascii("NCSD"));
            Bin.put(image, 0x4000, ncch);
            return image;
        }
        if (options.container.equals("cia")) {
            int certSize = 0xA00;
            int ticketSize = 0x350;
            int tmdSize = 0xB34;
            int certOffset = Bin.align(0x2020, 64);
            int ticketOffset = Bin.align(certOffset + certSize, 64);
            int tmdOffset = Bin.align(ticketOffset + ticketSize, 64);
            int contentOffset = Bin.align(tmdOffset + tmdSize, 64);
            byte[] image = new byte[contentOffset + ncch.length];
            Bin.writeInt(image, 0, 0x2020);
            Bin.writeInt(image, 8, certSize);
            Bin.writeInt(image, 0xC, ticketSize);
            Bin.writeInt(image, 0x10, tmdSize);
            Bin.writeLong(image, 0x18, ncch.length);
            Bin.put(image, certOffset, Bin.noise(random, certSize));
            Bin.put(image, ticketOffset, Bin.noise(random, ticketSize));
            Bin.put(image, tmdOffset, Bin.noise(random, tmdSize));
            Bin.writeIntBigEndian(image, tmdOffset, 0x00010004);
            int tmdHeader = tmdOffset + 4 + 0x100 + 0x3C;
            image[tmdHeader + 0x9C] = 0x1C;
            image[tmdHeader + 0x9D] = 0x40;
            Bin.put(image, contentOffset, ncch);
            return image;
        }
        return ncch;
    }

    private interface NcchOperation {
        Object apply(NCCH ncch) throws Exception;
    }

    private static Map<String, Object> op(List<Object> log, NCCH ncch, Map<String, Object> description, NcchOperation operation) {
        description.put("outcome", Guard.run(() -> operation.apply(ncch)));
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
        Options first = new Options();
        first.codeCompressed = true;
        first.noCrypto = true;
        first.logo = true;
        first.plain = true;
        first.shuffleData = true;
        cases.add(runCase(workDir, "ncch_compressed", 0x1CC, first, true));
        Options second = new Options();
        second.container = "cci";
        second.battleCro = true;
        second.level3Log2 = 12;
        second.iconFirst = true;
        cases.add(runCase(workDir, "cci_plain", 0x2CC, second, true));
        Options mismatched = new Options();
        mismatched.container = "cci";
        mismatched.battleCro = true;
        mismatched.level3Log2 = 12;
        mismatched.level1Log2 = 9;
        cases.add(runCase(workDir, "cci_mismatched_blocks", 0x5CC, mismatched, true));
        Options third = new Options();
        third.container = "cia";
        third.codeCompressed = true;
        third.noCrypto = true;
        third.battleCro = true;
        cases.add(runCase(workDir, "cia_update", 0x3CC, third, false));
        Options fourth = new Options();
        fourth.validRomfs = false;
        cases.add(runCase(workDir, "encrypted_like", 0x4CC, fourth, false));
        Map<String, Object> root = Json.object();
        root.put("cases", cases);
        return root;
    }

    private static Map<String, Object> runCase(Path workDir, String name, long seed, Options options, boolean saveNcch) throws Exception {
        Random random = new Random(seed);
        List<FileRecord> records = new ArrayList<>();
        byte[] plainCode = Bin.structured(random, 0x3000);
        byte[] image = wrap(random, options, ncch(random, options, records, plainCode));
        Path romPath = workDir.resolve(name + (options.container.equals("cia") ? ".cia" : ".3ds"));
        Files.write(romPath, image);
        Map<String, Object> vector = Json.object();
        vector.put("name", name);
        vector.put("file_name", romPath.getFileName().toString());
        vector.put("input", image);
        vector.put("cxi_offset", NCCH.getCXIOffsetInFile(romPath.toString()));
        Method productCodeReader = Abstract3DSRomHandler.class.getDeclaredMethod("getProductCodeFromFile", String.class);
        productCodeReader.setAccessible(true);
        String productCode = (String) productCodeReader.invoke(null, romPath.toString());
        String titleId = Abstract3DSRomHandler.getTitleIdFromFile(romPath.toString());
        vector.put("product_code", productCode);
        vector.put("title_id", titleId);
        Map<String, Object> opened = Guard.run(() -> new NCCH(romPath.toString(), productCode, titleId));
        vector.put("open", opened.get("result"));
        if (!"ok".equals(opened.get("result"))) {
            vector.put("open_error", opened.get("error"));
            return vector;
        }
        NCCH ncch = (NCCH) opened.get("value");
        opened.remove("value");
        List<Object> log = new ArrayList<>();
        op(log, ncch, describe("version"), NCCH::getVersion);
        op(log, ncch, describe("is_decrypted"), NCCH::isDecrypted);
        op(log, ncch, describe("has_file", "path", "a/0/1"), n -> n.hasFile("a/0/1"));
        op(log, ncch, describe("read_file", "path", "a/0/1"), n -> n.getFile("a/0/1"));
        op(log, ncch, describe("read_file", "path", "sound/bgm.bcsar"), n -> n.getFile("sound/bgm.bcsar"));
        op(log, ncch, describe("read_file", "path", "nope"), n -> n.getFile("nope"));
        op(log, ncch, describe("read_code"), NCCH::getCode);
        if (records.size() > 2) {
            byte[] bigger = Bin.noise(random, records.get(2).data.length + 0x333);
            op(log, ncch, describe("write_file", "path", "a/0/0", "data", bigger), n -> {
                n.writeFile("a/0/0", bigger);
                return true;
            });
            byte[] smaller = Bin.noise(random, 0x21);
            op(log, ncch, describe("write_file", "path", "zz.bin", "data", smaller), n -> {
                n.writeFile("zz.bin", smaller);
                return true;
            });
            byte[] code = plainCode.clone();
            for (int i = 0; i < code.length; i += 101) {
                code[i] ^= 0x33;
            }
            byte[] longerCode = Arrays.copyOf(code, code.length + 0x40);
            op(log, ncch, describe("write_code", "data", longerCode), n -> {
                n.writeCode(longerCode);
                return true;
            });
            op(log, ncch, describe("read_code"), NCCH::getCode);
        }
        vector.put("operations", log);
        if (saveNcch) {
            Path outPath = workDir.resolve(name + "_out.3ds");
            Files.deleteIfExists(outPath);
            long randomizerSeed = 1234567890123L + seed;
            vector.put("seed", randomizerSeed);
            vector.put("acronym", "ORAS");
            vector.put("saved", Guard.run(() -> {
                ncch.saveAsNCCH(outPath.toString(), "ORAS", randomizerSeed);
                return Files.readAllBytes(outPath);
            }));
        }
        Path layeredRoot = workDir.resolve(name + "_layered");
        vector.put("layered_fs", Guard.run(() -> {
            ncch.saveAsLayeredFS(layeredRoot.toString());
            List<Object> files = new ArrayList<>();
            List<Path> paths;
            try (Stream<Path> walk = Files.walk(layeredRoot)) {
                paths = walk.filter(Files::isRegularFile).sorted().toList();
            }
            for (Path path : paths) {
                Map<String, Object> entry = Json.object();
                entry.put("path", layeredRoot.relativize(path).toString().replace('\\', '/'));
                entry.put("data", Files.readAllBytes(path));
                files.add(entry);
            }
            return files;
        }));
        ByteArrayOutputStream diagnostics = new ByteArrayOutputStream();
        vector.put("diagnostics_outcome", Guard.run(() -> {
            ncch.printRomDiagnostics(new PrintStream(diagnostics, true, StandardCharsets.UTF_8), null);
            return true;
        }));
        vector.put("diagnostics", new ArrayList<Object>(Arrays.asList(diagnostics.toString(StandardCharsets.UTF_8).split("\\R"))));
        return vector;
    }
}
