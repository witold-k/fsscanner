package fsscanner;

import com.dylibso.chicory.log.SystemLogger;
import com.dylibso.chicory.wasi.WasiOptions;
import com.dylibso.chicory.wasi.WasiPreview1;
import com.dylibso.chicory.runtime.Store;
import com.dylibso.chicory.runtime.Instance;
import com.dylibso.chicory.wasm.Parser;
import com.dylibso.chicory.wasm.WasmModule;
import java.io.InputStream;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;

public class CollectToMdChicory {
    public static void main(String[] args) {
        // 1. Pfad ermitteln und normalisieren
        String userDir = args.length > 0 ? args[0] : ".";
        Path targetHostPath = Path.of(userDir).toAbsolutePath().normalize();

        if (targetHostPath.equals(Path.of("/").toAbsolutePath().normalize())) {
            System.err.println("Fehler: Das Scannen des gesamten System-Root '/' ist aus Sicherheits- und Stabilitätsgründen in der WASM-Sandbox gesperrt.");
            System.exit(1);
        }

        // 2. WASM-Modul laden
        InputStream wasmStream = CollectToMdChicory.class.getResourceAsStream("/collect_to_md.wasm");
        if (wasmStream == null) {
            throw new RuntimeException("Konnte collect_to_md.wasm nicht im JAR finden!");
        }
        WasmModule module = Parser.parse(wasmStream);
        SystemLogger logger = new SystemLogger();

        // Argumente für Rust umschreiben
        List<String> wasmArgs = new ArrayList<>();
        wasmArgs.add("collect_to_md");
        wasmArgs.add("/target_dir");
        for (int i = 1; i < args.length; i++) {
            wasmArgs.add(args[i]);
        }

        // 3. Chicory WASI konfigurieren
        WasiOptions options = WasiOptions.builder()
                .withDirectory("/target_dir", targetHostPath)
                .withArguments(wasmArgs)
                .withStdout(System.out)
                .withStderr(System.err)
                .build();

        WasiPreview1 wasi = WasiPreview1.builder()
                .withLogger(logger)
                .withOptions(options)
                .build();

        // 4. Ausführen & Exceptions abfangen
        Store store = new Store().addFunction(wasi.toHostFunctions());

        try {
            Instance instance = store.instantiate("collect-to-md", module);

            if (instance.export("_start") != null) {
                instance.export("_start").apply();
            }
        } catch (com.dylibso.chicory.wasi.WasiExitException e) {
            System.exit(e.exitCode());
        } catch (com.dylibso.chicory.runtime.TrapException e) {
            if (!e.getMessage().contains("unreachable") && !e.getMessage().contains("exit")) {
                throw e;
            }
        }
    }
}

