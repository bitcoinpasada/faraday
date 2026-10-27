//! The satisfaction of a `wsh` or `sh(wsh)` miniscript, chosen by the
//! size of the signatures it carries.
//!
//! `miniscript` 12's non-malleable satisfier chooses between two ways
//! of satisfying a fragment by an estimate in which every ECDSA
//! signature is 73 bytes, the most it can be. Two satisfactions that
//! differ only in whose signature they carry then weigh the same, and
//! it keeps the second it built, though a DER signature with its
//! sighash byte is 71 to 73 bytes and the PSBT holds the real ones: for
//! `or_b(pk(A),s:pk(B))` with both signed, a 72-byte signature from A
//! and a 71-byte one from B, it writes A's.
//!
//! This makes the same choices that satisfier makes, fragment by
//! fragment (`miniscript`'s `Satisfaction::satisfy_helper` and
//! `dissatisfy_helper`), with the signatures and every other element
//! measured as they are written: each its bytes and a length byte.
//! Where two satisfactions weigh the same, the first branch the
//! fragment names is taken, as python-bip380's satisfier does
//! (`docs/CROSSCHECK.md` §10): `or_b`, `or_d`, `or_c` and `or_i` their
//! left branch, `andor` its `and`, and an `or_i` dissatisfied its left.
//! A `thresh` orders its branches and a `multi` its signatures as
//! `miniscript` does: a `multi` takes the first signatures in key
//! order. What can be satisfied at all is `miniscript`'s answer, not
//! this one: it runs only on an input `miniscript` has finalized.

use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;
use core::mem;

use bitcoin::PublicKey;
use bitcoin::hashes::hash160;
use miniscript::psbt::PsbtInputSatisfier;
use miniscript::{Miniscript, Satisfier, Segwitv0, Terminal, Threshold};

/// `miniscript`'s standardness limit on a P2WSH witness's items.
const MAX_STANDARD_P2WSH_STACK_ITEMS: usize = 100;

/// The estimate `miniscript` gives every ECDSA signature, its length
/// byte included.
const SIG_ESTIMATE: usize = 73;

#[derive(Clone)]
struct Item {
    bytes: Vec<u8>,
    sig: bool,
}

impl Item {
    fn data(bytes: Vec<u8>) -> Self {
        Item { bytes, sig: false }
    }

    /// The element as a witness writes it: its bytes and a length byte.
    fn size(&self) -> usize {
        self.bytes.len() + 1
    }

    /// `miniscript`'s estimate of it, which only a `thresh` reads here.
    fn estimate(&self) -> usize {
        if self.sig { SIG_ESTIMATE } else { self.size() }
    }
}

/// `miniscript`'s `Witness`: a stack, or none that we can make
/// (`Unavailable`: a third party might), or none anyone can make
/// without a key (`Impossible`).
#[derive(Clone)]
enum Stack {
    Items(Vec<Item>),
    Unavailable,
    Impossible,
}

impl Stack {
    fn of(items: Vec<Item>) -> Self {
        Stack::Items(items)
    }

    fn push_0() -> Self {
        Stack::of(vec![Item::data(Vec::new())])
    }

    fn push_1() -> Self {
        Stack::of(vec![Item::data(vec![1])])
    }

    /// `one`'s items, then `two`'s.
    fn combine(one: Stack, two: Stack) -> Stack {
        match (one, two) {
            (Stack::Impossible, _) | (_, Stack::Impossible) => Stack::Impossible,
            (Stack::Unavailable, _) | (_, Stack::Unavailable) => Stack::Unavailable,
            (Stack::Items(mut a), Stack::Items(b)) => {
                a.extend(b);
                Stack::Items(a)
            }
        }
    }

    /// Whether `self` is strictly smaller than `other`, a stack being
    /// smaller than no stack, and `Impossible` smaller than
    /// `Unavailable` (`miniscript`'s order, by real size).
    fn smaller(&self, other: &Stack) -> bool {
        match (self, other) {
            (Stack::Items(a), Stack::Items(b)) => size(a) < size(b),
            (Stack::Items(_), _) => true,
            (_, Stack::Items(_)) => false,
            (Stack::Impossible, Stack::Unavailable) => true,
            _ => false,
        }
    }
}

fn size(items: &[Item]) -> usize {
    items.iter().map(Item::size).sum()
}

fn estimate(items: &[Item]) -> usize {
    let count = items.len();
    let varint = match count {
        0..=0xfc => 1,
        0xfd..=0xffff => 3,
        _ => 5,
    };
    items.iter().map(Item::estimate).sum::<usize>() + varint
}

#[derive(Clone)]
struct Sat {
    stack: Stack,
    has_sig: bool,
}

impl Sat {
    fn new(stack: Stack, has_sig: bool) -> Self {
        Sat { stack, has_sig }
    }

    fn empty() -> Self {
        Sat::new(Stack::of(Vec::new()), false)
    }

    /// `miniscript`'s `concatenate_rev`: `other`'s items, then `self`'s,
    /// so that `left.then(right)` reads as the script does.
    fn then(self, other: Sat) -> Sat {
        Sat::new(
            Stack::combine(other.stack, self.stack),
            self.has_sig || other.has_sig,
        )
    }

    /// `miniscript`'s `Satisfaction::minimum`, by real size, a tie going
    /// to `first`.
    fn either(first: Sat, second: Sat) -> Sat {
        match (&first.stack, &second.stack) {
            (Stack::Impossible, _) => return second,
            (_, Stack::Impossible) => return first,
            _ => {}
        }
        match (first.has_sig, second.has_sig) {
            // Neither needs a signature: a third party could swap one
            // for the other.
            (false, false) => Sat::new(Stack::Unavailable, false),
            // Only one needs no signature: that one, or a third party
            // could strip the signature and use it.
            (false, true) => first,
            (true, false) => second,
            (true, true) => {
                if second.stack.smaller(&first.stack) {
                    second
                } else {
                    first
                }
            }
        }
    }

    /// `miniscript`'s `Satisfaction::minimum_mall`, by real size, a tie
    /// going to `first`: the choice between an `or_i`'s two
    /// dissatisfactions.
    fn either_mall(first: Sat, second: Sat) -> Sat {
        match (&first.stack, &second.stack) {
            (Stack::Impossible | Stack::Unavailable, _) => return second,
            (_, Stack::Impossible | Stack::Unavailable) => return first,
            _ => {}
        }
        let has_sig = first.has_sig && second.has_sig;
        let stack = if second.stack.smaller(&first.stack) {
            second.stack
        } else {
            first.stack
        };
        Sat::new(stack, has_sig)
    }
}

type Node = Terminal<PublicKey, Segwitv0>;
type Branches = Threshold<Arc<Miniscript<PublicKey, Segwitv0>>, 0>;

struct Satisfy<'a> {
    s: PsbtInputSatisfier<'a>,
    root_has_sig: bool,
}

impl Satisfy<'_> {
    /// The PSBT input's satisfier, read for SegWit v0 keys.
    fn s(&self) -> &dyn Satisfier<PublicKey> {
        &self.s
    }

    fn sig(&self, pk: &PublicKey) -> Stack {
        match self.s().lookup_ecdsa_sig(pk) {
            Some(sig) => Stack::of(vec![Item {
                bytes: sig.to_vec(),
                sig: true,
            }]),
            None => Stack::Impossible,
        }
    }

    fn raw_pkh_key(&self, h: &hash160::Hash) -> Option<PublicKey> {
        self.s().lookup_raw_pkh_pk(h).or_else(|| {
            Satisfier::<PublicKey>::lookup_raw_pkh_ecdsa_sig(&self.s, h).map(|(pk, _)| pk)
        })
    }

    fn preimage(found: Option<[u8; 32]>) -> Stack {
        match found {
            Some(p) => Stack::of(vec![Item::data(p.to_vec())]),
            // A preimage may be known to someone else.
            None => Stack::Unavailable,
        }
    }

    fn timelock(&self, met: bool) -> Sat {
        let stack = if met {
            Stack::of(Vec::new())
        } else if self.root_has_sig {
            Stack::Impossible
        } else {
            Stack::Unavailable
        };
        Sat::new(stack, false)
    }

    fn sat(&self, node: &Node) -> Sat {
        match node {
            Terminal::PkK(pk) => Sat::new(self.sig(pk), true),
            Terminal::PkH(pk) => Sat::new(
                Stack::combine(self.sig(pk), Stack::of(vec![Item::data(pk.to_bytes())])),
                true,
            ),
            Terminal::RawPkH(h) => {
                let stack = match (
                    Satisfier::<PublicKey>::lookup_raw_pkh_ecdsa_sig(&self.s, h),
                    self.raw_pkh_key(h),
                ) {
                    (Some((_, sig)), Some(pk)) => Stack::of(vec![
                        Item {
                            bytes: sig.to_vec(),
                            sig: true,
                        },
                        Item::data(pk.to_bytes()),
                    ]),
                    _ => Stack::Impossible,
                };
                Sat::new(stack, true)
            }
            Terminal::After(t) => self.timelock(self.s().check_after((*t).into())),
            Terminal::Older(t) => self.timelock(self.s().check_older((*t).into())),
            Terminal::Ripemd160(h) => Sat::new(Self::preimage(self.s().lookup_ripemd160(h)), false),
            Terminal::Hash160(h) => Sat::new(Self::preimage(self.s().lookup_hash160(h)), false),
            Terminal::Sha256(h) => Sat::new(Self::preimage(self.s().lookup_sha256(h)), false),
            Terminal::Hash256(h) => Sat::new(Self::preimage(self.s().lookup_hash256(h)), false),
            Terminal::True => Sat::empty(),
            Terminal::False => Sat::new(Stack::Impossible, false),
            Terminal::Alt(sub)
            | Terminal::Swap(sub)
            | Terminal::Check(sub)
            | Terminal::Verify(sub)
            | Terminal::NonZero(sub)
            | Terminal::ZeroNotEqual(sub) => self.sat(&sub.node),
            Terminal::DupIf(sub) => {
                let sat = self.sat(&sub.node);
                Sat::new(Stack::combine(sat.stack, Stack::push_1()), sat.has_sig)
            }
            Terminal::AndV(l, r) | Terminal::AndB(l, r) => {
                self.sat(&l.node).then(self.sat(&r.node))
            }
            Terminal::AndOr(a, b, c) => Sat::either(
                self.sat(&a.node).then(self.sat(&b.node)),
                self.dissat(&a.node).then(self.sat(&c.node)),
            ),
            Terminal::OrB(l, r) => Sat::either(
                self.sat(&l.node).then(self.dissat(&r.node)),
                self.dissat(&l.node).then(self.sat(&r.node)),
            ),
            Terminal::OrD(l, r) | Terminal::OrC(l, r) => Sat::either(
                self.sat(&l.node),
                self.dissat(&l.node).then(self.sat(&r.node)),
            ),
            Terminal::OrI(l, r) => {
                let l = self.sat(&l.node);
                let r = self.sat(&r.node);
                Sat::either(
                    Sat::new(Stack::combine(l.stack, Stack::push_1()), l.has_sig),
                    Sat::new(Stack::combine(r.stack, Stack::push_0()), r.has_sig),
                )
            }
            Terminal::Thresh(t) if t.k() == t.n() => t
                .iter()
                .map(|s| self.sat(&s.node))
                .fold(Sat::empty(), Sat::then),
            Terminal::Thresh(t) => self.thresh(t),
            Terminal::Multi(t) => {
                let sigs: Vec<Item> = t
                    .iter()
                    .filter_map(|pk| self.s().lookup_ecdsa_sig(pk))
                    .take(t.k())
                    .map(|sig| Item {
                        bytes: sig.to_vec(),
                        sig: true,
                    })
                    .collect();
                if sigs.len() < t.k() {
                    Sat::new(Stack::Impossible, false)
                } else {
                    Sat::new(Stack::combine(Stack::push_0(), Stack::of(sigs)), true)
                }
            }
            // Not a SegWit v0 fragment: the parse refuses it.
            Terminal::MultiA(_) => Sat::new(Stack::Impossible, false),
        }
    }

    fn dissat(&self, node: &Node) -> Sat {
        match node {
            Terminal::PkK(_) => Sat::new(Stack::push_0(), false),
            Terminal::PkH(pk) => Sat::new(
                Stack::combine(Stack::push_0(), Stack::of(vec![Item::data(pk.to_bytes())])),
                false,
            ),
            Terminal::RawPkH(h) => {
                let key = match Satisfier::<PublicKey>::lookup_raw_pkh_pk(&self.s, h) {
                    Some(pk) => Stack::of(vec![Item::data(pk.to_bytes())]),
                    None => Stack::Unavailable,
                };
                Sat::new(Stack::combine(Stack::push_0(), key), false)
            }
            Terminal::False => Sat::empty(),
            Terminal::True
            | Terminal::Older(_)
            | Terminal::After(_)
            | Terminal::Verify(_)
            | Terminal::OrC(..) => Sat::new(Stack::Impossible, false),
            Terminal::Sha256(_)
            | Terminal::Hash256(_)
            | Terminal::Ripemd160(_)
            | Terminal::Hash160(_) => Sat::new(Stack::of(vec![Item::data(vec![0; 32])]), false),
            Terminal::Alt(sub)
            | Terminal::Swap(sub)
            | Terminal::Check(sub)
            | Terminal::ZeroNotEqual(sub) => self.dissat(&sub.node),
            Terminal::DupIf(_) | Terminal::NonZero(_) => Sat::new(Stack::push_0(), false),
            Terminal::AndV(v, other) => self.sat(&v.node).then(self.dissat(&other.node)),
            Terminal::AndB(l, r)
            | Terminal::OrB(l, r)
            | Terminal::OrD(l, r)
            | Terminal::AndOr(l, _, r) => self.dissat(&l.node).then(self.dissat(&r.node)),
            Terminal::OrI(l, r) => {
                let l = self.dissat(&l.node);
                let r = self.dissat(&r.node);
                Sat::either_mall(
                    Sat::new(Stack::combine(l.stack, Stack::push_1()), l.has_sig),
                    Sat::new(Stack::combine(r.stack, Stack::push_0()), r.has_sig),
                )
            }
            Terminal::Thresh(t) => t
                .iter()
                .map(|s| self.dissat(&s.node))
                .fold(Sat::empty(), Sat::then),
            Terminal::Multi(t) => {
                Sat::new(Stack::of(vec![Item::data(Vec::new()); t.k() + 1]), false)
            }
            Terminal::MultiA(t) => Sat::new(Stack::of(vec![Item::data(Vec::new()); t.n()]), false),
        }
    }

    /// `miniscript`'s `Satisfaction::thresh`: the branches ordered by
    /// whether they can be satisfied at all, then those needing no
    /// signature first, then by the estimate of what satisfying one adds
    /// over dissatisfying it, the first k satisfied.
    fn thresh(&self, t: &Branches) -> Sat {
        let mut sats: Vec<Sat> = t.iter().map(|s| self.sat(&s.node)).collect();
        let mut chosen: Vec<Sat> = t.iter().map(|s| self.dissat(&s.node)).collect();
        let mut order: Vec<usize> = (0..t.n()).collect();
        order.sort_by_key(|&i| {
            let weight = match (&sats[i].stack, &chosen[i].stack) {
                (Stack::Unavailable | Stack::Impossible, _) => i64::MAX,
                (_, Stack::Unavailable | Stack::Impossible) => i64::MIN,
                (Stack::Items(s), Stack::Items(d)) => estimate(s) as i64 - estimate(d) as i64,
            };
            let impossible = matches!(sats[i].stack, Stack::Impossible);
            (impossible, sats[i].has_sig, weight)
        });
        for &i in &order[..t.k()] {
            mem::swap(&mut chosen[i], &mut sats[i]);
        }
        if matches!(sats[order[t.k() - 1]].stack, Stack::Impossible) {
            Sat::new(Stack::Impossible, false)
        } else if !sats[order[t.k()]].has_sig
            && !matches!(sats[order[t.k()]].stack, Stack::Impossible)
        {
            Sat::new(Stack::Unavailable, false)
        } else {
            chosen.into_iter().fold(Sat::empty(), Sat::then)
        }
    }
}

/// Costs no curve work, only a walk of the script: the witness stack
/// (without the witness script) that satisfies `ms` for input `index`,
/// by the rule above; `None` when there is none.
pub(super) fn stack(
    psbt: &bitcoin::Psbt,
    index: usize,
    ms: &Miniscript<PublicKey, Segwitv0>,
) -> Option<Vec<Vec<u8>>> {
    let satisfy = Satisfy {
        s: PsbtInputSatisfier::new(psbt, index),
        root_has_sig: ms.ty.mall.safe,
    };
    match satisfy.sat(&ms.node).stack {
        Stack::Items(items) if items.len() <= MAX_STANDARD_P2WSH_STACK_ITEMS => {
            Some(items.into_iter().map(|i| i.bytes).collect())
        }
        _ => None,
    }
}
