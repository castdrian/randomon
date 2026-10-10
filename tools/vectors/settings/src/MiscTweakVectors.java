import com.dabomstew.pkrandom.MiscTweak;

import java.lang.reflect.Field;
import java.lang.reflect.Modifier;
import java.util.ArrayList;
import java.util.Collections;
import java.util.List;

final class MiscTweakVectors {
    private MiscTweakVectors() {
    }

    static String generate() throws Exception {
        List<String> tweaks = new ArrayList<>();
        for (MiscTweak tweak : MiscTweak.allTweaks) {
            tweaks.add(Json.obj()
                    .put("constant", constantName(tweak))
                    .put("value", tweak.getValue())
                    .put("name", tweak.getTweakName())
                    .put("priority", priority(tweak))
                    .toString());
        }
        List<MiscTweak> sorted = new ArrayList<>(MiscTweak.allTweaks);
        Collections.sort(sorted);
        List<String> sortedNames = new ArrayList<>();
        for (MiscTweak tweak : sorted) {
            sortedNames.add(constantName(tweak));
        }
        return Json.obj()
                .raw("tweaks", Json.array(tweaks))
                .raw("priority_order", Json.strings(sortedNames))
                .toString();
    }

    private static String constantName(MiscTweak tweak) throws Exception {
        for (Field field : MiscTweak.class.getDeclaredFields()) {
            if (Modifier.isStatic(field.getModifiers()) && field.getType() == MiscTweak.class
                    && field.get(null) == tweak) {
                return field.getName();
            }
        }
        throw new IllegalStateException("Unnamed tweak " + tweak.getValue());
    }

    private static int priority(MiscTweak tweak) throws Exception {
        Field field = MiscTweak.class.getDeclaredField("priority");
        field.setAccessible(true);
        return field.getInt(tweak);
    }
}
