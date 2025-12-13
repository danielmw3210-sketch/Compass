use wasmtime::*;

pub struct SVM {
    engine: Engine,
}

impl SVM {
    pub fn new() -> Self {
        let config = Config::new();
        // config.consume_fuel(true); // Enable gas metering later
        let engine = Engine::new(&config).expect("Failed to create Wasmtime engine");
        SVM { engine }
    }

    /// Run a simple WASM binary that exports a "run" function
    pub fn execute(&self, wasm_bytes: &[u8]) -> Result<(), String> {
        let mut store = Store::new(&self.engine, ());
        let module = Module::from_binary(&self.engine, wasm_bytes)
            .map_err(|e| format!("Invalid WASM binary: {}", e))?;

        // Linker for host functions (Empty for now)
        let linker = Linker::new(&self.engine);

        let instance = linker.instantiate(&mut store, &module)
            .map_err(|e| format!("Failed to instantiate module: {}", e))?;

        // Look for the "run" export
        let run = instance.get_typed_func::<(), ()>(&mut store, "run")
            .map_err(|_| "Contract must export 'run' function".to_string())?;

        run.call(&mut store, ())
            .map_err(|e| format!("Runtime Error: {}", e))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execute_empty_module() {
        // A minimal WASM binary (header + strict empty module)
        // This won't work with "run" expectation, but proves engine init.
        // wat: (module (func (export "run"))) 
        let wat = r#"(module (func (export "run")))"#;
        let wasm = wat::parse_str(wat).unwrap();

        let vm = SVM::new();
        let res = vm.execute(&wasm);
        assert!(res.is_ok());
    }
}
