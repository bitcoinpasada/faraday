//! Faraday's English words for the transfer area.

macro_rules! words_transfer {
    ([$next:ident $($rest:ident)*] $($acc:tt)*) => {
        $next! {
            [$($rest)*]
            $($acc)*
    // ----- transfer (faraday-core/src/transfer/mod.rs) -----
    /// Text in `transfer_send`. `{}`: the most Transfer sends, as
    /// `size_text` writes it, then `self.medium.a()`.
    transfer_larger_than: "Larger than {}: carry it on {}",
    /// Text in `transfer_send`. `{}`: `base_name(&path)`.
    transfer_is_empty: "{} is empty",
    /// Note in `transfer_scanned`. `{}`: `name`.
    transfer_saving: "Saving {}",
    /// Text in `transfer_saved`. `{}`: `path`.
    transfer_saved_secret: "Saved {}: this file holds a secret",
    /// Text in `transfer_saved`. `{}`: `path`.
    transfer_saved: "Saved {}",
    /// Error in `transfer_saved`. `{}`: `reason`.
    transfer_not_saved: "Not saved: {}",
    // ----- transfer (faraday-core/src/transfer/screen.rs) -----
    /// Title in `draw`.
    transfer_transfer: "Transfer",
    /// Section label in `send`.
    transfer_send: "Send",
    /// Text in `send`. `{}`: `name`.
    transfer_reading: "Reading {}",
    /// Line in `send`.
    transfer_no_files: "No files",
    /// A Downloads row's size when the file is over what Transfer sends.
    /// `{}`: the most it sends, as `size_text` writes it.
    transfer_over: "Over {}",
    /// Section label of the text card in `send_text`.
    transfer_send_text: "Send text",
    /// Label of the text field in `send_text`.
    transfer_text_label: "Text",
    /// The empty text field's hint in `send_text`.
    transfer_text_hint: "Paste or type the text",
    /// Supporting text under the field in `send_text`. `{}`: the
    /// characters in the text, then its size as `size_text` writes it.
    transfer_text_count: "{} characters · {}",
    /// Button in `send_text`.
    transfer_text_paste: "Paste",
    /// Button in `send_text`.
    transfer_text_clear: "Clear",
    /// Button in `send_text`.
    transfer_text_show: "Show as QR",
    /// Title of the QR sheet of a text sent.
    transfer_text_sheet_title: "Text",
    /// Refusal under the field when the whole text is a secret. `{}`: the
    /// kind's name, as `Kind::label` writes it, in lower case.
    transfer_text_secret: "Not shown: this is {}",
    /// Refusal under the field when a secret is found in the text. `{}`:
    /// the kind's name, as `Kind::label` writes it, in lower case.
    transfer_text_maybe_secret: "Not shown: it holds {}",
    /// Refusal under the field when the text holds part of a code not known
    /// to be public, or a whole one that holds a private key or a share.
    transfer_text_partial: "Not shown: it holds a code that may hold a secret",
    /// Refusal under the field when the text holds bare hex of a seed's
    /// length.
    transfer_text_seed_hex: "Not shown: it holds hex of a seed's length",
    /// Refusal under the field when the text is too long. `{}`: the most
    /// text sent, as `size_text` writes it.
    transfer_text_too_long: "Over {}: send it as a file",
    /// Section label in `receive`.
    transfer_receive: "Receive",
    /// Button in `receive`.
    transfer_scan_camera: "Scan with the camera",
    /// Button in `receive`.
    transfer_open_folder: "Open folder",
        }
    };
}
