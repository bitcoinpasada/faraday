//! A number written as decimal digits on the stack.

use core::ops::Deref;

use zeroize::Zeroize;

/// The longest decimal form of a `u64` or an `i64`, in bytes (a sign and
/// 19 digits, or 20 digits).
const WIDTH: usize = 20;

/// A number's decimal digits, held on the stack and wiped when dropped.
pub struct Num {
    buf: [u8; WIDTH],
    start: usize,
}

impl Num {
    /// The digits of `n`.
    pub fn from_u64(n: u64) -> Num {
        Num::digits(n, false)
    }

    /// The digits of `n`, with a minus sign when it is below zero.
    pub fn from_i64(n: i64) -> Num {
        Num::digits(n.unsigned_abs(), n < 0)
    }

    fn digits(mut n: u64, negative: bool) -> Num {
        let mut buf = [0u8; WIDTH];
        let mut start = WIDTH;
        loop {
            start -= 1;
            buf[start] = b'0' + (n % 10) as u8;
            n /= 10;
            if n == 0 {
                break;
            }
        }
        if negative {
            start -= 1;
            buf[start] = b'-';
        }
        Num { buf, start }
    }

    /// The digits as text.
    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.buf[self.start..]).unwrap_or("")
    }
}

impl Deref for Num {
    type Target = str;

    fn deref(&self) -> &str {
        self.as_str()
    }
}

impl Drop for Num {
    fn drop(&mut self) {
        self.buf.zeroize();
    }
}

impl core::fmt::Debug for Num {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Num({} bytes)", self.as_str().len())
    }
}
