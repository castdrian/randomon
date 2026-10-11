import com.dabomstew.pkrandom.FileFunctions;
import com.dabomstew.pkrandom.RandomSource;
import com.dabomstew.pkrandom.Randomizer;
import com.dabomstew.pkrandom.Settings;
import com.dabomstew.pkrandom.SettingsUpdater;
import com.dabomstew.pkrandom.romhandlers.Gen1RomHandler;
import com.dabomstew.pkrandom.romhandlers.Gen2RomHandler;
import com.dabomstew.pkrandom.romhandlers.Gen3RomHandler;
import com.dabomstew.pkrandom.romhandlers.Gen4RomHandler;
import com.dabomstew.pkrandom.romhandlers.Gen5RomHandler;
import com.dabomstew.pkrandom.romhandlers.Gen6RomHandler;
import com.dabomstew.pkrandom.romhandlers.Gen7RomHandler;
import com.dabomstew.pkrandom.romhandlers.RomHandler;

import java.io.ByteArrayOutputStream;
import java.io.File;
import java.io.PrintStream;
import java.io.PrintWriter;
import java.io.StringWriter;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.List;
import java.util.ResourceBundle;

public class ParityRunner {
    private static final RomHandler.Factory[] FACTORIES = new RomHandler.Factory[] {
            new Gen1RomHandler.Factory(),
            new Gen2RomHandler.Factory(),
            new Gen3RomHandler.Factory(),
            new Gen4RomHandler.Factory(),
            new Gen5RomHandler.Factory(),
            new Gen6RomHandler.Factory(),
            new Gen7RomHandler.Factory()
    };

    public static void main(String[] args) throws Exception {
        if (args.length != 3 && args.length != 4) {
            System.err.println("usage: ParityRunner <rom> <jobs file> <output directory> [game update]");
            System.exit(2);
        }
        String romPath = new File(args[0]).getAbsolutePath();
        String updatePath = args.length == 4 ? new File(args[3]).getAbsolutePath() : null;
        List<String> jobs = Files.readAllLines(Paths.get(args[1]), StandardCharsets.UTF_8);
        Path outputDirectory = Paths.get(args[2]);
        Files.createDirectories(outputDirectory);
        ResourceBundle bundle = ResourceBundle.getBundle("com/dabomstew/pkrandom/newgui/Bundle");
        for (String job : jobs) {
            if (job.trim().isEmpty()) {
                continue;
            }
            String[] fields = job.split("\t");
            boolean saveAsDirectory = fields.length > 4 && fields[4].equals("dir");
            runJob(romPath, updatePath, fields[0], Long.parseLong(fields[1]), fields[2], fields[3],
                    saveAsDirectory, outputDirectory, bundle);
        }
    }

    private static Settings parseSettings(String mode, String text) throws Exception {
        if (mode.equals("versioned")) {
            int version = Integer.parseInt(text.substring(0, 3));
            String body = text.substring(3);
            if (version < Settings.VERSION) {
                body = new SettingsUpdater().update(version, body);
            }
            return Settings.fromString(body);
        }
        return Settings.fromString(text);
    }

    private static RomHandler loadHandler(String romPath, String updatePath) {
        for (RomHandler.Factory factory : FACTORIES) {
            if (factory.isLoadable(romPath)) {
                RomHandler handler = factory.create(RandomSource.instance());
                handler.loadRom(romPath);
                if (updatePath != null) {
                    handler.loadGameUpdate(updatePath);
                }
                return handler;
            }
        }
        throw new IllegalArgumentException("unsupported ROM " + romPath);
    }

    private static void runJob(String romPath, String updatePath, String id, long seed, String mode,
                               String settingsText, boolean saveAsDirectory, Path outputDirectory,
                               ResourceBundle bundle) throws Exception {
        Path romOutput = outputDirectory.resolve(id + ".rom");
        Path logOutput = outputDirectory.resolve(id + ".log");
        Path checkValueOutput = outputDirectory.resolve(id + ".cv");
        Path errorOutput = outputDirectory.resolve(id + ".err");
        try {
            Settings settings = parseSettings(mode, settingsText);
            settings.setCustomNames(FileFunctions.getCustomNames());
            RomHandler handler = loadHandler(romPath, updatePath);
            settings.tweakForRom(handler);
            ByteArrayOutputStream buffer = new ByteArrayOutputStream();
            PrintStream log = new PrintStream(buffer, false, "UTF-8");
            handler.setLog(log);
            int checkValue = new Randomizer(settings, handler, bundle, saveAsDirectory)
                    .randomize(romOutput.toString(), log, seed);
            log.flush();
            Files.write(logOutput, buffer.toByteArray());
            Files.write(checkValueOutput, Integer.toString(checkValue).getBytes(StandardCharsets.UTF_8));
        } catch (Throwable error) {
            StringWriter trace = new StringWriter();
            error.printStackTrace(new PrintWriter(trace));
            Files.write(errorOutput, trace.toString().getBytes(StandardCharsets.UTF_8));
        }
    }
}
