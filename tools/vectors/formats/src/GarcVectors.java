import com.dabomstew.pkrandom.ctr.GARCArchive;
import cuecompressors.BLZCoder;

import java.io.ByteArrayOutputStream;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.Random;
import java.util.TreeMap;

public final class GarcVectors {
    private GarcVectors() {
    }

    private static byte[] lzss(byte[] data) {
        return new BLZCoder(null).BLZ_EncodePub(data, false, false, "GARC");
    }

    public static byte[] build(int version, List<TreeMap<Integer, byte[]>> directories, int padTo, int paddingValue) {
        int headerSize = version == 4 ? 0x1C : 0x24;
        int fatoSize = 12 + 4 * directories.size();
        int fatbSize = 12;
        for (TreeMap<Integer, byte[]> directory : directories) {
            fatbSize += 4 + 12 * directory.size();
        }
        int dataOffset = headerSize + fatoSize + fatbSize + 12;
        byte[] fato = new byte[fatoSize];
        byte[] fatb = new byte[fatbSize];
        Bin.put(fato, 0, Bin.ascii("OTAF"));
        Bin.writeInt(fato, 4, fatoSize);
        Bin.writeShort(fato, 8, directories.size());
        Bin.writeShort(fato, 10, paddingValue);
        Bin.put(fatb, 0, Bin.ascii("BTAF"));
        Bin.writeInt(fatb, 4, fatbSize);
        Bin.writeInt(fatb, 8, directories.size());
        ByteArrayOutputStream payload = new ByteArrayOutputStream();
        int fatbCursor = 12;
        int largest = 0;
        for (int index = 0; index < directories.size(); index++) {
            TreeMap<Integer, byte[]> directory = directories.get(index);
            Bin.writeInt(fato, 12 + index * 4, fatbCursor - 12);
            int vector = 0;
            for (int bit : directory.keySet()) {
                vector |= 1 << bit;
            }
            Bin.writeInt(fatb, fatbCursor, vector);
            fatbCursor += 4;
            for (byte[] file : directory.values()) {
                int start = payload.size();
                payload.writeBytes(file);
                Bin.writeInt(fatb, fatbCursor, start);
                Bin.writeInt(fatb, fatbCursor + 4, start + file.length);
                Bin.writeInt(fatb, fatbCursor + 8, file.length);
                fatbCursor += 12;
                largest = Math.max(largest, file.length);
                while (payload.size() % padTo != 0) {
                    payload.write(paddingValue);
                }
            }
        }
        byte[] data = payload.toByteArray();
        byte[] header = new byte[headerSize];
        Bin.put(header, 0, Bin.ascii("CRAG"));
        Bin.writeInt(header, 4, headerSize);
        Bin.writeShort(header, 8, 0xFEFF);
        Bin.writeShort(header, 10, version == 4 ? 0x0400 : 0x0600);
        Bin.writeInt(header, 12, 4);
        Bin.writeInt(header, 16, dataOffset);
        Bin.writeInt(header, 20, dataOffset + data.length);
        if (version == 4) {
            Bin.writeInt(header, 24, largest);
        } else {
            Bin.writeInt(header, 24, Bin.align(largest, padTo));
            Bin.writeInt(header, 28, largest);
            Bin.writeInt(header, 32, padTo);
        }
        byte[] fimb = new byte[12];
        Bin.put(fimb, 0, Bin.ascii("BMIF"));
        Bin.writeInt(fimb, 4, 12);
        Bin.writeInt(fimb, 8, data.length);
        ByteArrayOutputStream out = new ByteArrayOutputStream();
        out.writeBytes(header);
        out.writeBytes(fato);
        out.writeBytes(fatb);
        out.writeBytes(fimb);
        out.writeBytes(data);
        return out.toByteArray();
    }

    private static TreeMap<Integer, byte[]> single(byte[] data) {
        TreeMap<Integer, byte[]> directory = new TreeMap<>();
        directory.put(0, data);
        return directory;
    }

    public static Map<String, Object> vectors() {
        Random random = new Random(0x6A2C);
        List<Object> cases = new ArrayList<>();

        List<TreeMap<Integer, byte[]>> plain = new ArrayList<>();
        plain.add(single(lzss(Bin.structured(random, 400))));
        plain.add(single(Bin.noise(random, 33)));
        plain.add(single(lzss(Bin.structured(random, 1500))));
        plain.add(single(new byte[] {0x11, 0x05, 0, 0, 0x00, 'a', 'b', 'c', 'd', 'e', 1, 2, 3, 4, 5, 6, 7, 8, 9, 10}));
        plain.add(single(Bin.structured(random, 64)));
        cases.add(garcCase(build(4, plain, 4, 0xFF), new boolean[] {true, true, false, true, true}));

        List<TreeMap<Integer, byte[]>> folders = new ArrayList<>();
        TreeMap<Integer, byte[]> folder = new TreeMap<>();
        folder.put(0, lzss(Bin.structured(random, 200)));
        folder.put(1, lzss(Bin.structured(random, 90)));
        folder.put(3, Bin.noise(random, 17));
        folders.add(folder);
        folders.add(single(lzss(Bin.structured(random, 700))));
        folders.add(new TreeMap<>());
        TreeMap<Integer, byte[]> second = new TreeMap<>();
        second.put(2, Bin.noise(random, 9));
        second.put(5, Bin.noise(random, 21));
        folders.add(second);
        cases.add(garcCase(build(6, folders, 4, 0x00), new boolean[] {true, false, false, true}));

        List<TreeMap<Integer, byte[]>> wide = new ArrayList<>();
        for (int i = 0; i < 6; i++) {
            wide.add(single(i % 2 == 0 ? lzss(Bin.structured(random, 100 + random.nextInt(900))) : Bin.noise(random, 1 + random.nextInt(40))));
        }
        cases.add(garcCase(build(6, wide, 0x80, 0xFF), new boolean[] {false, false, true, true, false, false}));

        List<TreeMap<Integer, byte[]>> broken = new ArrayList<>();
        broken.add(single(lzss(Bin.structured(random, 100))));
        broken.add(single(new byte[] {0x11, 0x10, 0, 0, 0x00, 1, 2}));
        cases.add(garcCase(build(4, broken, 4, 0xFF), new boolean[] {true, false}));

        byte[] wrongMagic = build(4, plain, 4, 0xFF);
        wrongMagic[0] = 'X';
        cases.add(garcCase(wrongMagic, new boolean[] {true}));

        Map<String, Object> root = Json.object();
        root.put("cases", cases);
        return root;
    }

    private static Map<String, Object> garcCase(byte[] input, boolean[] selection) {
        Map<String, Object> vector = Json.object();
        vector.put("input", input);
        List<Object> selected = new ArrayList<>();
        for (boolean flag : selection) {
            selected.add(flag);
        }
        vector.put("selection", selected);
        byte[] replacement = Bin.structured(new Random(7), 333);
        vector.put("replacement", replacement);
        List<Object> modes = new ArrayList<>();
        for (String mode : new String[] {"skip", "all", "selected"}) {
            Map<String, Object> modeVector = Json.object();
            modeVector.put("mode", mode);
            modeVector.put("outcome", Guard.run(() -> {
                GARCArchive garc;
                if (mode.equals("skip")) {
                    garc = new GARCArchive(input, true);
                } else if (mode.equals("all")) {
                    garc = new GARCArchive(input, false);
                } else {
                    List<Boolean> flags = new ArrayList<>();
                    for (boolean flag : selection) {
                        flags.add(flag);
                    }
                    garc = new GARCArchive(input, flags);
                }
                Map<String, Object> result = Json.object();
                result.put("files", describe(garc));
                GARCArchive archive = garc;
                result.put("bytes", Guard.run(archive::getBytes));
                result.put("bytes_after_set", Guard.run(() -> {
                    archive.setFile(0, replacement);
                    return archive.getBytes();
                }));
                return result;
            }));
            modes.add(modeVector);
        }
        vector.put("modes", modes);
        return vector;
    }

    private static List<Object> describe(GARCArchive garc) {
        List<Object> directories = new ArrayList<>();
        for (Map<Integer, byte[]> directory : garc.files) {
            List<Object> entries = new ArrayList<>();
            for (Map.Entry<Integer, byte[]> entry : directory.entrySet()) {
                Map<String, Object> item = Json.object();
                item.put("key", entry.getKey());
                item.put("data", entry.getValue());
                entries.add(item);
            }
            directories.add(entries);
        }
        return directories;
    }
}
