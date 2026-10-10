import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.Map;

public final class VectorGen {
    private VectorGen() {
    }

    public static void main(String[] args) throws IOException {
        if (args.length != 1) {
            throw new IllegalArgumentException("usage: VectorGen <output-directory>");
        }
        Path output = Paths.get(args[0]);
        Files.createDirectories(output);
        write(output, "random.json", RandomVectors.build());
        write(output, "shuffle.json", CollectionVectors.buildShuffle());
        write(output, "hash_map.json", CollectionVectors.buildHashMap());
        write(output, "hash_set.json", CollectionVectors.buildHashSet());
        write(output, "number_to_string.json", NumberVectors.buildDoubleToString());
        write(output, "round.json", NumberVectors.buildRound());
        write(output, "strict_math.json", NumberVectors.buildStrictMath());
        write(output, "hash_code.json", NumberVectors.buildHashCodes());
        write(output, "format.json", FormatVectors.build());
    }

    private static void write(Path directory, String name, Map<String, Object> content) throws IOException {
        Path file = directory.resolve(name);
        Files.write(file, Json.write(content).getBytes(StandardCharsets.UTF_8));
        System.out.println("wrote " + file + " (" + Files.size(file) + " bytes)");
    }
}
