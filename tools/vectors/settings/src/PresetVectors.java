import com.dabomstew.pkrandom.CustomNamesSet;
import com.dabomstew.pkrandom.Settings;
import com.dabomstew.pkrandom.Utils;
import com.dabomstew.pkrandom.Version;

import java.io.ByteArrayInputStream;
import java.io.ByteArrayOutputStream;
import java.io.DataInputStream;
import java.io.DataOutputStream;
import java.nio.ByteBuffer;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Base64;
import java.util.List;
import java.util.Random;

final class PresetVectors {
    private static final Random RANDOM = new Random(0x9E5E7L);

    private PresetVectors() {
    }

    static String generate(List<String> settingsStrings, Path configDirectory) throws Exception {
        byte[] defaultNames = Files.readAllBytes(configDirectory.resolve("customnames.rncn"));
        long[] seeds = {0L, 1L, -1L, Long.MIN_VALUE, Long.MAX_VALUE, 123456789012345L, -987654321L};
        List<byte[]> inputs = new ArrayList<>();
        for (int i = 0; i < 40; i++) {
            long seed = i < seeds.length ? seeds[i] : RANDOM.nextLong();
            String config = settingsStrings.get(RANDOM.nextInt(settingsStrings.size()));
            byte[] names = i % 3 == 0 ? CustomNamesVectors.randomNamesBytes(RANDOM) : defaultNames;
            inputs.add(preset(Version.VERSION, seed, config, names));
        }
        String[] exoticConfigs = {"", "\u0000", "A\u0000B", "é€", "😀", "\ud800", "x\udc00y",
                "\u007f\u0080߿ࠀ￿", settingsStrings.get(0) + "é"};
        for (String config : exoticConfigs) {
            inputs.add(preset(Version.VERSION, 42L, config, defaultNames));
        }
        byte[] valid = preset(Version.VERSION, 7L, settingsStrings.get(3), defaultNames);
        int[] mismatched = {Version.VERSION - 1, Version.VERSION + 1, 170, 0, -1, 100, 999};
        for (int version : mismatched) {
            byte[] copy = Arrays.copyOf(valid, valid.length);
            ByteBuffer.wrap(copy).putInt(0, version);
            inputs.add(copy);
        }
        int[] cuts = {0, 3, 4, 11, 12, 13, 14, 20, 14 + settingsStrings.get(3).length(),
                15 + settingsStrings.get(3).length(), 19 + settingsStrings.get(3).length(), valid.length - 1};
        for (int cut : cuts) {
            inputs.add(Arrays.copyOf(valid, cut));
        }
        byte[] badNamesVersion = Arrays.copyOf(valid, valid.length);
        badNamesVersion[14 + settingsStrings.get(3).length()] = 2;
        inputs.add(badNamesVersion);
        byte[] badUtf = preset(Version.VERSION, 7L, "abc", defaultNames);
        badUtf[15] = (byte) 0xC0;
        inputs.add(badUtf);
        byte[] overlong = preset(Version.VERSION, 7L, "abcd", defaultNames);
        overlong[14] = (byte) 0xC1;
        overlong[15] = (byte) 0x81;
        inputs.add(overlong);
        inputs.add(preset(Version.VERSION, 9L, corruptChecksum(settingsStrings.get(5)), defaultNames));
        inputs.add(preset(Version.VERSION, 9L, withTrailingByte(settingsStrings.get(6)), defaultNames));
        List<String> cases = new ArrayList<>();
        for (byte[] input : inputs) {
            cases.add(Json.obj().raw("bytes", Json.base64(input)).raw("read", read(input)).toString());
        }
        return Json.obj()
                .raw("cases", Json.array(cases))
                .raw("write_utf", writeUtfVectors())
                .raw("read_utf", readUtfVectors())
                .toString();
    }

    static byte[] preset(int version, long seed, String config, byte[] names) throws Exception {
        ByteArrayOutputStream bytes = new ByteArrayOutputStream();
        DataOutputStream out = new DataOutputStream(bytes);
        out.writeInt(version);
        out.writeLong(seed);
        out.writeUTF(config);
        out.write(names);
        out.close();
        return bytes.toByteArray();
    }

    private static String corruptChecksum(String settingsString) {
        byte[] data = Base64.getDecoder().decode(settingsString);
        data[data.length - 5] ^= 1;
        return Base64.getEncoder().encodeToString(data);
    }

    private static String withTrailingByte(String settingsString) {
        byte[] data = Base64.getDecoder().decode(settingsString);
        byte[] body = Arrays.copyOf(data, data.length - 8 + 1);
        return Base64.getEncoder().encodeToString(DecodeFuzzVectors.withChecksums(body, 0));
    }

    private static String read(byte[] bytes) {
        try {
            DataInputStream in = new DataInputStream(new ByteArrayInputStream(bytes));
            int version = in.readInt();
            if (version != Version.VERSION) {
                Json.Obj out = Json.obj().put("result", "version_mismatch").put("version", version);
                if (version < Version.VERSION) {
                    out.put("required_release", Version.oldVersions.getOrDefault(version, "Unknown"));
                }
                return out.toString();
            }
            long seed = in.readLong();
            String config = in.readUTF();
            int namesOffset = 4 + 8 + 2 + modifiedUtf8Length(bytes);
            CustomNamesSet names = new CustomNamesSet(in);
            return Json.obj()
                    .put("result", "read")
                    .put("seed", seed)
                    .raw("config_units", Json.units(config))
                    .raw("custom_names_bytes", Json.base64(Arrays.copyOfRange(bytes, namesOffset, bytes.length)))
                    .raw("custom_names", CustomNamesVectors.describe(names))
                    .raw("required_rom_name", requiredRomName(config, names))
                    .toString();
        } catch (Throwable error) {
            return Json.obj().put("result", "error").put("error", Outcomes.errorName(error)).toString();
        }
    }

    private static int modifiedUtf8Length(byte[] bytes) {
        return ((bytes[12] & 0xFF) << 8) | (bytes[13] & 0xFF);
    }

    private static String requiredRomName(String config, CustomNamesSet names) {
        try {
            Utils.validatePresetSupplementFiles(config, names);
            byte[] data = Base64.getDecoder().decode(config);
            int nameLength = data[Settings.LENGTH_OF_SETTINGS_DATA] & 0xFF;
            if (data.length != Settings.LENGTH_OF_SETTINGS_DATA + 9 + nameLength) {
                return Json.obj().put("result", "invalid_length").toString();
            }
            String name = new String(data, Settings.LENGTH_OF_SETTINGS_DATA + 1, nameLength, "US-ASCII");
            return Json.obj().put("result", "ok").put("name", name).toString();
        } catch (Throwable error) {
            return Json.obj().put("result", "error").put("error", Outcomes.errorName(error)).toString();
        }
    }

    private static String writeUtfVectors() throws Exception {
        List<String> strings = new ArrayList<>(Arrays.asList("", "abc", "\u0000", "A\u0000B", "\u007f", "\u0080",
                "߿", "ࠀ", "￿", "é€", "😀", "\ud800", "x\udc00y", "﻿BOM"));
        for (int i = 0; i < 40; i++) {
            strings.add(CustomNamesVectors.randomText(RANDOM, 1 + RANDOM.nextInt(30), true));
        }
        strings.add(repeat('A', 65535));
        strings.add(repeat('A', 65536));
        strings.add(repeat('é', 32767));
        strings.add(repeat('é', 32768));
        strings.add(repeat('€', 21845));
        strings.add(repeat('€', 21846));
        List<String> cases = new ArrayList<>();
        for (String value : strings) {
            Json.Obj out = Json.obj();
            if (value.length() > 1000) {
                out.put("repeat_unit", value.charAt(0)).put("repeat_count", value.length());
            } else {
                out.raw("units", Json.units(value));
            }
            try {
                ByteArrayOutputStream bytes = new ByteArrayOutputStream();
                new DataOutputStream(bytes).writeUTF(value);
                byte[] encoded = bytes.toByteArray();
                if (value.length() > 1000) {
                    out.put("encoded_length", encoded.length);
                } else {
                    out.raw("bytes", Json.base64(encoded));
                }
            } catch (Throwable error) {
                out.put("error", Outcomes.errorName(error));
            }
            cases.add(out.toString());
        }
        return Json.array(cases);
    }

    private static String readUtfVectors() {
        List<String> cases = new ArrayList<>();
        for (int i = 0; i < 300; i++) {
            byte[] payload = CustomNamesVectors.trickyBytes(RANDOM, RANDOM.nextInt(24));
            int declared = RANDOM.nextInt(10) == 0 ? payload.length + 1 + RANDOM.nextInt(3) : payload.length;
            ByteBuffer buffer = ByteBuffer.allocate(2 + payload.length);
            buffer.putShort((short) declared);
            buffer.put(payload);
            byte[] input = buffer.array();
            Json.Obj out = Json.obj().raw("bytes", Json.base64(input));
            try {
                String decoded = new DataInputStream(new ByteArrayInputStream(input)).readUTF();
                out.raw("units", Json.units(decoded));
            } catch (Throwable error) {
                out.put("error", Outcomes.errorName(error));
            }
            cases.add(out.toString());
        }
        return Json.array(cases);
    }

    private static String repeat(char c, int count) {
        char[] chars = new char[count];
        Arrays.fill(chars, c);
        return new String(chars);
    }
}
