import com.dabomstew.pkrandom.Settings;

import java.io.FileInputStream;
import java.io.FileOutputStream;
import java.nio.file.Files;
import java.nio.file.Path;

final class Outcomes {
    private Outcomes() {
    }

    static String errorName(Throwable error) {
        return error.getClass().getSimpleName();
    }

    static String decode(String encoded) throws Exception {
        Settings decoded;
        try {
            decoded = Settings.fromString(encoded);
        } catch (Throwable error) {
            return Json.obj().put("error", errorName(error)).toString();
        }
        return describeDecoded(decoded);
    }

    static String describeDecoded(Settings decoded) throws Exception {
        Json.Obj out = Json.obj().raw("fields", SettingsReflection.dump(decoded));
        try {
            out.put("reencoded", decoded.toString());
        } catch (Throwable error) {
            out.put("reencode_error", errorName(error));
        }
        return out.toString();
    }

    static byte[] writeRnqs(Settings settings) throws Exception {
        Path file = Files.createTempFile("vector", ".rnqs");
        try (FileOutputStream out = new FileOutputStream(file.toFile())) {
            settings.write(out);
        }
        byte[] bytes = Files.readAllBytes(file);
        Files.delete(file);
        return bytes;
    }

    static String readRnqs(byte[] bytes) throws Exception {
        Path file = Files.createTempFile("vector", ".rnqs");
        Files.write(file, bytes);
        try (FileInputStream in = new FileInputStream(file.toFile())) {
            Settings settings = Settings.read(in);
            return describeDecoded(settings);
        } catch (Throwable error) {
            return Json.obj().put("error", errorName(error)).put("message", error.getMessage()).toString();
        } finally {
            Files.delete(file);
        }
    }
}
