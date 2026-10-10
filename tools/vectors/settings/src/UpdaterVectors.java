import com.dabomstew.pkrandom.Settings;
import com.dabomstew.pkrandom.SettingsUpdater;
import com.dabomstew.pkrandom.Version;

import java.util.ArrayList;
import java.util.Base64;
import java.util.List;
import java.util.Random;

final class UpdaterVectors {
    static final int[] EXTRA_VERSIONS = {Integer.MIN_VALUE, -1, 0, 50, 101, 103, 115, 155, 165, 171, 173, 200, 299,
            300, 305, 316, 318, 320, 321, 322, 400};

    private static final int[][] INSERTED_BYTES_BEFORE = {{321, 0}, {319, 1}, {317, 1}, {315, 1}, {314, 2}, {311, 6},
            {300, 2}, {172, -7}, {171, 4}, {170, 3}, {162, 1}, {160, 9}, {150, 6}, {110, 1}};

    private final List<String> currentStrings;
    private final List<OldString> oldStrings = new ArrayList<>();
    private final Random random = new Random(0x0BD0DA7EL);

    UpdaterVectors(List<String> currentStrings) {
        this.currentStrings = currentStrings;
    }

    List<OldString> oldStrings() {
        return oldStrings;
    }

    static List<Integer> versions() {
        List<Integer> versions = new ArrayList<>();
        for (int version = 0; version < Version.VERSION; version++) {
            if (Version.oldVersions.containsKey(version)) {
                versions.add(version);
            }
        }
        for (int version : EXTRA_VERSIONS) {
            versions.add(version);
        }
        return versions;
    }

    static int nativeLength(int version, int romNameLength) {
        int length = Settings.LENGTH_OF_SETTINGS_DATA + 1 + romNameLength + 8;
        for (int[] step : INSERTED_BYTES_BEFORE) {
            if (version < step[0]) {
                length -= step[1];
            }
        }
        return length;
    }

    String generate() throws Exception {
        List<String> cases = new ArrayList<>();
        for (int version : versions()) {
            for (int i = 0; i < 12; i++) {
                cases.add(describe(version, nativeBlock(version), true));
            }
            for (int i = 0; i < 4; i++) {
                cases.add(describe(version, randomLengthBlock(), false));
            }
            for (int i = 0; i < 3; i++) {
                String current = currentStrings.get(random.nextInt(currentStrings.size()));
                cases.add(describe(version, Base64.getDecoder().decode(current), false));
            }
        }
        cases.add(describeString(170, "not base64!"));
        cases.add(describeString(170, ""));
        return Json.obj().raw("cases", Json.array(cases)).toString();
    }

    private String describe(int version, byte[] block, boolean remember) throws Exception {
        String input = Base64.getEncoder().encodeToString(block);
        if (remember) {
            oldStrings.add(new OldString(version, input));
        }
        return describeString(version, input);
    }

    private String describeString(int version, String input) throws Exception {
        Json.Obj out = Json.obj().put("version", version).put("input", input);
        String output;
        try {
            output = new SettingsUpdater().update(version, input);
        } catch (Throwable error) {
            return out.put("error", Outcomes.errorName(error)).toString();
        }
        return out.put("output", output).raw("decoded", Outcomes.decode(output)).toString();
    }

    private byte[] nativeBlock(int version) {
        int length = Math.max(0, nativeLength(version, random.nextInt(25)));
        byte[] block = new byte[length];
        int strategy = random.nextInt(3);
        for (int i = 0; i < length; i++) {
            if (strategy == 0) {
                block[i] = (byte) random.nextInt(256);
            } else if (strategy == 1) {
                block[i] = random.nextBoolean() ? 0 : (byte) (1 << random.nextInt(8));
            } else {
                block[i] = random.nextInt(4) == 0 ? (byte) random.nextInt(256) : 0;
            }
        }
        return block;
    }

    private byte[] randomLengthBlock() {
        byte[] block = new byte[random.nextInt(210)];
        random.nextBytes(block);
        return block;
    }

    static final class OldString {
        final int version;
        final String string;

        OldString(int version, String string) {
            this.version = version;
            this.string = string;
        }
    }
}
