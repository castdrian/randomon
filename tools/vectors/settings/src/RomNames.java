import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;

final class RomNames {
    private RomNames() {
    }

    static List<String> load(Path configDirectory) throws Exception {
        List<String> names = new ArrayList<>();
        for (int generation = 1; generation <= 7; generation++) {
            Path ini = configDirectory.resolve("gen" + generation + "_offsets.ini");
            String prefix = generation == 3 ? "" : "Pokemon ";
            for (String line : Files.readAllLines(ini, StandardCharsets.UTF_8)) {
                String trimmed = line.replace("\uFEFF", "").trim();
                if (trimmed.startsWith("[") && trimmed.endsWith("]")) {
                    names.add(prefix + trimmed.substring(1, trimmed.length() - 1));
                }
            }
        }
        names.add("Pokemon VietCrystal");
        return names;
    }

    static List<String> unusual() {
        List<String> names = new ArrayList<>();
        names.add("");
        names.add("Pokémon Émeraude");
        names.add("\u0000\u007f");
        names.add("😀 emoji");
        names.add(repeat('A', 255));
        names.add(repeat('B', 256));
        names.add(repeat('C', 300));
        return names;
    }

    private static String repeat(char c, int count) {
        StringBuilder out = new StringBuilder();
        for (int i = 0; i < count; i++) {
            out.append(c);
        }
        return out.toString();
    }
}
