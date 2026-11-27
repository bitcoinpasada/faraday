//! What volvelle's worksheet takes from `wasm_bindgen::prelude`.

/// The prelude: the attribute and the error type.
pub mod prelude {
    pub use wasm_bindgen_macro::wasm_bindgen;

    /// The error a DOM call returns; never made here.
    #[derive(Debug)]
    pub struct JsError;
}
