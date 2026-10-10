import compressors.DSDecmp;
import cuecompressors.BLZCoder;

import java.io.ByteArrayOutputStream;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.Random;

public final class CompressionVectors {
    private CompressionVectors() {
    }

    public static Map<String, Object> blz() {
        Random random = new Random(0xB12);
        List<Object> encodes = new ArrayList<>();
        List<Object> decodes = new ArrayList<>();
        List<byte[]> inputs = new ArrayList<>();
        int[] sizes = {0, 1, 7, 64, 300, 1000, 4096, 7000};
        for (int size : sizes) {
            inputs.add(Bin.structured(random, size));
            inputs.add(Bin.lowEntropy(random, size, 3));
        }
        inputs.add(new byte[5000]);
        inputs.add(Bin.structured(random, 0x4000 + 1500));
        inputs.add(Bin.noise(random, 600));
        for (byte[] input : inputs) {
            for (boolean arm9 : new boolean[] {false, true}) {
                if (arm9 && input.length < 0x4000 && input.length != 0 && input.length != 300) {
                    continue;
                }
                for (boolean best : new boolean[] {false, true}) {
                    Map<String, Object> vector = Json.object();
                    vector.put("input", input);
                    vector.put("arm9", arm9);
                    vector.put("best", best);
                    Map<String, Object> outcome = Guard.run(() -> new BLZCoder(null).BLZ_EncodePub(input, arm9, best, "vector"), 4000);
                    vector.put("outcome", outcome);
                    encodes.add(vector);
                    if ("ok".equals(outcome.get("result"))) {
                        byte[] encoded = (byte[]) outcome.get("value");
                        decodes.add(blzDecodeVector(encoded));
                        if (encoded.length > 16) {
                            byte[] damaged = encoded.clone();
                            damaged[random.nextInt(encoded.length - 12)] ^= (byte) (1 + random.nextInt(255));
                            decodes.add(blzDecodeVector(damaged));
                        }
                    }
                }
            }
        }
        byte[] notCoded = Bin.structured(random, 100);
        Bin.writeInt(notCoded, 96, 0);
        decodes.add(blzDecodeVector(notCoded));
        byte[] badHeader = Bin.structured(random, 64);
        Bin.writeInt(badHeader, 60, 0x100);
        badHeader[59] = 3;
        decodes.add(blzDecodeVector(badHeader));
        decodes.add(blzDecodeVector(new byte[] {1, 2, 3}));
        Map<String, Object> root = Json.object();
        root.put("encode", encodes);
        root.put("decode", decodes);
        return root;
    }

    private static Map<String, Object> blzDecodeVector(byte[] input) {
        Map<String, Object> vector = Json.object();
        vector.put("input", input);
        vector.put("outcome", Guard.run(() -> new BLZCoder(null).BLZ_DecodePub(input, "vector")));
        return vector;
    }

    public static Map<String, Object> lzss() {
        Random random = new Random(0x1255);
        List<Object> encodes = new ArrayList<>();
        List<Object> decodes = new ArrayList<>();
        List<byte[]> inputs = new ArrayList<>();
        for (int size : new int[] {0, 1, 2, 3, 17, 100, 600, 3000, 9000}) {
            inputs.add(Bin.structured(random, size));
            inputs.add(Bin.lowEntropy(random, size, 2));
        }
        inputs.add(new byte[0x11000]);
        inputs.add(Bin.noise(random, 200));
        inputs.add(Bin.noise(random, 20));
        for (byte[] input : inputs) {
            Map<String, Object> vector = Json.object();
            vector.put("input", input);
            Map<String, Object> outcome = Guard.run(() -> new BLZCoder(null).BLZ_EncodePub(input, false, false, "GARC"));
            vector.put("outcome", outcome);
            encodes.add(vector);
            if ("ok".equals(outcome.get("result"))) {
                byte[] encoded = (byte[]) outcome.get("value");
                decodes.add(lzssDecodeVector(encoded));
                byte[] padded = new byte[Bin.align(encoded.length, 4) + 8];
                System.arraycopy(encoded, 0, padded, 0, encoded.length);
                decodes.add(lzssDecodeVector(padded));
                if (encoded.length > 6) {
                    byte[] damaged = encoded.clone();
                    damaged[4 + random.nextInt(encoded.length - 4)] ^= (byte) (1 + random.nextInt(255));
                    decodes.add(lzssDecodeVector(damaged));
                    byte[] truncated = new byte[encoded.length - 2];
                    System.arraycopy(encoded, 0, truncated, 0, truncated.length);
                    decodes.add(lzssDecodeVector(truncated));
                }
            }
        }
        decodes.add(lzssDecodeVector(new byte[] {0x10, 1, 0, 0}));
        decodes.add(lzssDecodeVector(new byte[] {0x11}));
        decodes.add(lzssDecodeVector(new byte[] {0x11, 0, 0, 0, 3, 0, 0, 0, 0, 1, 2, 3}));
        decodes.add(lzssDecodeVector(new byte[] {0x11, 0, 0, 0, (byte) 0xFF, (byte) 0xFF, (byte) 0xFF, (byte) 0xFF}));
        Map<String, Object> root = Json.object();
        root.put("encode", encodes);
        root.put("decode", decodes);
        return root;
    }

    private static Map<String, Object> lzssDecodeVector(byte[] input) {
        Map<String, Object> vector = Json.object();
        vector.put("input", input);
        vector.put("outcome", Guard.run(() -> new BLZCoder(null).BLZ_DecodePub(input, "GARC")));
        return vector;
    }

    public static Map<String, Object> ds() {
        Random random = new Random(0xD5);
        List<Object> cases = new ArrayList<>();
        for (int i = 0; i < 40; i++) {
            byte[] stream = randomLz10(random, 1 + random.nextInt(2000), i % 5 == 0, i % 7 == 3);
            cases.add(dsVector(stream, 0));
        }
        for (int i = 0; i < 40; i++) {
            byte[] stream = randomLz11(random, 1 + random.nextInt(2500), i % 5 == 0);
            cases.add(dsVector(stream, 0));
        }
        byte[] lz = randomLz10(random, 300, false, false);
        byte[] shifted = new byte[lz.length + 17];
        System.arraycopy(lz, 0, shifted, 17, lz.length);
        cases.add(dsVector(shifted, 17));
        cases.add(dsVector(new byte[] {0x12, 0, 0, 0}, 0));
        cases.add(dsVector(new byte[] {0x10, 0, 0, 0, 0, 0, 0, 4, 0, 1, 2, 3, 4}, 0));
        cases.add(dsVector(new byte[] {0x10, 4, 0, 0, (byte) 0x80, 0x00, 0x00}, 0));
        cases.add(dsVector(new byte[] {0x11, 4, 0, 0, (byte) 0x40, 1, 0x20, 0x00}, 0));
        byte[] lz11FromLzss = new BLZCoder(null).BLZ_EncodePub(Bin.structured(random, 5000), false, false, "GARC");
        cases.add(dsVector(lz11FromLzss, 0));
        Map<String, Object> root = Json.object();
        root.put("cases", cases);
        return root;
    }

    private static Map<String, Object> dsVector(byte[] input, int offset) {
        Map<String, Object> vector = Json.object();
        vector.put("input", input);
        vector.put("offset", offset);
        vector.put("outcome", Guard.run(() -> DSDecmp.Decompress(input, offset)));
        return vector;
    }

    private static byte[] randomLz10(Random random, int length, boolean overshootBackref, boolean extendedLength) {
        ByteArrayOutputStream out = new ByteArrayOutputStream();
        out.write(0x10);
        if (extendedLength) {
            out.write(0);
            out.write(0);
            out.write(0);
            out.write(length >>> 24);
            out.write(length >>> 16);
            out.write(length >>> 8);
            out.write(length);
        } else {
            out.write(length);
            out.write(length >> 8);
            out.write(length >> 16);
        }
        int produced = 0;
        while (produced < length) {
            int flags = 0;
            ByteArrayOutputStream block = new ByteArrayOutputStream();
            for (int bit = 0; bit < 8; bit++) {
                boolean finished = produced >= length;
                boolean reference = produced > 1 && random.nextInt(3) != 0;
                if (finished) {
                    block.write(random.nextInt(2) == 0 ? 0 : random.nextInt(256));
                    continue;
                }
                if (reference) {
                    int remaining = length - produced;
                    int count = 3 + random.nextInt(16);
                    if (!overshootBackref) {
                        count = Math.min(count, remaining);
                    }
                    if (count < 3) {
                        block.write(random.nextInt(256));
                        produced++;
                        continue;
                    }
                    int displacement = random.nextInt(Math.min(produced - 1, 0x1000));
                    flags |= 0x80 >> bit;
                    block.write(((count - 3) << 4) | (displacement >> 8));
                    block.write(displacement & 0xFF);
                    produced += count;
                } else {
                    block.write(random.nextInt(256));
                    produced++;
                }
            }
            out.write(flags);
            byte[] blockBytes = block.toByteArray();
            out.write(blockBytes, 0, blockBytes.length);
        }
        return out.toByteArray();
    }

    private static byte[] randomLz11(Random random, int length, boolean overshoot) {
        ByteArrayOutputStream out = new ByteArrayOutputStream();
        out.write(0x11);
        out.write(length);
        out.write(length >> 8);
        out.write(length >> 16);
        int produced = 0;
        while (produced < length) {
            int flags = 0;
            ByteArrayOutputStream block = new ByteArrayOutputStream();
            for (int bit = 0; bit < 8 && produced < length; bit++) {
                if (produced > 2 && random.nextInt(3) != 0) {
                    int form = random.nextInt(3);
                    int count;
                    if (form == 0) {
                        count = 3 + random.nextInt(14);
                    } else if (form == 1) {
                        count = 0x11 + random.nextInt(0x100);
                    } else {
                        count = 0x111 + random.nextInt(0x400);
                    }
                    if (!overshoot) {
                        count = Math.min(count, length - produced);
                        if (count < 3) {
                            block.write(random.nextInt(256));
                            produced++;
                            continue;
                        }
                        form = count <= 0x10 ? 0 : (count <= 0x110 ? 1 : 2);
                    }
                    int displacement = random.nextInt(Math.min(produced - 1, 0x1000));
                    flags |= 0x80 >> bit;
                    if (form == 0) {
                        block.write(((count - 1) << 4) | (displacement >> 8));
                        block.write(displacement & 0xFF);
                    } else if (form == 1) {
                        int biased = count - 0x11;
                        block.write(biased >> 4);
                        block.write(((biased & 0xF) << 4) | (displacement >> 8));
                        block.write(displacement & 0xFF);
                    } else {
                        int biased = count - 0x111;
                        block.write(0x10 | (biased >> 12));
                        block.write((biased >> 4) & 0xFF);
                        block.write(((biased & 0xF) << 4) | (displacement >> 8));
                        block.write(displacement & 0xFF);
                    }
                    produced += count;
                } else {
                    block.write(random.nextInt(256));
                    produced++;
                }
            }
            out.write(flags);
            byte[] blockBytes = block.toByteArray();
            out.write(blockBytes, 0, blockBytes.length);
        }
        return out.toByteArray();
    }
}
