import com.dabomstew.pkrandom.Version;

import java.nio.ByteBuffer;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.Random;

final class RnqsVectors {
    private static final Random RANDOM = new Random(0x0B5E55EDL);

    private RnqsVectors() {
    }

    static String generate(List<UpdaterVectors.OldString> oldStrings) throws Exception {
        List<byte[]> inputs = new ArrayList<>();
        for (UpdaterVectors.OldString old : oldStrings) {
            if (RANDOM.nextInt(3) == 0) {
                inputs.add(file(old.version, old.string));
            }
        }
        String sample = oldStrings.get(oldStrings.size() - 1).string;
        int[] headerVersions = {100 << 24, 172 << 24, 173 << 24, (255 << 24) | 5, 0x80000000, -1, Version.VERSION + 1,
                Version.VERSION, 0, 0x00FFFFFF};
        for (int version : headerVersions) {
            inputs.add(file(version, sample));
        }
        byte[] valid = file(Version.VERSION - 1, sample);
        for (int length = 0; length < 9; length++) {
            inputs.add(Arrays.copyOf(valid, length));
        }
        inputs.add(Arrays.copyOf(valid, valid.length - 3));
        inputs.add(withLength(valid, valid.length - 8 + 4));
        inputs.add(withLength(valid, valid.length - 8 - 4));
        inputs.add(withLength(valid, -1));
        inputs.add(withLength(valid, 0));
        byte[] extended = Arrays.copyOf(valid, valid.length + 5);
        inputs.add(extended);
        byte[] nonAscii = Arrays.copyOf(valid, valid.length);
        nonAscii[12] = (byte) 0xC3;
        nonAscii[13] = (byte) 0xA9;
        inputs.add(nonAscii);
        List<String> cases = new ArrayList<>();
        for (byte[] input : inputs) {
            cases.add(Json.obj().raw("bytes", Json.base64(input)).raw("read", Outcomes.readRnqs(input)).toString());
        }
        return Json.obj().raw("cases", Json.array(cases)).toString();
    }

    private static byte[] file(int version, String settings) {
        byte[] body = settings.getBytes(StandardCharsets.UTF_8);
        ByteBuffer buffer = ByteBuffer.allocate(body.length + 8);
        buffer.putInt(version);
        buffer.putInt(body.length);
        buffer.put(body);
        return buffer.array();
    }

    private static byte[] withLength(byte[] file, int length) {
        byte[] copy = Arrays.copyOf(file, file.length);
        ByteBuffer.wrap(copy).putInt(4, length);
        return copy;
    }
}
