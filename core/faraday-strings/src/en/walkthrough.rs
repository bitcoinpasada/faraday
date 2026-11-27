//! Faraday's English words for the walkthrough area.

macro_rules! words_walkthrough {
    ([$next:ident $($rest:ident)*] $($acc:tt)*) => {
        $next! {
            [$($rest)*]
            $($acc)*

        }
    };
}
