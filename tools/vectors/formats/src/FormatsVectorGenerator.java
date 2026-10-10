import java.io.OutputStream;
import java.io.PrintStream;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Map;

public final class FormatsVectorGenerator {
    private FormatsVectorGenerator() {
    }

    private interface VectorSource {
        Map<String, Object> generate() throws Exception;
    }

    public static void main(String[] args) throws Exception {
        Path outputDirectory = Path.of(args[0]);
        Path workDirectory = Path.of(args[1]);
        Path testPatchRoot = Path.of(args[2]);
        Path upstreamPatches = Path.of(args[3]);
        Files.createDirectories(outputDirectory);
        Files.createDirectories(workDirectory);
        PrintStream log = System.out;
        PrintStream silent = new PrintStream(OutputStream.nullOutputStream());
        System.setOut(silent);
        System.setErr(silent);
        Guard.install();
        write(log, outputDirectory, "jdk", MiscVectors::jdk);
        write(log, outputDirectory, "blz", CompressionVectors::blz);
        write(log, outputDirectory, "lzss", CompressionVectors::lzss);
        write(log, outputDirectory, "ds", CompressionVectors::ds);
        write(log, outputDirectory, "search", MiscVectors::search);
        write(log, outputDirectory, "ips", () -> MiscVectors.ips(upstreamPatches, testPatchRoot));
        write(log, outputDirectory, "crc16", MiscVectors::crc16);
        write(log, outputDirectory, "timsort", MiscVectors::timsort);
        write(log, outputDirectory, "mini", MiscVectors::mini);
        write(log, outputDirectory, "smdh", MiscVectors::smdh);
        write(log, outputDirectory, "amx", MiscVectors::amx);
        write(log, outputDirectory, "bflim", MiscVectors::bflim);
        write(log, outputDirectory, "narc", MiscVectors::narc);
        write(log, outputDirectory, "garc", GarcVectors::vectors);
        write(log, outputDirectory, "nds_rom", () -> NdsVectors.vectors(workDirectory));
        write(log, outputDirectory, "ncch", () -> NcchVectors.vectors(workDirectory));
        log.println("done");
        System.exit(0);
    }

    private static void write(PrintStream log, Path directory, String name, VectorSource source) throws Exception {
        long start = System.currentTimeMillis();
        String json = Json.write(source.generate());
        Files.writeString(directory.resolve(name + ".json"), json, StandardCharsets.UTF_8);
        log.println(name + ": " + json.length() + " bytes in " + (System.currentTimeMillis() - start) + " ms");
    }
}
