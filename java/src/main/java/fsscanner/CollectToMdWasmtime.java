package fsscanner;

import io.github.kawamuray.wasmtime.Engine;
import io.github.kawamuray.wasmtime.Linker;
import io.github.kawamuray.wasmtime.Module;
import io.github.kawamuray.wasmtime.Store;
import io.github.kawamuray.wasmtime.Func;
import io.github.kawamuray.wasmtime.WasmtimeException;
import io.github.kawamuray.wasmtime.wasi.WasiCtx;
import io.github.kawamuray.wasmtime.wasi.WasiCtxBuilder;
import java.io.InputStream;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;

public class CollectToMdWasmtime {
    public static void main(String[] args) {
        // 1. Pfad ermitteln und normalisieren
        String userDir = args.length > 0 ? args[0] : ".";
        Path targetHostPath = Path.of(userDir).toAbsolutePath().normalize();

        if (targetHostPath.equals(Path.of("/").toAbsolutePath().normalize())) {
            System.err.println("Fehler: Das Scannen des gesamten System-Root '/' ist aus Sicherheits- und Stabilitätsgründen in der WASM-Sandbox gesperrt.");
            System.exit(1);
        }

        // 2. WASM-Modul aus den Ressourcen als Bytes einlesen
        byte[] wasmBytes;
        try (InputStream wasmStream = CollectToMdWasmtime.class.getResourceAsStream("/collect_to_md.wasm")) {
            if (wasmStream == null) {
                throw new RuntimeException("Konnte collect_to_md.wasm nicht im JAR finden!");
            }
            wasmBytes = wasmStream.readAllBytes();
        } catch (Exception e) {
            throw new RuntimeException("Fehler beim Laden des WASM-Moduls", e);
        }

        // Argumente für Rust umschreiben
        List<String> wasmArgs = new ArrayList<>();
        wasmArgs.add("collect_to_md");
        wasmArgs.add("/target_dir");
        for (int i = 1; i < args.length; i++) {
            wasmArgs.add(args[i]);
        }

        // 3. Wasmtime Engine und Linker vorbereiten
        try (Engine engine = new Engine();
             Linker linker = new Linker(engine)) {

            // WASI Host-Funktionen im Linker registrieren
            WasiCtx.addToLinker(linker);

            // WASI Kontext bauen (Verzeichnis-Mapping und Argumente)
            // .args() verlangt ein Iterable (z.B. List<String>)
            WasiCtx wasiCtx = new WasiCtxBuilder()
                    .inheritStdout()
                    .inheritStderr()
                    .preopenedDir(targetHostPath.toString(), "/target_dir")
                    .args(wasmArgs)
                    .build();

            // In wasmtime-java wird der WasiCtx als Kontext-Daten-Objekt in den Store gegeben
            try (Store<WasiCtx> store = Store.withoutData(engine)) {
                store.data(wasiCtx); // Optional: Setzt die internen Daten, falls gebraucht

                // Modul wird über den Konstruktor geladen (fromBytes gibt es hier nicht)
                try (Module module = new Module(engine, wasmBytes)) {

                    // Instanziieren und verlinken
                    linker.module(store, "collect-to-md", module);

                    // Einstiegspunkt holen und ausführen
                    Func startFunc = linker.get(store, "collect-to-md", "_start")
                            .orElseThrow(() -> new RuntimeException("Einstiegspunkt _start nicht im Modul gefunden!"))
                            .func();

                    startFunc.call(store);
                }
            }
        } catch (WasmtimeException e) {
            // Wasmtime reicht Exit-Codes im Fehlertext oder Statustyp durch
            int exitCode = extractExitCode(e);
            System.exit(exitCode);
        } catch (Exception e) {
            e.printStackTrace();
            System.exit(1);
        }
    }

    /**
     * Hilfsmethode, um den exakten Exit-Code aus der WasmtimeException auszulesen.
     */
    private static int extractExitCode(WasmtimeException e) {
        String msg = e.getMessage();
        if (msg != null && msg.contains("wasi exit code")) {
            try {
                String[] parts = msg.split("wasi exit code");
                if (parts.length > 1) {
                    return Integer.parseInt(parts[1].trim());
                }
            } catch (Exception ignored) {}
        }
        return 1;
    }
}

