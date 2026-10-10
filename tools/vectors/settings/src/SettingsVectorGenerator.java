import com.dabomstew.pkrandom.Settings;
import com.dabomstew.pkrandom.Version;

import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.ArrayList;
import java.util.List;

public final class SettingsVectorGenerator {
    private SettingsVectorGenerator() {
    }

    public static void main(String[] args) throws Exception {
        String mode = args[0];
        Path outputDirectory = Paths.get(args[1]);
        Path configDirectory = Paths.get(args[2]);
        Files.createDirectories(outputDirectory);
        if (mode.equals("line-separator")) {
            String fileName = args[3];
            write(outputDirectory.resolve(fileName), CustomNamesVectors.serializationVectors(configDirectory));
            return;
        }
        List<String> romNames = RomNames.load(configDirectory);
        SettingsStringVectors settingsStrings = new SettingsStringVectors(romNames);
        write(outputDirectory.resolve("settings_strings.json"), settingsStrings.generate());
        write(outputDirectory.resolve("decode_fuzz.json"), DecodeFuzzVectors.generate(settingsStrings.encodedStrings()));
        UpdaterVectors updater = new UpdaterVectors(settingsStrings.encodedStrings());
        write(outputDirectory.resolve("updater.json"), updater.generate());
        write(outputDirectory.resolve("versioned_strings.json"),
                VersionedStringVectors.generate(settingsStrings.encodedStrings(), updater.oldStrings()));
        write(outputDirectory.resolve("rnqs.json"), RnqsVectors.generate(updater.oldStrings()));
        write(outputDirectory.resolve("presets.json"),
                PresetVectors.generate(settingsStrings.encodedStrings(), configDirectory));
        write(outputDirectory.resolve("custom_names.json"), CustomNamesVectors.generate(configDirectory));
        write(outputDirectory.resolve("java_text.json"), JavaTextVectors.generate());
        write(outputDirectory.resolve("constants.json"), constants());
    }

    private static String constants() throws Exception {
        List<String> oldVersions = new ArrayList<>();
        for (int version = 0; version <= 1000; version++) {
            String name = Version.oldVersions.get(version);
            if (name != null) {
                oldVersions.add(Json.obj().put("version", version).put("name", name).toString());
            }
        }
        return Json.obj()
                .put("version", Version.VERSION)
                .put("version_string", Version.VERSION_STRING)
                .put("length_of_settings_data", Settings.LENGTH_OF_SETTINGS_DATA)
                .raw("old_versions", Json.array(oldVersions))
                .raw("misc_tweaks", MiscTweakVectors.generate())
                .toString();
    }

    private static void write(Path path, String json) throws Exception {
        Files.write(path, (json + "\n").getBytes(StandardCharsets.UTF_8));
    }
}
