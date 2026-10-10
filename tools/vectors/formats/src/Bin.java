import java.io.ByteArrayOutputStream;
import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.util.Random;

public final class Bin {
    private Bin() {
    }

    public static byte[] lowEntropy(Random random, int length, int alphabet) {
        byte[] data = new byte[length];
        for (int i = 0; i < length; i++) {
            data[i] = (byte) random.nextInt(alphabet);
        }
        return data;
    }

    public static byte[] noise(Random random, int length) {
        byte[] data = new byte[length];
        random.nextBytes(data);
        return data;
    }

    public static byte[] structured(Random random, int length) {
        ByteArrayOutputStream out = new ByteArrayOutputStream();
        while (out.size() < length) {
            int kind = random.nextInt(4);
            if (kind == 0) {
                int run = 1 + random.nextInt(40);
                int value = random.nextInt(256);
                for (int i = 0; i < run; i++) {
                    out.write(value);
                }
            } else if (kind == 1) {
                byte[] current = out.toByteArray();
                if (current.length > 4) {
                    int start = random.nextInt(current.length);
                    int run = Math.min(current.length - start, 1 + random.nextInt(300));
                    out.write(current, start, run);
                }
            } else if (kind == 2) {
                out.write(random.nextInt(256));
            } else {
                byte[] text = ("pokemon" + random.nextInt(1000) + " ").getBytes(StandardCharsets.US_ASCII);
                out.write(text, 0, text.length);
            }
        }
        byte[] full = out.toByteArray();
        byte[] trimmed = new byte[length];
        System.arraycopy(full, 0, trimmed, 0, length);
        return trimmed;
    }

    public static void writeInt(byte[] data, int offset, int value) {
        data[offset] = (byte) value;
        data[offset + 1] = (byte) (value >> 8);
        data[offset + 2] = (byte) (value >> 16);
        data[offset + 3] = (byte) (value >> 24);
    }

    public static int readInt(byte[] data, int offset) {
        return (data[offset] & 0xFF) | ((data[offset + 1] & 0xFF) << 8) | ((data[offset + 2] & 0xFF) << 16)
                | ((data[offset + 3] & 0xFF) << 24);
    }

    public static void writeLong(byte[] data, int offset, long value) {
        for (int i = 0; i < 8; i++) {
            data[offset + i] = (byte) (value >> (8 * i));
        }
    }

    public static void writeShort(byte[] data, int offset, int value) {
        data[offset] = (byte) value;
        data[offset + 1] = (byte) (value >> 8);
    }

    public static void writeIntBigEndian(byte[] data, int offset, int value) {
        data[offset] = (byte) (value >> 24);
        data[offset + 1] = (byte) (value >> 16);
        data[offset + 2] = (byte) (value >> 8);
        data[offset + 3] = (byte) value;
    }

    public static void put(byte[] data, int offset, byte[] source) {
        System.arraycopy(source, 0, data, offset, source.length);
    }

    public static int align(int value, int alignment) {
        return (value + alignment - 1) & ~(alignment - 1);
    }

    public static String sha256(byte[] data) {
        try {
            byte[] digest = MessageDigest.getInstance("SHA-256").digest(data);
            StringBuilder hex = new StringBuilder();
            for (byte b : digest) {
                hex.append(String.format("%02x", b & 0xFF));
            }
            return hex.toString();
        } catch (Exception e) {
            throw new RuntimeException(e);
        }
    }

    public static byte[] utf16(String text) {
        return text.getBytes(StandardCharsets.UTF_16LE);
    }

    public static byte[] ascii(String text) {
        return text.getBytes(StandardCharsets.US_ASCII);
    }
}
