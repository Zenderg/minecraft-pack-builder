package com.mpb.runtime;

import java.io.ByteArrayInputStream;
import java.io.ByteArrayOutputStream;
import java.io.DataInputStream;
import java.io.DataOutputStream;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.DirectoryStream;
import java.nio.file.Files;
import java.nio.file.Path;
import java.time.Instant;
import java.util.ArrayList;
import java.util.Comparator;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.UUID;
import java.util.zip.GZIPInputStream;
import java.util.zip.GZIPOutputStream;

public final class MpbManagerFileService {
    private static final int DATA_VERSION_1_20_1 = 3465;
    private static final String MPB_JSON_TAG = "MPBJson";

    private final MpbRuntimePaths paths;
    private final MpbSchemeRepository repository;

    public MpbManagerFileService(MpbRuntimePaths paths) {
        this.paths = paths;
        this.repository = new MpbSchemeRepository(paths.schemesDirectory());
        try {
            Files.createDirectories(importDirectory());
            Files.createDirectories(exportDirectory());
        } catch (IOException error) {
            throw new IllegalStateException("Could not prepare MPB import/export directories: " + error.getMessage(), error);
        }
    }

    public List<Path> importFiles() {
        try {
            Files.createDirectories(importDirectory());
            List<Path> files = new ArrayList<>();
            try (DirectoryStream<Path> stream = Files.newDirectoryStream(importDirectory())) {
                for (Path file : stream) {
                    if (Files.isRegularFile(file) && Format.fromFileName(file.getFileName().toString()) != null) {
                        files.add(file);
                    }
                }
            }
            files.sort(Comparator.comparing(path -> path.getFileName().toString(), String.CASE_INSENSITIVE_ORDER));
            return files;
        } catch (IOException error) {
            throw new IllegalStateException("Could not list MPB import files: " + error.getMessage(), error);
        }
    }

    public Path exportScheme(String schemeId, Format format) {
        try {
            Files.createDirectories(exportDirectory());
            String schemeJson = repository.read(schemeId);
            String safeName = safeFileName(MpbJson.flatFields(schemeJson).getOrDefault("name", schemeId));
            Path output = exportDirectory().resolve(safeName + "." + format.extension());
            Files.write(output, format == Format.SCHEM ? writeSchem(schemeJson) : writeLitematic(schemeJson));
            return output;
        } catch (IOException error) {
            throw new IllegalStateException("Could not export MPB scheme: " + error.getMessage(), error);
        }
    }

    public String importFile(String fileName) {
        Path file = importDirectory().resolve(fileName).normalize();
        if (!file.startsWith(importDirectory())) {
            throw new IllegalArgumentException("Import file must stay inside mpb/import.");
        }
        Format format = Format.fromFileName(file.getFileName().toString());
        if (format == null) {
            throw new IllegalArgumentException("Import file must end with .schem or .litematic.");
        }
        try {
            String importedJson = readEmbeddedMpbJson(Files.readAllBytes(file));
            String schemeId = UUID.randomUUID().toString();
            String now = Instant.now().toString();
            String name = stripKnownExtension(file.getFileName().toString());
            String normalized = importedJson
                    .replaceFirst("\"schemeId\"\\s*:\\s*\"(?:\\\\.|[^\"])*\"", "\"schemeId\": " + MpbJson.quote(schemeId))
                    .replaceFirst("\"name\"\\s*:\\s*\"(?:\\\\.|[^\"])*\"", "\"name\": " + MpbJson.quote(name))
                    .replaceFirst("\"createdAt\"\\s*:\\s*\"(?:\\\\.|[^\"])*\"", "\"createdAt\": " + MpbJson.quote(now))
                    .replaceFirst("\"updatedAt\"\\s*:\\s*\"(?:\\\\.|[^\"])*\"", "\"updatedAt\": " + MpbJson.quote(now));
            repository.update(schemeId, normalized);
            return schemeId;
        } catch (IOException error) {
            throw new IllegalStateException("Could not import MPB scheme: " + error.getMessage(), error);
        }
    }

    public Path importDirectory() {
        return paths.instanceRoot().resolve("mpb/import").normalize();
    }

    public Path exportDirectory() {
        return paths.instanceRoot().resolve("mpb/export").normalize();
    }

    private byte[] writeSchem(String schemeJson) throws IOException {
        MpbGuideScheme scheme = MpbGuideScheme.load(paths, MpbJson.flatFields(schemeJson).get("schemeId"));
        Bounds bounds = bounds(scheme);
        Palette palette = palette(scheme);
        byte[] blockData = new byte[Math.max(1, bounds.width() * bounds.height() * bounds.depth())];
        for (MpbGuideScheme.Block block : scheme.blocks()) {
            int index = (block.y() * bounds.depth() * bounds.width()) + (block.z() * bounds.width()) + block.x();
            blockData[index] = (byte) palette.indexes().getOrDefault(block.blockId(), 0).intValue();
        }

        ByteArrayOutputStream raw = new ByteArrayOutputStream();
        try (DataOutputStream out = new DataOutputStream(raw)) {
            rootStart(out);
            intTag(out, "Version", 3);
            intTag(out, "DataVersion", DATA_VERSION_1_20_1);
            shortTag(out, "Width", bounds.width());
            shortTag(out, "Height", bounds.height());
            shortTag(out, "Length", bounds.depth());
            intArrayTag(out, "Offset", new int[] {0, 0, 0});
            compoundStart(out, "Palette");
            for (Map.Entry<String, Integer> entry : palette.indexes().entrySet()) {
                intTag(out, entry.getKey(), entry.getValue());
            }
            end(out);
            intTag(out, "PaletteMax", palette.indexes().size());
            byteArrayTag(out, "BlockData", blockData);
            emptyCompoundList(out, "BlockEntities");
            emptyCompoundList(out, "Entities");
            compoundStart(out, "Metadata");
            stringTag(out, "Name", scheme.name());
            stringTag(out, "Author", "Minecraft Pack Builder");
            end(out);
            stringTag(out, MPB_JSON_TAG, schemeJson);
            end(out);
        }
        return gzip(raw.toByteArray());
    }

    private byte[] writeLitematic(String schemeJson) throws IOException {
        MpbGuideScheme scheme = MpbGuideScheme.load(paths, MpbJson.flatFields(schemeJson).get("schemeId"));
        Bounds bounds = bounds(scheme);
        ByteArrayOutputStream raw = new ByteArrayOutputStream();
        try (DataOutputStream out = new DataOutputStream(raw)) {
            rootStart(out);
            intTag(out, "Version", 6);
            intTag(out, "SubVersion", 1);
            intTag(out, "MinecraftDataVersion", DATA_VERSION_1_20_1);
            compoundStart(out, "Metadata");
            stringTag(out, "Name", scheme.name());
            stringTag(out, "Author", "Minecraft Pack Builder");
            stringTag(out, "Description", "Exported by Minecraft Pack Builder");
            intTag(out, "RegionCount", 1);
            intTag(out, "TotalBlocks", scheme.blocks().size());
            intTag(out, "TotalVolume", Math.max(1, bounds.width() * bounds.height() * bounds.depth()));
            compoundStart(out, "EnclosingSize");
            intTag(out, "x", bounds.width());
            intTag(out, "y", bounds.height());
            intTag(out, "z", bounds.depth());
            end(out);
            longTag(out, "TimeCreated", 0L);
            longTag(out, "TimeModified", 0L);
            end(out);
            compoundStart(out, "Regions");
            compoundStart(out, scheme.name().isBlank() ? "MPB Scheme" : scheme.name());
            compoundStart(out, "Position");
            intTag(out, "x", 0);
            intTag(out, "y", 0);
            intTag(out, "z", 0);
            end(out);
            compoundStart(out, "Size");
            intTag(out, "x", bounds.width());
            intTag(out, "y", bounds.height());
            intTag(out, "z", bounds.depth());
            end(out);
            emptyCompoundList(out, "BlockStatePalette");
            longArrayTag(out, "BlockStates", new long[0]);
            emptyCompoundList(out, "TileEntities");
            emptyCompoundList(out, "Entities");
            end(out);
            end(out);
            stringTag(out, MPB_JSON_TAG, schemeJson);
            end(out);
        }
        return gzip(raw.toByteArray());
    }

    private String readEmbeddedMpbJson(byte[] gzipBytes) throws IOException {
        byte[] raw;
        try (GZIPInputStream gzip = new GZIPInputStream(new ByteArrayInputStream(gzipBytes))) {
            raw = gzip.readAllBytes();
        }
        try (DataInputStream in = new DataInputStream(new ByteArrayInputStream(raw))) {
            int rootType = in.readUnsignedByte();
            if (rootType != 10) {
                throw new IOException("NBT root is not a compound.");
            }
            readString(in);
            String embedded = findStringTag(in, MPB_JSON_TAG);
            if (embedded == null || embedded.isBlank()) {
                throw new IOException("File does not contain MPB scheme payload.");
            }
            return embedded;
        }
    }

    private String findStringTag(DataInputStream in, String targetName) throws IOException {
        while (true) {
            int type = in.readUnsignedByte();
            if (type == 0) {
                return null;
            }
            String name = readString(in);
            if (type == 8) {
                String value = readString(in);
                if (targetName.equals(name)) {
                    return value;
                }
            } else if (type == 10) {
                String nested = findStringTag(in, targetName);
                if (nested != null) {
                    return nested;
                }
            } else {
                skipPayload(in, type);
            }
        }
    }

    private void skipPayload(DataInputStream in, int type) throws IOException {
        switch (type) {
            case 1 -> in.skipNBytes(1);
            case 2 -> in.skipNBytes(2);
            case 3 -> in.skipNBytes(4);
            case 4 -> in.skipNBytes(8);
            case 7 -> in.skipNBytes(in.readInt());
            case 9 -> {
                int elementType = in.readUnsignedByte();
                int length = in.readInt();
                for (int index = 0; index < length; index++) {
                    skipPayload(in, elementType);
                }
            }
            case 11 -> in.skipNBytes((long) in.readInt() * 4L);
            case 12 -> in.skipNBytes((long) in.readInt() * 8L);
            default -> throw new IOException("Unsupported NBT tag type " + type + ".");
        }
    }

    private byte[] gzip(byte[] raw) throws IOException {
        ByteArrayOutputStream bytes = new ByteArrayOutputStream();
        try (GZIPOutputStream gzip = new GZIPOutputStream(bytes)) {
            gzip.write(raw);
        }
        return bytes.toByteArray();
    }

    private Bounds bounds(MpbGuideScheme scheme) {
        int maxX = 0;
        int maxY = 0;
        int maxZ = 0;
        for (MpbGuideScheme.Block block : scheme.blocks()) {
            maxX = Math.max(maxX, block.x());
            maxY = Math.max(maxY, block.y());
            maxZ = Math.max(maxZ, block.z());
        }
        return new Bounds(maxX + 1, maxY + 1, maxZ + 1);
    }

    private Palette palette(MpbGuideScheme scheme) {
        Map<String, Integer> indexes = new LinkedHashMap<>();
        indexes.put("minecraft:air", 0);
        for (MpbGuideScheme.Block block : scheme.blocks()) {
            indexes.computeIfAbsent(block.blockId(), ignored -> indexes.size());
        }
        return new Palette(indexes);
    }

    private String safeFileName(String value) {
        String cleaned = value == null ? "scheme" : value.trim().replaceAll("[^A-Za-z0-9_.-]+", "-");
        if (cleaned.isBlank()) {
            return "scheme";
        }
        return cleaned.length() > 80 ? cleaned.substring(0, 80) : cleaned;
    }

    private String stripKnownExtension(String fileName) {
        String lower = fileName.toLowerCase();
        if (lower.endsWith(".litematic")) {
            return fileName.substring(0, fileName.length() - ".litematic".length());
        }
        if (lower.endsWith(".schem")) {
            return fileName.substring(0, fileName.length() - ".schem".length());
        }
        return fileName;
    }

    private void rootStart(DataOutputStream out) throws IOException {
        out.writeByte(10);
        writeString(out, "");
    }

    private void compoundStart(DataOutputStream out, String name) throws IOException {
        out.writeByte(10);
        writeString(out, name);
    }

    private void intTag(DataOutputStream out, String name, int value) throws IOException {
        out.writeByte(3);
        writeString(out, name);
        out.writeInt(value);
    }

    private void longTag(DataOutputStream out, String name, long value) throws IOException {
        out.writeByte(4);
        writeString(out, name);
        out.writeLong(value);
    }

    private void shortTag(DataOutputStream out, String name, int value) throws IOException {
        out.writeByte(2);
        writeString(out, name);
        out.writeShort(value);
    }

    private void stringTag(DataOutputStream out, String name, String value) throws IOException {
        out.writeByte(8);
        writeString(out, name);
        writeString(out, value == null ? "" : value);
    }

    private void byteArrayTag(DataOutputStream out, String name, byte[] values) throws IOException {
        out.writeByte(7);
        writeString(out, name);
        out.writeInt(values.length);
        out.write(values);
    }

    private void intArrayTag(DataOutputStream out, String name, int[] values) throws IOException {
        out.writeByte(11);
        writeString(out, name);
        out.writeInt(values.length);
        for (int value : values) {
            out.writeInt(value);
        }
    }

    private void longArrayTag(DataOutputStream out, String name, long[] values) throws IOException {
        out.writeByte(12);
        writeString(out, name);
        out.writeInt(values.length);
        for (long value : values) {
            out.writeLong(value);
        }
    }

    private void emptyCompoundList(DataOutputStream out, String name) throws IOException {
        out.writeByte(9);
        writeString(out, name);
        out.writeByte(10);
        out.writeInt(0);
    }

    private void end(DataOutputStream out) throws IOException {
        out.writeByte(0);
    }

    private void writeString(DataOutputStream out, String value) throws IOException {
        byte[] bytes = value.getBytes(StandardCharsets.UTF_8);
        out.writeShort(bytes.length);
        out.write(bytes);
    }

    private String readString(DataInputStream in) throws IOException {
        int length = in.readUnsignedShort();
        byte[] bytes = in.readNBytes(length);
        return new String(bytes, StandardCharsets.UTF_8);
    }

    public enum Format {
        SCHEM("schem"),
        LITEMATIC("litematic");

        private final String extension;

        Format(String extension) {
            this.extension = extension;
        }

        public String extension() {
            return extension;
        }

        public static Format fromFileName(String fileName) {
            String lower = fileName.toLowerCase();
            if (lower.endsWith(".schem")) {
                return SCHEM;
            }
            if (lower.endsWith(".litematic")) {
                return LITEMATIC;
            }
            return null;
        }
    }

    private record Bounds(int width, int height, int depth) {}

    private record Palette(Map<String, Integer> indexes) {}
}
