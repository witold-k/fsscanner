package fsscanner;

import io.github.stefanrichterhuber.wasmtimejavang.*;
import io.github.stefanrichterhuber.wasmtimejavang.wasip2wasicli.WasiCliContext;
import io.github.stefanrichterhuber.wasmtimejavang.wasip2wasifilesystem.WasiFilesystemContext;

import java.io.InputStream;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;

public class CollectToMdWasmtime {
    public static void main(String[] args) {
        // 1. Target path resolution
        String userDir = args.length > 0 ? args[0] : ".";
        Path targetHostPath = Path.of(userDir).toAbsolutePath().normalize();

        if (targetHostPath.equals(Path.of("/").toAbsolutePath().normalize())) {
            System.err.println("Error: Scanning the entire system root '/' is blocked within the WASM sandbox for security and stability reasons.");
            System.exit(1);
        }

        // 2. Read the WASM module from resources as bytes
        byte[] wasmBytes;
        try (InputStream wasmStream = CollectToMdWasmtime.class.getResourceAsStream("/collect_to_md.wasm")) {
            if (wasmStream == null) {
                throw new RuntimeException("Could not find collect_to_md.wasm inside the JAR!");
            }
            wasmBytes = wasmStream.readAllBytes();
        } catch (Exception e) {
            throw new RuntimeException("Error loading the WASM module", e);
        }

        // Rewrite arguments for the Rust application
        // WICHTIG: Wir übergeben den realen, absoluten Pfad als String an Rust
        List<String> wasmArgs = new ArrayList<>();
        wasmArgs.add("collect_to_md");
        wasmArgs.add(targetHostPath.toString());
        for (int i = 1; i < args.length; i++) {
            wasmArgs.add(args[i]);
        }

        // 3. Initialize the Engine and Store
        try (WasmtimeEngine engine = new WasmtimeEngine();
             WasmtimeStore store = new WasmtimeStore(engine)) {

            // 4. Initialize the Linker for WASI Preview 2 Components
            WasmtimeComponentLinker linker = new WasmtimeComponentLinker(engine, store);

            // 5. Configure CLI Context arguments via the explicit instance
            WasiCliContext cliContext = new WasiCliContext();
            cliContext.withArguments(wasmArgs);
            linker.linkContext(cliContext);

            // 6. Configure Directory Preopens on the Filesystem Context
            WasiFilesystemContext fsContext = new WasiFilesystemContext();
            // WICHTIG: 1:1 Mapping. Der reale Host-Pfad wird im Gast auf exakt denselben Pfad-String gemappt!
            fsContext.withDirectory(targetHostPath, targetHostPath.toString());
            linker.linkContext(fsContext);

            // 7. Compile and Link the WebAssembly Component from bytes
            try (WasmtimeComponent component = new WasmtimeComponent(engine, wasmBytes)) {

                // Link remaining required default interfaces (clocks, random, io) via ServiceLoader
                linker.linkRequired(component);

                // 8. Instantiate the component using the ComponentInstance constructor
                try (WasmtimeComponentInstance instance = new WasmtimeComponentInstance(store, component, linker)) {

                    // The SDK automatically discovers the 'wasi:cli/run@...' interface
                    java.util.concurrent.Callable<io.github.stefanrichterhuber.wasmtimejavang.component.WitResult> runnable = instance.asCliRunnable();

                    // 9. Execute the component
                    io.github.stefanrichterhuber.wasmtimejavang.component.WitResult result = runnable.call();

                    if (result != null) {
                        System.out.println("WASM execution finished. Result: " + result);
                    }
                }
            }
        } catch (Exception e) {
            e.printStackTrace();
            System.exit(1);
        }
    }
}

