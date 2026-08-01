package fsscanner;

import io.github.fasm.WasmEngine;
import io.github.fasm.WasmModule;
import io.github.fasm.WasmInstance;
import io.github.fasm.wasi.WasiContext;
import io.github.fasm.wasi.WasiException;
import java.io.InputStream;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;

public class CollectToMdFasm {
    public static void main(String[] args) {
        // 1. Pfad ermitteln und normalisieren
        String userDir = args.length > 0 ? args[0] : ".";
        Path targetHostPath = Path.of(userDir).toAbsolutePath().normalize();

        if (targetHostPath.equals(Path.of("/").toAbsolutePath().normalize())) {
            System.err.println("Fehler: Das Scannen des gesamten System-Root '/' ist aus Sicherheits- und Stabilitätsgründen in der WASM-Sandbox gesperrt.");
            System.exit(1);
        }

        // 2. WASM-Modul aus den Ressourcen als Bytes laden
        byte[] wasmBytes;
        try (InputStream wasmStream = CollectToMdFasm.class.getResourceAsStream("/collect_to_md.wasm")) {
            if (wasmStream == null) {
                throw new RuntimeException("Konnte collect_to_md.wasm nicht im JAR finden!");
            }
            wasmBytes = wasmStream.readAllBytes();
        } catch (Exception e) {
            throw new RuntimeException("Fehler beim Lesen des WASM-Moduls", e);
        }

        // Argumente für Rust umschreiben
        List<String> wasmArgs = new ArrayList<>();
        wasmArgs.add("collect_to_md");
        wasmArgs.add("/target_dir");
        for (int i = 1; i < args.length; i++) {
            wasmArgs.add(args[i]);
        }

        // 3. Fasm WASI Kontext konfigurieren
        // Fasm leitet standardmäßig stdout/stderr an das System weiter
        WasiContext wasiContext = WasiContext.builder()
                .withPreopenedDirectory(targetHostPath.toString(), "/target_dir")
                .withArguments(wasmArgs)
                .build();

        // 4. Engine initialisieren und ausführen
        try (WasmEngine engine = new WasmEngine()) {
            WasmModule module = engine.loadModule(wasmBytes);

            // Verknüpft das Modul mit den WASI-Host-Funktionen
            try (WasmInstance instance = module.instantiate(wasiContext)) {
                // Fasm führt standardmäßig '_start' beim Aufruf aus oder bietet direkten Zugriff
                instance.call("_start");
            }
        } catch (WasiException e) {
            // Fasm wirft bei einem std::process::exit in Rust eine WasiException mit dem Code
            System.exit(e.getExitCode());
        } catch (Exception e) {
            // Allgemeine Runtime- oder Linker-Fehler abfangen
            e.printStackTrace();
            System.exit(1);
        }
    }
}

