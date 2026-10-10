import com.dabomstew.pkrandom.Settings;
import com.dabomstew.pkrandom.pokemon.ExpCurve;
import com.dabomstew.pkrandom.pokemon.GenRestrictions;

import java.lang.reflect.Field;
import java.lang.reflect.Modifier;
import java.util.ArrayList;
import java.util.List;
import java.util.Random;

final class SettingsReflection {
    private static final List<Field> FIELDS = encodableFields();

    private SettingsReflection() {
    }

    private static List<Field> encodableFields() {
        List<Field> fields = new ArrayList<>();
        for (Field field : Settings.class.getDeclaredFields()) {
            if (Modifier.isStatic(field.getModifiers()) || field.getName().equals("customNames")) {
                continue;
            }
            field.setAccessible(true);
            fields.add(field);
        }
        return fields;
    }

    static String dump(Settings settings) throws IllegalAccessException {
        Json.Obj out = Json.obj();
        for (Field field : FIELDS) {
            out.raw(field.getName(), valueJson(field.get(settings)));
        }
        return out.toString();
    }

    private static String valueJson(Object value) {
        if (value == null) {
            return "null";
        }
        if (value instanceof Boolean || value instanceof Integer) {
            return value.toString();
        }
        if (value instanceof Enum) {
            return Json.str(((Enum<?>) value).name());
        }
        if (value instanceof int[]) {
            return Json.ints((int[]) value);
        }
        if (value instanceof GenRestrictions) {
            return Integer.toString(((GenRestrictions) value).toInt());
        }
        if (value instanceof String) {
            return Json.str((String) value);
        }
        throw new IllegalStateException("Unsupported field type " + value.getClass().getName());
    }

    static Settings defaults(String romName) throws IllegalAccessException {
        Settings settings = new Settings();
        settings.setRomName(romName);
        settings.setSelectedEXPCurve(ExpCurve.MEDIUM_FAST);
        return settings;
    }

    static Settings random(Random random, String romName, boolean unconstrainedIntegers)
            throws IllegalAccessException {
        Settings settings = new Settings();
        for (Field field : FIELDS) {
            Class<?> type = field.getType();
            if (type == boolean.class) {
                field.setBoolean(settings, random.nextBoolean());
            } else if (type == int.class) {
                field.setInt(settings, unconstrainedIntegers ? random.nextInt() : plausibleInt(field.getName(), random));
            } else if (type.isEnum()) {
                Object[] constants = type.getEnumConstants();
                field.set(settings, constants[random.nextInt(constants.length)]);
            } else if (type == int[].class) {
                field.set(settings, starters(random, unconstrainedIntegers));
            } else if (type == GenRestrictions.class) {
                field.set(settings, random.nextInt(4) == 0 ? null : new GenRestrictions(random.nextInt(256)));
            } else if (type == String.class) {
                field.set(settings, romName);
            } else {
                throw new IllegalStateException("Unsupported field " + field.getName());
            }
        }
        return settings;
    }

    private static int[] starters(Random random, boolean unconstrainedIntegers) {
        int[] starters = new int[3];
        for (int i = 0; i < starters.length; i++) {
            starters[i] = unconstrainedIntegers ? random.nextInt() : random.nextInt(1100);
        }
        return starters;
    }

    private static int plausibleInt(String name, Random random) {
        switch (name) {
            case "currentMiscTweaks":
                return random.nextInt(1 << 23);
            case "guaranteedMoveCount":
                return 2 + random.nextInt(3);
            case "movesetsGoodDamagingPercent":
            case "tmsGoodDamagingPercent":
            case "tutorsGoodDamagingPercent":
                return random.nextInt(101);
            case "trainersForceFullyEvolvedLevel":
                return 30 + random.nextInt(36);
            case "trainersLevelModifier":
            case "wildLevelModifier":
            case "totemLevelModifier":
            case "staticLevelModifier":
                return random.nextInt(101) - 50;
            case "eliteFourUniquePokemonNumber":
                return random.nextInt(3);
            case "additionalBossTrainerPokemon":
            case "additionalImportantTrainerPokemon":
            case "additionalRegularTrainerPokemon":
                return random.nextInt(6);
            case "minimumCatchRateLevel":
                return 1 + random.nextInt(5);
            case "updateBaseStatsToGeneration":
            case "updateMovesToGeneration":
                return random.nextInt(9);
            default:
                throw new IllegalStateException("No plausible range for " + name);
        }
    }
}
