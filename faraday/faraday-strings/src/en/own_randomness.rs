//! Faraday's English words for the own randomness area.

macro_rules! words_own_randomness {
    ([$next:ident $($rest:ident)*] $($acc:tt)*) => {
        $next! {
            [$($rest)*]
            $($acc)*
    // ----- Your own randomness (docs/OWN-RANDOMNESS.md §2), faraday-core -----
    /// The status line's first part. `{}`: the source the shell named, or `EN.create_device_os`.
    own_system: "System: {}",
    /// The status line. `{}`: `own_system`, then what was added or `own_none`.
    own_status: "{} · your own: {}",
    /// Nothing added yet, in the status line.
    own_none: "none",
    /// One count in the status line. `{}`: the number, then its noun.
    own_count: "{} {}",
    /// Status line noun, one.
    own_camera_frame: "camera frame",
    /// Status line noun, more than one.
    own_camera_frames: "camera frames",
    /// Status line noun, one.
    own_dice_roll: "dice roll",
    /// Status line noun, more than one.
    own_dice_rolls: "dice rolls",
    /// Status line noun, one.
    own_coin_flip: "coin flip",
    /// Status line noun, more than one.
    own_coin_flips: "coin flips",
    /// Status line noun, one.
    own_card: "card",
    /// Status line noun, more than one.
    own_cards: "cards",
    /// Status line noun, one.
    own_pointer_key: "pointer move or key",
    /// Status line noun, more than one.
    own_pointer_keys: "pointer moves and keys",
    /// Settings' row's title.
    own_randomness: "Randomness",
    /// The page's title, and the button on Settings and New key that opens it.
    own_add_randomness: "Add randomness",
    /// Add randomness's label over its sources.
    own_add_to_pool: "Add randomness to the pool",
    /// Add randomness once one batch was added. `{}`: the batch, `own_count`.
    own_added_once: "Added to the pool once: {}",
    /// Add randomness once more than one was added. `{}`: how many times, then the last batch, `own_count`.
    own_added_times: "Added to the pool {} times · last: {}",
    /// Source choice.
    own_source_camera: "Camera",
    /// Source choice.
    own_source_dice: "Dice",
    /// Source choice.
    own_source_coins: "Coins",
    /// Source choice.
    own_source_cards: "Cards",
    /// The batch not yet added. `{}`: how many, then `own_dice_rolls` or its kin.
    own_batch: "In this batch: {} {}",
    /// The dice batch not yet added. `{}`: how many, then `own_dice_rolls` or its kin, then the bits they hold if fair (one decimal).
    own_batch_bits: "In this batch: {} {} · {} bits",
    /// A dice batch of 99 rolls or more.
    own_enough_256: "Enough for 256 bits on its own",
    /// The button that adds the batch.
    own_add: "Add",
    /// Keyboard hint under the dice.
    own_dice_keys: "Type 1 to 6 · Backspace takes one back · Enter adds",
    /// Keyboard hint under the coins.
    own_coin_keys: "Type H or T · Backspace takes one back · Enter adds",
    /// Keyboard hint under the camera.
    own_camera_keys: "Space takes a picture",
    /// Source choice, on the desktop.
    own_source_pointer_keys: "Pointer and keys",
    /// Inside Pointer and keys' area.
    own_pointer_keys_area: "Move the pointer here and type",
    /// Pointer and keys' count, one.
    own_pointer_move: "pointer move",
    /// Pointer and keys' count, more than one.
    own_pointer_moves: "pointer moves",
    /// Pointer and keys' count, one.
    own_key: "key",
    /// Pointer and keys' count, more than one.
    own_keys: "keys",
    /// What Pointer and keys has taken. `{}`: the pointer moves, then the keys, each `own_count`.
    own_pointer_keys_taken: "{}, {}",
        }
    };
}
