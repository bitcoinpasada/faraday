//! Faraday's English words for Check an address.

macro_rules! words_address_check {
    ([$next:ident $($rest:ident)*] $($acc:tt)*) => {
        $next! {
            [$($rest)*]
            $($acc)*

    // ----- address check (faraday-core/src/address_check/) -----
    /// Line on the empty part of Check an address.
    address_check_no_address: "No address to check",
    /// Title over the text files waiting in Files to choose from.
    address_check_choose_file: "Choose a file with addresses",
    /// Cancel of the files menu, `files_menu`.
    address_check_cancel: "Cancel",
        }
    };
}
