//! `#[wasm_bindgen]` with nothing generated: the worksheet runs natively.

extern crate proc_macro;
use proc_macro::TokenStream;

/// Returns the item it is put on, unchanged.
#[proc_macro_attribute]
pub fn wasm_bindgen(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}
