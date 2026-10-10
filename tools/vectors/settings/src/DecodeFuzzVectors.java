import java.nio.ByteBuffer;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Base64;
import java.util.List;
import java.util.Random;
import java.util.zip.CRC32;

final class DecodeFuzzVectors {
    private static final Random RANDOM = new Random(0xDEC0DEL);

    private DecodeFuzzVectors() {
    }

    static String generate(List<String> validStrings) throws Exception {
        List<String> inputs = new ArrayList<>();
        for (int i = 0; i < 400; i++) {
            inputs.add(encode(mutatedBody(validStrings)));
        }
        for (int i = 0; i < 150; i++) {
            inputs.add(encode(randomSettingsBody()));
        }
        for (int i = 0; i < 80; i++) {
            inputs.add(encode(randomBytes(RANDOM.nextInt(70))));
        }
        for (int i = 0; i < 40; i++) {
            inputs.add(Base64.getEncoder().encodeToString(randomBytes(RANDOM.nextInt(80))));
        }
        inputs.addAll(malformedBase64(validStrings.get(1)));
        List<String> cases = new ArrayList<>();
        for (String input : inputs) {
            cases.add(Json.obj().put("input", input).raw("decoded", Outcomes.decode(input)).toString());
        }
        return Json.obj().raw("cases", Json.array(cases)).toString();
    }

    static byte[] withChecksums(byte[] body, int namesChecksum) {
        CRC32 checksum = new CRC32();
        checksum.update(body);
        ByteBuffer buffer = ByteBuffer.allocate(body.length + 8);
        buffer.put(body);
        buffer.putInt((int) checksum.getValue());
        buffer.putInt(namesChecksum);
        return buffer.array();
    }

    static byte[] randomBytes(int length) {
        byte[] bytes = new byte[length];
        RANDOM.nextBytes(bytes);
        return bytes;
    }

    private static String encode(byte[] body) {
        return Base64.getEncoder().encodeToString(withChecksums(body, RANDOM.nextInt()));
    }

    private static byte[] mutatedBody(List<String> validStrings) {
        byte[] data = Base64.getDecoder().decode(validStrings.get(RANDOM.nextInt(validStrings.size())));
        byte[] body = Arrays.copyOf(data, data.length - 8);
        int flips = 1 + RANDOM.nextInt(4);
        for (int i = 0; i < flips; i++) {
            int index = RANDOM.nextInt(51);
            body[index] ^= (byte) (1 << RANDOM.nextInt(8));
        }
        return body;
    }

    private static byte[] randomSettingsBody() {
        int nameLength = RANDOM.nextInt(40);
        byte[] body = new byte[52 + nameLength];
        RANDOM.nextBytes(body);
        if (RANDOM.nextBoolean()) {
            body[51] = (byte) nameLength;
        }
        return body;
    }

    private static List<String> malformedBase64(String valid) {
        List<String> inputs = new ArrayList<>();
        String unpadded = valid.replace("=", "");
        inputs.add("");
        inputs.add("A");
        inputs.add("AA");
        inputs.add("AA=");
        inputs.add("AA==");
        inputs.add("AAA=");
        inputs.add("====");
        inputs.add("AA==AA==");
        inputs.add(unpadded);
        inputs.add(valid + "=");
        inputs.add(valid + "A");
        inputs.add(valid + "AA");
        inputs.add(valid + " ");
        inputs.add(" " + valid);
        inputs.add(valid.substring(0, 10) + "\n" + valid.substring(10));
        inputs.add(valid.replace('A', '-'));
        inputs.add(valid.replace('/', '_'));
        inputs.add(valid.replace('+', '*'));
        inputs.add(valid.substring(0, 5) + "é" + valid.substring(6));
        inputs.add(valid.substring(0, 5) + "€" + valid.substring(6));
        for (int cut = 1; cut < 12; cut++) {
            inputs.add(unpadded.substring(0, unpadded.length() - cut));
        }
        String[] tails = {"Q", "Qg", "Qh", "Qi", "QUI", "QUJ", "QUL", "Qg=", "QUI=", "Q===", "QUJD", "QUJDRA"};
        for (String tail : tails) {
            inputs.add(unpadded.substring(0, unpadded.length() - unpadded.length() % 4) + tail);
        }
        return inputs;
    }
}
