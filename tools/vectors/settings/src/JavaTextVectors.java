import java.io.ByteArrayInputStream;
import java.io.InputStreamReader;
import java.io.Reader;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.List;
import java.util.Random;

final class JavaTextVectors {
    private JavaTextVectors() {
    }

    static String generate() throws Exception {
        Random random = new Random(0x7E77L);
        List<String> decodes = new ArrayList<>();
        for (int i = 0; i < 1500; i++) {
            byte[] bytes = CustomNamesVectors.trickyBytes(random, random.nextInt(12));
            decodes.add(Json.obj()
                    .raw("bytes", Json.base64(bytes))
                    .raw("string_units", Json.units(new String(bytes, StandardCharsets.UTF_8)))
                    .raw("reader_units", Json.units(readerDecode(bytes)))
                    .toString());
        }
        List<String> digits = new ArrayList<>();
        for (int c = 0; c <= 0xFFFF; c++) {
            int digit = Character.digit((char) c, 10);
            if (digit >= 0) {
                digits.add("[" + c + "," + digit + "]");
            }
        }
        List<String> parses = new ArrayList<>();
        String[] samples = {"322", "000", "099", "+32", "-12", "+-1", "--1", "+", "-", "1+2", " 32", "32 ", "3a2",
                "٣٢٢", "３２２", "๓๒๑", "٣٢x", "1²³",
                "ⅣⅡⅡ", "abc", "1e2", "0x1", "\u0000\u0000\u0000"};
        for (String sample : samples) {
            Json.Obj out = Json.obj().raw("units", Json.units(sample));
            try {
                out.put("value", Integer.parseInt(sample));
            } catch (NumberFormatException error) {
                out.put("error", Outcomes.errorName(error));
            }
            parses.add(out.toString());
        }
        return Json.obj()
                .raw("utf8_decodes", Json.array(decodes))
                .raw("decimal_digits", "[" + String.join(",", digits) + "]")
                .raw("parse_int", Json.array(parses))
                .toString();
    }

    private static String readerDecode(byte[] bytes) throws Exception {
        StringBuilder out = new StringBuilder();
        try (Reader reader = new InputStreamReader(new ByteArrayInputStream(bytes), StandardCharsets.UTF_8)) {
            int c;
            while ((c = reader.read()) != -1) {
                out.append((char) c);
            }
        }
        return out.toString();
    }
}
