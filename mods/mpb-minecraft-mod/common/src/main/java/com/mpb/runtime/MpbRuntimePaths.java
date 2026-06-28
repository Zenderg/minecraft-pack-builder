package com.mpb.runtime;

import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;

public record MpbRuntimePaths(
        Path instanceRoot,
        Path mpbDirectory,
        Path schemesDirectory,
        Path cacheDirectory,
        Path knowledgeDirectory,
        Path configFile,
        Path runtimePidFile) {
    public static MpbRuntimePaths discover() {
        Path workingDirectory = Paths.get("").toAbsolutePath().normalize();
        Path instanceRoot = inferInstanceRoot(workingDirectory);
        return fromInstanceRoot(instanceRoot);
    }

    public static MpbRuntimePaths fromInstanceRoot(Path instanceRoot) {
        Path mpbDirectory = instanceRoot.resolve("mpb");
        return new MpbRuntimePaths(
                instanceRoot,
                mpbDirectory,
                mpbDirectory.resolve("schemes"),
                mpbDirectory.resolve("cache"),
                mpbDirectory.resolve("knowledge"),
                mpbDirectory.resolve("config.json"),
                mpbDirectory.resolve("runtime.pid"));
    }

    public void writeRuntimePid() {
        try {
            Files.createDirectories(mpbDirectory);
            Files.writeString(runtimePidFile, Long.toString(ProcessHandle.current().pid()));
        } catch (Exception error) {
            throw new IllegalStateException("Could not write MPB runtime pid: " + error.getMessage(), error);
        }
    }

    public void deleteRuntimePid() {
        try {
            Files.deleteIfExists(runtimePidFile);
        } catch (Exception ignored) {
            // Stale pid files are treated as best-effort running hints by the patcher.
        }
    }

    public void prepare() {
        try {
            Files.createDirectories(mpbDirectory);
            Files.createDirectories(schemesDirectory);
            Files.createDirectories(cacheDirectory);
            if (!Files.exists(configFile)) {
                MpbRuntimeConfig.load(configFile);
            }
        } catch (Exception error) {
            throw new IllegalStateException("Could not prepare MPB runtime folders: " + error.getMessage(), error);
        }
    }

    private static Path inferInstanceRoot(Path workingDirectory) {
        Path fileName = workingDirectory.getFileName();
        if (fileName != null) {
            String name = fileName.toString();
            if (".minecraft".equals(name) || "minecraft".equals(name)) {
                Path parent = workingDirectory.getParent();
                if (parent != null) {
                    return parent;
                }
            }
        }
        if (Files.isDirectory(workingDirectory.resolve(".minecraft"))) {
            return workingDirectory;
        }
        return workingDirectory;
    }
}
