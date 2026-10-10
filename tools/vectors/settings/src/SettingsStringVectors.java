import com.dabomstew.pkrandom.FileFunctions;
import com.dabomstew.pkrandom.Settings;
import com.dabomstew.pkrandom.SysConstants;

import java.util.ArrayList;
import java.util.List;
import java.util.Random;

final class SettingsStringVectors {
    private final List<String> romNames;
    private final List<String> encoded = new ArrayList<>();
    private final Random random = new Random(0x5E771465L);

    SettingsStringVectors(List<String> romNames) {
        this.romNames = romNames;
    }

    List<String> encodedStrings() {
        return encoded;
    }

    String generate() throws Exception {
        List<String> cases = new ArrayList<>();
        cases.add(describe("default", SettingsReflection.defaults("Pokemon Red (U)")));
        for (String romName : romNames) {
            cases.add(describe("rom " + romName, SettingsReflection.random(random, romName, false)));
        }
        for (String romName : RomNames.unusual()) {
            cases.add(describe("unusual rom name " + romName.length(), SettingsReflection.random(random, romName, false)));
        }
        for (int i = 0; i < 150; i++) {
            String romName = romNames.get(random.nextInt(romNames.size()));
            cases.add(describe("plausible " + i, SettingsReflection.random(random, romName, false)));
        }
        for (int i = 0; i < 60; i++) {
            String romName = romNames.get(random.nextInt(romNames.size()));
            cases.add(describe("unconstrained " + i, SettingsReflection.random(random, romName, true)));
        }
        return Json.obj()
                .put("custom_names_checksum", FileFunctions.getFileChecksum(SysConstants.customNamesFile))
                .raw("cases", Json.array(cases))
                .toString();
    }

    private String describe(String name, Settings settings) throws Exception {
        String string = settings.toString();
        encoded.add(string);
        return Json.obj()
                .put("name", name)
                .raw("fields", SettingsReflection.dump(settings))
                .put("string", string)
                .raw("rnqs", Json.base64(Outcomes.writeRnqs(settings)))
                .raw("decoded", Outcomes.decode(string))
                .toString();
    }
}
