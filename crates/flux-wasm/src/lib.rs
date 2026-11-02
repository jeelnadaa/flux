//! Flux WebAssembly bindings for browser execution.
#![deny(missing_docs)]

use wasm_bindgen::prelude::*;

/// Returns the Flux DSP engine version string.
#[wasm_bindgen]
pub fn flux_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert_eq!(flux_version(), "0.1.0");
    }
}
