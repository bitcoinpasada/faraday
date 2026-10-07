//! The fountain code of BCR-2020-005: a message split into fragments,
//! then an endless stream of parts, each the XOR of a chosen subset.
//!
//! The first `n` parts are the `n` fragments in order. Every later part
//! mixes a subset chosen from the part's own sequence number and the
//! message checksum, so a receiver that joins the stream late, or misses
//! frames, still resolves every fragment. That is what a coordinator
//! such as Sparrow emits and what the device must finish.

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::vec::Vec;

use super::Fault;
use super::bytewords::crc32;
use super::xoshiro::Xoshiro256;

/// One part of a fountain-encoded message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Part {
    /// 1-based position in the stream.
    pub sequence: usize,
    /// How many fragments the message was split into.
    pub sequence_count: usize,
    /// Length of the message before padding.
    pub message_length: usize,
    /// CRC-32 of the message.
    pub checksum: u32,
    /// The fragment, or the XOR of several.
    pub data: Vec<u8>,
}

impl Part {
    /// The part as CBOR: an untagged five-element array of `seqNum`,
    /// `seqLen`, `messageLen`, `checksum` and `data`, each integer in
    /// its shortest form and the data a definite-length byte string.
    pub fn cbor(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.data.len() + 16);
        out.push(0x85);
        push_uint(&mut out, self.sequence as u32);
        push_uint(&mut out, self.sequence_count as u32);
        push_uint(&mut out, self.message_length as u32);
        push_uint(&mut out, self.checksum);
        out.extend_from_slice(&super::cbor_bytes(&self.data));
        out
    }

    /// The part behind `cbor`. Trailing bytes are an error.
    pub fn from_cbor(cbor: &[u8]) -> Result<Self, Fault> {
        let mut cursor = Cursor { bytes: cbor, at: 0 };
        if cursor.read()? != 0x85 {
            return Err(Fault::Cbor);
        }
        let part = Self {
            sequence: cursor.uint()? as usize,
            sequence_count: cursor.uint()? as usize,
            message_length: cursor.uint()? as usize,
            checksum: cursor.uint()?,
            data: cursor.byte_string()?,
        };
        if cursor.at != cursor.bytes.len() {
            return Err(Fault::Cbor);
        }
        Ok(part)
    }

    /// Which fragments this part is the XOR of.
    pub fn indexes(&self) -> Vec<usize> {
        choose_fragments(self.sequence, self.sequence_count, self.checksum)
    }

    /// The `<seq>-<count>` segment of the part's URI.
    pub fn sequence_id(&self) -> alloc::string::String {
        alloc::format!("{}-{}", self.sequence, self.sequence_count)
    }
}

/// A CBOR head for an unsigned integer, in the shortest form.
fn push_uint(out: &mut Vec<u8>, value: u32) {
    if value < 24 {
        out.push(value as u8);
    } else if value <= 0xff {
        out.extend_from_slice(&[0x18, value as u8]);
    } else if value <= 0xffff {
        out.push(0x19);
        out.extend_from_slice(&(value as u16).to_be_bytes());
    } else {
        out.push(0x1a);
        out.extend_from_slice(&value.to_be_bytes());
    }
}

/// Reads the five CBOR items of a part, and nothing else.
struct Cursor<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Cursor<'_> {
    fn read(&mut self) -> Result<u8, Fault> {
        let byte = *self.bytes.get(self.at).ok_or(Fault::Cbor)?;
        self.at += 1;
        Ok(byte)
    }

    fn take(&mut self, n: usize) -> Result<&[u8], Fault> {
        let end = self.at.checked_add(n).ok_or(Fault::Cbor)?;
        let slice = self.bytes.get(self.at..end).ok_or(Fault::Cbor)?;
        self.at = end;
        Ok(slice)
    }

    /// The argument of a CBOR head whose major type is `major`.
    fn argument(&mut self, major: u8) -> Result<u64, Fault> {
        let head = self.read()?;
        if head >> 5 != major {
            return Err(Fault::Cbor);
        }
        Ok(match head & 0x1f {
            n @ 0..=23 => u64::from(n),
            24 => u64::from(self.read()?),
            25 => {
                let b = self.take(2)?;
                u64::from(u16::from_be_bytes([b[0], b[1]]))
            }
            26 => {
                let b = self.take(4)?;
                u64::from(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
            }
            27 => {
                let b = self.take(8)?;
                u64::from_be_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]])
            }
            _ => return Err(Fault::Cbor),
        })
    }

    fn uint(&mut self) -> Result<u32, Fault> {
        u32::try_from(self.argument(0)?).map_err(|_| Fault::Cbor)
    }

    fn byte_string(&mut self) -> Result<Vec<u8>, Fault> {
        let len = usize::try_from(self.argument(2)?).map_err(|_| Fault::Cbor)?;
        Ok(self.take(len)?.to_vec())
    }
}

/// Emits the parts of one message, endlessly.
#[derive(Debug)]
pub struct Encoder {
    fragments: Vec<Vec<u8>>,
    fragment_count: usize,
    message_length: usize,
    checksum: u32,
    current_sequence: usize,
}

impl Encoder {
    /// An encoder over `message`, split into fragments of at most
    /// `max_fragment_length` bytes.
    pub fn new(message: &[u8], max_fragment_length: usize) -> Result<Self, Fault> {
        if message.is_empty() {
            return Err(Fault::EmptyMessage);
        }
        if max_fragment_length == 0 {
            return Err(Fault::InvalidFragmentLen);
        }
        let fragments = partition(message, fragment_length(message.len(), max_fragment_length));
        Ok(Self {
            fragment_count: fragments.len(),
            fragments,
            message_length: message.len(),
            checksum: crc32(message),
            current_sequence: 0,
        })
    }

    /// How many parts have been emitted.
    pub const fn current_sequence(&self) -> usize {
        self.current_sequence
    }

    /// How many fragments the message was split into.
    pub const fn fragment_count(&self) -> usize {
        self.fragment_count
    }

    /// The next part.
    pub fn next_part(&mut self) -> Part {
        self.current_sequence += 1;
        let indexes = choose_fragments(self.current_sequence, self.fragment_count, self.checksum);
        let mut mixed = alloc::vec![0; self.fragments[0].len()];
        for index in indexes {
            xor(&mut mixed, &self.fragments[index]);
        }
        Part {
            sequence: self.current_sequence,
            sequence_count: self.fragment_count,
            message_length: self.message_length,
            checksum: self.checksum,
            data: mixed,
        }
    }
}

/// Accumulates parts until every fragment is resolved.
#[derive(Default)]
pub struct Decoder {
    decoded: BTreeMap<usize, Vec<u8>>,
    received: BTreeSet<Vec<usize>>,
    buffer: BTreeMap<Vec<usize>, Vec<u8>>,
    queue: Vec<(usize, Vec<u8>)>,
    sequence_count: usize,
    message_length: usize,
    checksum: u32,
    fragment_length: usize,
}

impl Decoder {
    /// Takes one part in. `Ok(false)` means the part told the decoder
    /// nothing it did not already know.
    pub fn receive(&mut self, part: Part) -> Result<bool, Fault> {
        if self.complete() {
            return Ok(false);
        }
        if part.sequence_count == 0 || part.data.is_empty() || part.message_length == 0 {
            return Err(Fault::EmptyPart);
        }
        if part.sequence == 0 {
            return Err(Fault::InvalidSequence);
        }
        if self.received.is_empty() {
            self.sequence_count = part.sequence_count;
            self.message_length = part.message_length;
            self.checksum = part.checksum;
            self.fragment_length = part.data.len();
        } else if !self.validate(&part) {
            return Err(Fault::InconsistentPart);
        }
        let indexes = part.indexes();
        if self.received.contains(&indexes) {
            return Ok(false);
        }
        self.received.insert(indexes.clone());
        if indexes.len() == 1 {
            self.decoded.insert(indexes[0], part.data.clone());
            self.queue.push((indexes[0], part.data));
            self.process_queue();
        } else {
            self.process_complex(part.data, indexes);
        }
        Ok(true)
    }

    /// Reduces every buffered mix against a freshly resolved fragment,
    /// which may resolve further fragments in turn.
    fn process_queue(&mut self) {
        while let Some((index, simple)) = self.queue.pop() {
            let mixes: Vec<Vec<usize>> = self
                .buffer
                .keys()
                .filter(|indexes| indexes.contains(&index))
                .cloned()
                .collect();
            for indexes in mixes {
                let Some(mut data) = self.buffer.remove(&indexes) else {
                    continue;
                };
                let mut rest = indexes;
                rest.retain(|&x| x != index);
                xor(&mut data, &simple);
                self.settle(data, rest);
            }
        }
    }

    /// Reduces a mix against everything already resolved, then either
    /// resolves a fragment or buffers what is left.
    fn process_complex(&mut self, mut data: Vec<u8>, mut indexes: Vec<usize>) {
        let known: Vec<usize> = indexes
            .iter()
            .copied()
            .filter(|index| self.decoded.contains_key(index))
            .collect();
        if indexes.len() == known.len() {
            return;
        }
        for index in known {
            indexes.retain(|&x| x != index);
            if let Some(fragment) = self.decoded.get(&index) {
                let fragment = fragment.clone();
                xor(&mut data, &fragment);
            }
        }
        self.settle(data, indexes);
    }

    fn settle(&mut self, data: Vec<u8>, indexes: Vec<usize>) {
        if let [index] = indexes[..] {
            self.decoded.insert(index, data.clone());
            self.queue.push((index, data));
        } else {
            self.buffer.insert(indexes, data);
        }
    }

    /// Whether every fragment is resolved.
    pub fn complete(&self) -> bool {
        self.message_length != 0 && self.decoded.len() == self.sequence_count
    }

    /// How many fragments are resolved, or `None` before the first part.
    pub fn resolved_fragment_count(&self) -> Option<usize> {
        (self.message_length != 0).then_some(self.decoded.len())
    }

    /// How many fragments the message has; `0` before the first part.
    pub const fn fragment_count(&self) -> usize {
        self.sequence_count
    }

    /// Whether a part belongs to the message the decoder is assembling.
    fn validate(&self, part: &Part) -> bool {
        !self.received.is_empty()
            && part.sequence_count == self.sequence_count
            && part.message_length == self.message_length
            && part.checksum == self.checksum
            && part.data.len() == self.fragment_length
    }

    /// The message, once complete: the fragments joined, the padding
    /// checked to be zero and the checksum checked.
    pub fn message(&self) -> Result<Option<Vec<u8>>, Fault> {
        if !self.complete() {
            return Ok(None);
        }
        let mut combined = Vec::with_capacity(self.sequence_count * self.fragment_length);
        for index in 0..self.sequence_count {
            combined.extend_from_slice(self.decoded.get(&index).ok_or(Fault::Cbor)?);
        }
        let padding = combined.get(self.message_length..).ok_or(Fault::Cbor)?;
        if padding.iter().any(|&b| b != 0) {
            return Err(Fault::InvalidPadding);
        }
        combined.truncate(self.message_length);
        if crc32(&combined) != self.checksum {
            return Err(Fault::MessageChecksum);
        }
        Ok(Some(combined))
    }
}

/// `a / b`, rounded up.
const fn div_ceil(a: usize, b: usize) -> usize {
    if a.is_multiple_of(b) {
        a / b
    } else {
        a / b + 1
    }
}

/// The fragment length BCR-2020-005 picks: as many fragments as the
/// maximum needs, then the message spread evenly over them.
pub const fn fragment_length(data_length: usize, max_fragment_length: usize) -> usize {
    div_ceil(data_length, div_ceil(data_length, max_fragment_length))
}

/// `data` in fragments of `fragment_length`, the last zero-padded.
fn partition(data: &[u8], fragment_length: usize) -> Vec<Vec<u8>> {
    let mut padded = Vec::from(data);
    padded.resize(div_ceil(data.len(), fragment_length) * fragment_length, 0);
    padded.chunks(fragment_length).map(Vec::from).collect()
}

/// Which fragments part `sequence` mixes.
///
/// The first `fragment_count` parts are the fragments themselves. After
/// that the seed is the sequence number and the checksum, both 32-bit
/// big-endian, hashed with SHA-256.
fn choose_fragments(sequence: usize, fragment_count: usize, checksum: u32) -> Vec<usize> {
    if sequence <= fragment_count {
        return alloc::vec![sequence - 1];
    }
    let mut seed = [0u8; 8];
    seed[0..4].copy_from_slice(&(sequence as u32).to_be_bytes());
    seed[4..8].copy_from_slice(&checksum.to_be_bytes());

    let mut xoshiro = Xoshiro256::from_bytes(&seed);
    let degree = xoshiro.choose_degree(fragment_count);
    xoshiro.shuffled((0..fragment_count).collect(), degree)
}

fn xor(target: &mut [u8], other: &[u8]) {
    for (a, &b) in target.iter_mut().zip(other.iter()) {
        *a ^= b;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ur::xoshiro::test_utils::make_message;

    fn hex(bytes: &[u8]) -> alloc::string::String {
        bytes.iter().map(|b| alloc::format!("{b:02x}")).collect()
    }

    #[test]
    fn fragment_lengths_match_the_specification() {
        assert_eq!(fragment_length(12345, 1955), 1764);
        assert_eq!(fragment_length(12345, 30000), 12345);
        assert_eq!(fragment_length(10, 4), 4);
        assert_eq!(fragment_length(10, 5), 5);
        assert_eq!(fragment_length(10, 6), 5);
        assert_eq!(fragment_length(10, 10), 10);
    }

    #[test]
    fn the_message_is_split_as_the_specification_says() {
        let message = make_message("Wolf", 1024);
        let fragments = partition(&message, fragment_length(message.len(), 100));
        let expected = [
            "916ec65cf77cadf55cd7f9cda1a1030026ddd42e905b77adc36e4f2d3ccba44f7f04f2de44f42d84c374a0e149136f25b01852545961d55f7f7a8cde6d0e2ec43f3b2dcb644a2209e8c9e34af5c4747984a5e873c9cf5f965e25ee29039f",
            "df8ca74f1c769fc07eb7ebaec46e0695aea6cbd60b3ec4bbff1b9ffe8a9e7240129377b9d3711ed38d412fbb4442256f1e6f595e0fc57fed451fb0a0101fb76b1fb1e1b88cfdfdaa946294a47de8fff173f021c0e6f65b05c0a494e50791",
            "270a0050a73ae69b6725505a2ec8a5791457c9876dd34aadd192a53aa0dc66b556c0c215c7ceb8248b717c22951e65305b56a3706e3e86eb01c803bbf915d80edcd64d4d41977fa6f78dc07eecd072aae5bc8a852397e06034dba6a0b570",
            "797c3a89b16673c94838d884923b8186ee2db5c98407cab15e13678d072b43e406ad49477c2e45e85e52ca82a94f6df7bbbe7afbed3a3a830029f29090f25217e48d1f42993a640a67916aa7480177354cc7440215ae41e4d02eae9a1912",
            "33a6d4922a792c1b7244aa879fefdb4628dc8b0923568869a983b8c661ffab9b2ed2c149e38d41fba090b94155adbed32f8b18142ff0d7de4eeef2b04adf26f2456b46775c6c20b37602df7da179e2332feba8329bbb8d727a138b4ba7a5",
            "03215eda2ef1e953d89383a382c11d3f2cad37a4ee59a91236a3e56dcf89f6ac81dd4159989c317bd649d9cbc617f73fe10033bd288c60977481a09b343d3f676070e67da757b86de27bfca74392bac2996f7822a7d8f71a489ec6180390",
            "089ea80a8fcd6526413ec6c9a339115f111d78ef21d456660aa85f790910ffa2dc58d6a5b93705caef1091474938bd312427021ad1eeafbd19e0d916ddb111fabd8dcab5ad6a6ec3a9c6973809580cb2c164e26686b5b98cfb017a337968",
            "c7daaa14ae5152a067277b1b3902677d979f8e39cc2aafb3bc06fcf69160a853e6869dcc09a11b5009f91e6b89e5b927ab1527a735660faa6012b420dd926d940d742be6a64fb01cdc0cff9faa323f02ba41436871a0eab851e7f5782d10",
            "fbefde2a7e9ae9dc1e5c2c48f74f6c824ce9ef3c89f68800d44587bedc4ab417cfb3e7447d90e1e417e6e05d30e87239d3a5d1d45993d4461e60a0192831640aa32dedde185a371ded2ae15f8a93dba8809482ce49225daadfbb0fec629e",
            "23880789bdf9ed73be57fa84d555134630e8d0f7df48349f29869a477c13ccca9cd555ac42ad7f568416c3d61959d0ed568b2b81c7771e9088ad7fd55fd4386bafbf5a528c30f107139249357368ffa980de2c76ddd9ce4191376be0e6b5",
            "170010067e2e75ebe2d2904aeb1f89d5dc98cd4a6f2faaa8be6d03354c990fd895a97feb54668473e9d942bb99e196d897e8f1b01625cf48a7b78d249bb4985c065aa8cd1402ed2ba1b6f908f63dcd84b66425df00000000000000000000",
        ];
        assert_eq!(fragments.len(), expected.len());
        for (fragment, e) in fragments.iter().zip(expected) {
            assert_eq!(hex(fragment), e);
        }
        let mut rejoined: Vec<u8> = fragments.into_iter().flatten().collect();
        rejoined.truncate(message.len());
        assert_eq!(rejoined, message);
    }

    #[test]
    fn the_chosen_fragments_match_the_specification() {
        let message = make_message("Wolf", 1024);
        let checksum = crc32(&message);
        let expected = [
            alloc::vec![0],
            alloc::vec![1],
            alloc::vec![2],
            alloc::vec![3],
            alloc::vec![4],
            alloc::vec![5],
            alloc::vec![6],
            alloc::vec![7],
            alloc::vec![8],
            alloc::vec![9],
            alloc::vec![10],
            alloc::vec![9],
            alloc::vec![2, 5, 6, 8, 9, 10],
            alloc::vec![8],
            alloc::vec![1, 5],
            alloc::vec![1],
            alloc::vec![0, 2, 4, 5, 8, 10],
            alloc::vec![5],
            alloc::vec![2],
            alloc::vec![2],
            alloc::vec![0, 1, 3, 4, 5, 7, 9, 10],
            alloc::vec![0, 1, 2, 3, 5, 6, 8, 9, 10],
            alloc::vec![0, 2, 4, 5, 7, 8, 9, 10],
            alloc::vec![3, 5],
            alloc::vec![4],
            alloc::vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
            alloc::vec![0, 1, 3, 4, 5, 6, 7, 9, 10],
            alloc::vec![6],
            alloc::vec![5, 6],
            alloc::vec![7],
        ];
        for (sequence, e) in (1..=expected.len()).zip(expected) {
            let mut indexes = choose_fragments(sequence, 11, checksum);
            indexes.sort_unstable();
            assert_eq!(indexes, e);
        }
    }

    #[test]
    fn the_emitted_parts_match_the_specification() {
        let message = make_message("Wolf", 256);
        let mut encoder = Encoder::new(&message, 30).unwrap();
        let expected = [
            "916ec65cf77cadf55cd7f9cda1a1030026ddd42e905b77adc36e4f2d3c",
            "cba44f7f04f2de44f42d84c374a0e149136f25b01852545961d55f7f7a",
            "8cde6d0e2ec43f3b2dcb644a2209e8c9e34af5c4747984a5e873c9cf5f",
            "965e25ee29039fdf8ca74f1c769fc07eb7ebaec46e0695aea6cbd60b3e",
            "c4bbff1b9ffe8a9e7240129377b9d3711ed38d412fbb4442256f1e6f59",
            "5e0fc57fed451fb0a0101fb76b1fb1e1b88cfdfdaa946294a47de8fff1",
            "73f021c0e6f65b05c0a494e50791270a0050a73ae69b6725505a2ec8a5",
            "791457c9876dd34aadd192a53aa0dc66b556c0c215c7ceb8248b717c22",
            "951e65305b56a3706e3e86eb01c803bbf915d80edcd64d4d0000000000",
            "330f0f33a05eead4f331df229871bee733b50de71afd2e5a79f196de09",
            "3b205ce5e52d8c24a52cffa34c564fa1af3fdffcd349dc4258ee4ee828",
            "dd7bf725ea6c16d531b5f03254783803048ca08b87148daacd1cd7a006",
            "760be7ad1c6187902bbc04f539b9ee5eb8ea6833222edea36031306c01",
            "5bf4031217d2c3254b088fa7553778b5003632f46e21db129416f65b55",
            "73f021c0e6f65b05c0a494e50791270a0050a73ae69b6725505a2ec8a5",
            "b8546ebfe2048541348910267331c643133f828afec9337c318f71b7df",
            "23dedeea74e3a0fb052befabefa13e2f80e4315c9dceed4c8630612e64",
            "d01a8daee769ce34b6b35d3ca0005302724abddae405bdb419c0a6b208",
            "3171c5dc365766eff25ae47c6f10e7de48cfb8474e050e5fe997a6dc24",
            "e055c2433562184fa71b4be94f262e200f01c6f74c284b0dc6fae6673f",
        ];
        assert_eq!(encoder.fragment_count(), 9);
        for (sequence, e) in expected.into_iter().enumerate() {
            assert_eq!(encoder.current_sequence(), sequence);
            let part = encoder.next_part();
            assert_eq!(part.sequence, sequence + 1);
            assert_eq!(part.sequence_count, 9);
            assert_eq!(part.message_length, 256);
            assert_eq!(part.checksum, 23_570_951);
            assert_eq!(hex(&part.data), e);
        }
    }

    #[test]
    fn the_part_cbor_matches_the_specification() {
        let message = make_message("Wolf", 256);
        let mut encoder = Encoder::new(&message, 30).unwrap();
        let expected = [
            "8501091901001a0167aa07581d916ec65cf77cadf55cd7f9cda1a1030026ddd42e905b77adc36e4f2d3c",
            "8502091901001a0167aa07581dcba44f7f04f2de44f42d84c374a0e149136f25b01852545961d55f7f7a",
            "8503091901001a0167aa07581d8cde6d0e2ec43f3b2dcb644a2209e8c9e34af5c4747984a5e873c9cf5f",
            "8504091901001a0167aa07581d965e25ee29039fdf8ca74f1c769fc07eb7ebaec46e0695aea6cbd60b3e",
            "8505091901001a0167aa07581dc4bbff1b9ffe8a9e7240129377b9d3711ed38d412fbb4442256f1e6f59",
            "8506091901001a0167aa07581d5e0fc57fed451fb0a0101fb76b1fb1e1b88cfdfdaa946294a47de8fff1",
            "8507091901001a0167aa07581d73f021c0e6f65b05c0a494e50791270a0050a73ae69b6725505a2ec8a5",
            "8508091901001a0167aa07581d791457c9876dd34aadd192a53aa0dc66b556c0c215c7ceb8248b717c22",
            "8509091901001a0167aa07581d951e65305b56a3706e3e86eb01c803bbf915d80edcd64d4d0000000000",
            "850a091901001a0167aa07581d330f0f33a05eead4f331df229871bee733b50de71afd2e5a79f196de09",
        ];
        for e in expected {
            assert_eq!(hex(&encoder.next_part().cbor()), e);
        }
    }

    #[test]
    fn a_part_survives_cbor() {
        let part = Part {
            sequence: 12,
            sequence_count: 8,
            message_length: 100,
            checksum: 0x1234_5678,
            data: alloc::vec![1, 5, 3, 3, 5],
        };
        let cbor = part.cbor();
        assert_eq!(Part::from_cbor(&cbor).unwrap(), part);

        // The five items in every unsigned width a coordinator may use.
        Part::from_cbor(&[0x85, 0x1, 0x2, 0x3, 0x4, 0x41, 0x5]).unwrap();
        Part::from_cbor(&[
            0x85, 0x19, 0x1, 0x2, 0x19, 0x3, 0x4, 0x19, 0x5, 0x6, 0x19, 0x7, 0x8, 0x41, 0x5,
        ])
        .unwrap();
        Part::from_cbor(&[
            0x85, 0x1a, 0x1, 0x2, 0x3, 0x4, 0x1a, 0x5, 0x6, 0x7, 0x8, 0x1a, 0x9, 0x10, 0x11, 0x12,
            0x1a, 0x13, 0x14, 0x15, 0x16, 0x41, 0x5,
        ])
        .unwrap();

        // Not an array of five, an item of the wrong type, a value too
        // large for the field, a truncation, or trailing bytes.
        for bad in [
            &[0x18][..],
            &[0x1][..],
            &[0x84, 0x1, 0x2, 0x3, 0x4][..],
            &[0x86, 0x1, 0x2, 0x3, 0x4, 0x5, 0x6][..],
            &[0x85, 0x41, 0x2, 0x3, 0x4, 0x41, 0x5][..],
            &[0x85, 0x1, 0x2, 0x3, 0x4, 0x5][..],
            &[0x85, 0x1b, 0, 0, 0, 1, 0, 0, 0, 0, 0x2, 0x3, 0x4, 0x41, 0x5][..],
            &[0x85, 0x1, 0x2, 0x3, 0x4, 0x41][..],
            &[0x85, 0x1, 0x2, 0x3, 0x4, 0x41, 0x5, 0x00][..],
        ] {
            assert_eq!(Part::from_cbor(bad), Err(Fault::Cbor));
        }
    }

    #[test]
    fn a_stream_with_losses_still_finishes() {
        let message = make_message("Wolf", 32767);
        let mut encoder = Encoder::new(&message, 1000).unwrap();
        let mut decoder = Decoder::default();
        let mut skip = false;
        while !decoder.complete() {
            assert_eq!(decoder.message().unwrap(), None);
            let part = encoder.next_part();
            if !skip {
                let _ = decoder.receive(part);
            }
            skip = !skip;
        }
        assert_eq!(decoder.message().unwrap(), Some(message));
    }

    #[test]
    fn a_part_of_another_message_is_refused() {
        let message = make_message("Wolf", 1000);
        let mut encoder = Encoder::new(&message, 10).unwrap();
        let mut decoder = Decoder::default();
        let part = encoder.next_part();
        assert_eq!(
            part.data,
            alloc::vec![0x91, 0x6e, 0xc6, 0x5c, 0xf7, 0x7c, 0xad, 0xf5, 0x5c, 0xd7]
        );
        assert!(decoder.receive(part.clone()).unwrap());
        assert!(!decoder.receive(part).unwrap(), "the same indexes again");

        let mut other = encoder.next_part();
        other.checksum += 1;
        assert_eq!(decoder.receive(other), Err(Fault::InconsistentPart));

        while !decoder.complete() {
            let part = encoder.next_part();
            decoder.receive(part).unwrap();
        }
        assert!(!decoder.receive(encoder.next_part()).unwrap());
        assert_eq!(decoder.message().unwrap(), Some(message));
    }

    #[test]
    fn a_part_that_carries_nothing_is_refused() {
        let mut decoder = Decoder::default();
        let full = Part {
            sequence: 12,
            sequence_count: 8,
            message_length: 100,
            checksum: 0x1234_5678,
            data: alloc::vec![1, 5, 3, 3, 5],
        };
        let mut part = full.clone();
        part.sequence_count = 0;
        assert_eq!(decoder.receive(part), Err(Fault::EmptyPart));
        let mut part = full.clone();
        part.sequence = 0;
        assert_eq!(decoder.receive(part), Err(Fault::InvalidSequence));
        let mut part = full.clone();
        part.message_length = 0;
        assert_eq!(decoder.receive(part), Err(Fault::EmptyPart));
        let mut part = full;
        part.data = Vec::new();
        assert_eq!(decoder.receive(part), Err(Fault::EmptyPart));
    }

    #[test]
    fn a_message_that_does_not_check_out_is_not_returned() {
        // Padding that is not zero: the part claims one byte less than
        // the message it carries.
        let mut encoder = Encoder::new(b"Hello world", 20).unwrap();
        let mut part = encoder.next_part();
        part.message_length -= 1;
        let mut decoder = Decoder::default();
        decoder.receive(part).unwrap();
        assert_eq!(decoder.message(), Err(Fault::InvalidPadding));

        // A checksum that does not match the bytes.
        let mut decoder = Decoder::default();
        decoder
            .receive(Part {
                sequence: 1,
                sequence_count: 1,
                message_length: 3,
                checksum: 0,
                data: b"bad".to_vec(),
            })
            .unwrap();
        assert_eq!(decoder.message(), Err(Fault::MessageChecksum));
    }

    #[test]
    fn an_encoder_needs_a_message_and_a_fragment_length() {
        assert!(matches!(
            Encoder::new(b"foo", 0),
            Err(Fault::InvalidFragmentLen)
        ));
        assert!(matches!(Encoder::new(b"", 1), Err(Fault::EmptyMessage)));
    }
}
