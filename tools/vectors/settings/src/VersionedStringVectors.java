import com.dabomstew.pkrandom.Settings;
import com.dabomstew.pkrandom.SettingsUpdater;
import com.dabomstew.pkrandom.Version;

import java.io.UnsupportedEncodingException;
import java.util.ArrayList;
import java.util.List;

final class VersionedStringVectors {
    private VersionedStringVectors() {
    }

    static String generate(List<String> currentStrings, List<UpdaterVectors.OldString> oldStrings) throws Exception {
        List<String> inputs = new ArrayList<>();
        for (int i = 0; i < Math.min(40, currentStrings.size()); i++) {
            inputs.add(Version.VERSION + currentStrings.get(i));
        }
        inputs.add("  " + Version.VERSION + currentStrings.get(0) + " \t\n");
        inputs.add("323" + currentStrings.get(0));
        inputs.add("999" + currentStrings.get(0));
        inputs.add("+32" + currentStrings.get(0));
        inputs.add("-12" + currentStrings.get(0));
        inputs.add("٣٢٢" + currentStrings.get(0));
        inputs.add("３２１" + currentStrings.get(0));
        inputs.add("32a" + currentStrings.get(0));
        inputs.add("3 2" + currentStrings.get(0));
        inputs.add("");
        inputs.add("   ");
        inputs.add("3");
        inputs.add("32");
        inputs.add("322");
        inputs.add("321");
        inputs.add("abc");
        for (UpdaterVectors.OldString old : oldStrings) {
            if (old.version >= 100 && old.version <= 999) {
                inputs.add(old.version + old.string);
            }
        }
        List<String> cases = new ArrayList<>();
        for (String input : inputs) {
            cases.add(Json.obj().put("input", input).raw("outcome", load(input)).toString());
        }
        return Json.obj().raw("cases", Json.array(cases)).toString();
    }

    private static String load(String text) throws Exception {
        String configString = text.trim();
        if (configString.length() == 0) {
            return Json.obj().put("result", "empty").toString();
        }
        if (configString.length() < 3) {
            return Json.obj().put("result", "invalid").toString();
        }
        try {
            int version = Integer.parseInt(configString.substring(0, 3));
            if (version < Version.VERSION) {
                String updated = new SettingsUpdater().update(version, configString.substring(3));
                Settings settings = Settings.fromString(updated);
                return Json.obj().put("result", "updated").put("version", version)
                        .raw("fields", SettingsReflection.dump(settings)).toString();
            } else if (version > Version.VERSION) {
                return Json.obj().put("result", "too_new").put("version", version).toString();
            } else {
                Settings settings = Settings.fromString(configString.substring(3));
                return Json.obj().put("result", "loaded").put("version", version)
                        .raw("fields", SettingsReflection.dump(settings)).toString();
            }
        } catch (UnsupportedEncodingException | IllegalArgumentException error) {
            return Json.obj().put("result", "invalid").put("error", Outcomes.errorName(error)).toString();
        } catch (Throwable error) {
            return Json.obj().put("result", "crash").put("error", Outcomes.errorName(error)).toString();
        }
    }
}
