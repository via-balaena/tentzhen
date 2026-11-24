//! JAX's random numbers, as `jax.random` draws them at the JAX that `models/cleanup/uv.lock` pins,
//! 0.6.2: Threefry-2x32 with 20 rounds (`jax/_src/prng.py`, `_threefry2x32_lowering`), in the
//! layout JAX calls "partitionable", its default since 0.5.0 (`jax/_src/config.py`,
//! `jax_threefry_partitionable`). SocialJax's Clean Up draws all its randomness through `split`,
//! `uniform`, `randint` and `permutation` (`clean_up.py` and `movement.py` at the commit
//! `models/cleanup/socialjax.toml` pins), and each is here, bit for bit: tested against what JAX
//! itself draws, the vectors `models/cleanup/random_vectors.py` writes
//! (`jax_s_random_numbers_are_these_bit_for_bit`).
//!
//! In this layout the i-th of n numbers drawn from a key is the hash of the counter i, whatever n
//! is, so a draw of n numbers begins with every smaller draw from the same key
//! (`a_draw_begins_with_every_smaller_draw_from_its_key`). SocialJax draws three times from one key
//! in each step, for apple regrowth, the dirt's noise and the dirt's spawning (`clean_up.py`, lines
//! 1051, 1060 and 1068), so those three draws repeat each other's first numbers: a match has to
//! repeat them too.

/// A raw key: the two uint32s of `jax.random.PRNGKey`, or of one key `jax.random.split` returns.
pub type Key = [u32; 2];

/// Each round's rotation, the first four rounds of every eight and then the second four.
const ROTATIONS: [[u32; 4]; 2] = [[13, 15, 26, 6], [17, 29, 16, 24]];

/// Threefry-2x32 with 20 rounds: the counter `x` hashed under `key`, as `threefry2x32_p` computes
/// it. A key is injected after every four rounds, the three key words in turn, the second word
/// plus how many injections came before it.
pub fn threefry2x32(key: Key, x: [u32; 2]) -> [u32; 2] {
    let ks = [key[0], key[1], key[0] ^ key[1] ^ 0x1BD1_1BDA];
    let mut x = [x[0].wrapping_add(ks[0]), x[1].wrapping_add(ks[1])];
    for i in 0..5 {
        for r in ROTATIONS[i % 2] {
            x[0] = x[0].wrapping_add(x[1]);
            x[1] = x[1].rotate_left(r) ^ x[0];
        }
        x[0] = x[0].wrapping_add(ks[(i + 1) % 3]);
        x[1] = x[1]
            .wrapping_add(ks[(i + 2) % 3])
            .wrapping_add(i as u32 + 1);
    }
    x
}

/// `jax.random.PRNGKey(seed)` for a 32-bit seed, which is what JAX takes without 64-bit types: the
/// seed's bits in the second word and zero in the first (`threefry_seed`).
pub fn key(seed: i32) -> Key {
    [0, seed as u32]
}

/// The hash of the i-th counter under `key`: i as a 64-bit number, its high word first.
fn hash(key: Key, i: usize) -> [u32; 2] {
    let i = i as u64;
    threefry2x32(key, [(i >> 32) as u32, i as u32])
}

/// `jax.random.split(key, n)`: the i-th new key is the i-th counter's hash.
pub fn split(key: Key, n: usize) -> Vec<Key> {
    (0..n).map(|i| hash(key, i)).collect()
}

/// `jax.random.split(key)`, into two.
pub fn split2(key: Key) -> (Key, Key) {
    (hash(key, 0), hash(key, 1))
}

/// `jax.random.bits(key, (n,), uint32)`: the i-th number is the two words of the i-th counter's
/// hash, exclusive-ored. A narrower draw keeps the low bits of each.
pub fn bits(key: Key, n: usize) -> Vec<u32> {
    (0..n)
        .map(|i| {
            let h = hash(key, i);
            h[0] ^ h[1]
        })
        .collect()
}

/// `jax.random.uniform(key, (n,))`, from 0 up to but not including 1: each number's top 23 bits
/// as the mantissa of a float32 from 1 up to 2, less 1, which is exact.
pub fn uniform(key: Key, n: usize) -> Vec<f32> {
    bits(key, n)
        .into_iter()
        .map(|b| f32::from_bits((b >> 9) | 0x3F80_0000) - 1.0)
        .collect()
}

/// An integer type `randint` draws: SocialJax draws int8s and int16s.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Int {
    I8,
    I16,
    I32,
}

impl Int {
    fn bits(self) -> u32 {
        match self {
            Int::I8 => 8,
            Int::I16 => 16,
            Int::I32 => 32,
        }
    }

    fn min(self) -> i64 {
        -(1i64 << (self.bits() - 1))
    }

    fn max(self) -> i64 {
        (1i64 << (self.bits() - 1)) - 1
    }

    /// `v` wrapped into this type, two's complement, as the type's own addition wraps.
    fn wrap(self, v: i64) -> i64 {
        let shift = 64 - self.bits();
        (v << shift) >> shift
    }
}

/// A remainder as XLA takes it, unsigned: by zero, the number itself.
fn rem(a: u64, b: u64) -> u64 {
    if b == 0 { a } else { a % b }
}

/// `jax.random.randint(key, (n,), minval, maxval, dtype)`, from `minval` up to but not including
/// `maxval`, as JAX's `_randint` does it: the key split in two; from each half, n numbers of the
/// type's width; the span, `maxval - minval` in the type's unsigned width, or 1 where `maxval` is
/// not above `minval`, or one more where `maxval` is beyond the type; and each value `minval` plus
/// (high mod span × (2^width mod span) + low mod span) mod span, all in the unsigned width. The
/// bounds are clipped to the type first.
pub fn randint(key: Key, n: usize, minval: i64, maxval: i64, int: Int) -> Vec<i64> {
    let width = int.bits();
    let mask = (1u64 << width) - 1;
    let beyond = maxval > int.max();
    let lo = minval.clamp(int.min(), int.max());
    let hi = maxval.clamp(int.min(), int.max());
    let (k1, k2) = split2(key);
    let (high, low) = (bits(k1, n), bits(k2, n));
    let mut span = (hi - lo) as u64 & mask;
    if hi <= lo {
        span = 1;
    }
    if beyond && hi > lo {
        span = (span + 1) & mask;
    }
    let multiplier = rem(1 << (width / 2), span);
    let multiplier = rem((multiplier * multiplier) & mask, span);
    (0..n)
        .map(|i| {
            let (h, l) = (high[i] as u64 & mask, low[i] as u64 & mask);
            let offset = (rem(h, span) * multiplier + rem(l, span)) & mask;
            int.wrap(lo + rem(offset, span) as i64)
        })
        .collect()
}

/// How many times `jax.random.permutation` sorts n elements: ceil(3 ln n / ln(2^32 - 1)) in JAX
/// (`_shuffle`), which is the fewest r with n³ ≤ (2^32 - 1)^r, counted here in integers: none for
/// one element, once below 1,626, twice from there.
pub fn shuffle_rounds(n: usize) -> u32 {
    assert!(
        n as u64 <= u32::MAX as u64,
        "a shuffle of more than 2^32 - 1 elements"
    );
    let cube = (n.max(1) as u128).pow(3);
    let (mut rounds, mut power) = (0, 1u128);
    while power < cube {
        power *= u32::MAX as u128;
        rounds += 1;
    }
    rounds
}

/// `jax.random.permutation(key, n)`: 0 to n - 1, shuffled as JAX's `_shuffle` does, by sorting
/// them on n fresh random numbers, stably, once for each round, each round's numbers drawn from
/// the second half of the key and the first half kept for the next round. An array of rows is
/// shuffled in the same order: row i of the result is row `permutation(key, rows)[i]`.
pub fn permutation(key: Key, n: usize) -> Vec<u32> {
    let mut order: Vec<u32> = (0..n as u32).collect();
    let mut key = key;
    for _ in 0..shuffle_rounds(n) {
        let (next, sub) = split2(key);
        key = next;
        let mut sorted: Vec<(u32, u32)> = bits(sub, n).into_iter().zip(order).collect();
        sorted.sort_by_key(|&(k, _)| k);
        order = sorted.into_iter().map(|(_, v)| v).collect();
    }
    order
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    use std::path::PathBuf;

    fn root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    /// What `models/cleanup/random_vectors.py` writes: JAX's own draws.
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Vectors {
        jax: String,
        hash: Vec<HashCase>,
        seed: Vec<SeedCase>,
        split: Vec<SplitCase>,
        bits: Vec<BitsCase>,
        uniform: Vec<BitsCase>,
        randint: Vec<RandintCase>,
        permutation: Vec<PermutationCase>,
        rows: Vec<RowsCase>,
    }

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct HashCase {
        key: Key,
        count: [u32; 2],
        out: [u32; 2],
    }

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct SeedCase {
        seed: i32,
        key: Key,
    }

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct SplitCase {
        key: Key,
        n: usize,
        keys: Vec<Key>,
    }

    /// A draw of 32-bit numbers, or of float32s as their bits.
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct BitsCase {
        key: Key,
        bits: Vec<u32>,
    }

    #[derive(Deserialize, Clone, Copy)]
    enum Dtype {
        #[serde(rename = "int8")]
        I8,
        #[serde(rename = "int16")]
        I16,
        #[serde(rename = "int32")]
        I32,
    }

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct RandintCase {
        key: Key,
        minval: i64,
        maxval: i64,
        dtype: Dtype,
        values: Vec<i64>,
    }

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct PermutationCase {
        key: Key,
        n: usize,
        order: Vec<u32>,
    }

    /// `jax.random.permutation` of the rows [0, 1], [2, 3], ... [12, 13], an int16 array.
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct RowsCase {
        key: Key,
        rows: Vec<[i64; 2]>,
    }

    fn vectors() -> Vectors {
        let path = root().join("models/cleanup/random_vectors.json");
        let text = std::fs::read_to_string(&path).unwrap();
        serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
    }

    /// The JAX that `models/cleanup/uv.lock` pins.
    fn locked_jax() -> String {
        let lock = std::fs::read_to_string(root().join("models/cleanup/uv.lock")).unwrap();
        let mut lines = lock.lines();
        while let Some(line) = lines.next() {
            if line == "name = \"jax\"" {
                let version = lines.next().unwrap();
                return version
                    .strip_prefix("version = \"")
                    .and_then(|v| v.strip_suffix('"'))
                    .unwrap()
                    .to_string();
            }
        }
        panic!("uv.lock names no jax");
    }

    /// Random123's known answers for Threefry-2x32, 20 rounds, which JAX's hash gives too (the
    /// vectors' first three cases).
    #[test]
    fn threefry_gives_random123_s_known_answers() {
        assert_eq!(threefry2x32([0, 0], [0, 0]), [0x6b20_0159, 0x99ba_4efe]);
        assert_eq!(
            threefry2x32([u32::MAX, u32::MAX], [u32::MAX, u32::MAX]),
            [0x1cb9_96fc, 0xbb00_2be7]
        );
        assert_eq!(
            threefry2x32([0x1319_8a2e, 0x0370_7344], [0x243f_6a88, 0x85a3_08d3]),
            [0xc492_3a9c, 0x483d_f7a0]
        );
    }

    #[test]
    fn jax_s_random_numbers_are_these_bit_for_bit() {
        let v = vectors();
        assert_eq!(
            v.jax,
            locked_jax(),
            "the vectors are another JAX's than uv.lock pins"
        );
        for c in &v.hash {
            assert_eq!(threefry2x32(c.key, c.count), c.out, "hash of {:?}", c.count);
        }
        for c in &v.seed {
            assert_eq!(key(c.seed), c.key, "PRNGKey({})", c.seed);
        }
        for c in &v.split {
            assert_eq!(split(c.key, c.n), c.keys, "split({:?}, {})", c.key, c.n);
        }
        for c in &v.bits {
            assert_eq!(bits(c.key, c.bits.len()), c.bits, "bits({:?})", c.key);
        }
        for c in &v.uniform {
            let ours: Vec<u32> = uniform(c.key, c.bits.len())
                .iter()
                .map(|f| f.to_bits())
                .collect();
            assert_eq!(ours, c.bits, "uniform({:?})", c.key);
        }
        for c in &v.randint {
            let int = match c.dtype {
                Dtype::I8 => Int::I8,
                Dtype::I16 => Int::I16,
                Dtype::I32 => Int::I32,
            };
            let ours = randint(c.key, c.values.len(), c.minval, c.maxval, int);
            assert_eq!(
                ours, c.values,
                "randint({:?}, {}, {}, {int:?})",
                c.key, c.minval, c.maxval
            );
        }
        for c in &v.permutation {
            assert_eq!(
                permutation(c.key, c.n),
                c.order,
                "permutation({:?}, {})",
                c.key,
                c.n
            );
        }
        for c in &v.rows {
            let rows: Vec<[i64; 2]> = permutation(c.key, 7)
                .into_iter()
                .map(|i| [2 * i as i64, 2 * i as i64 + 1])
                .collect();
            assert_eq!(rows, c.rows, "permutation of rows, {:?}", c.key);
        }
        // Every kind of draw was checked, and the shuffles took both one round and two.
        assert!(!v.hash.is_empty() && !v.seed.is_empty() && !v.split.is_empty());
        assert!(!v.bits.is_empty() && !v.uniform.is_empty() && !v.randint.is_empty());
        assert!(!v.rows.is_empty());
        let rounds: Vec<u32> = v.permutation.iter().map(|c| shuffle_rounds(c.n)).collect();
        assert!(rounds.contains(&1) && rounds.contains(&2));
    }

    #[test]
    fn the_random_vectors_refuse_a_key_they_do_not_know() {
        let text = r#"{"jax":"0.6.2","hash":[],"seed":[],"split":[],"bits":[],"uniform":[],
            "randint":[],"permutation":[],"rows":[],"extra":[]}"#;
        assert!(serde_json::from_str::<Vectors>(text).is_err());
        let text = r#"{"key":[0,0],"bits":[],"n":3}"#;
        assert!(serde_json::from_str::<BitsCase>(text).is_err());
    }

    #[test]
    fn a_draw_begins_with_every_smaller_draw_from_its_key() {
        for k in [key(0), key(42), key(-1)] {
            let (long_bits, long_uniform) = (bits(k, 100), uniform(k, 100));
            for n in [1, 2, 7, 99] {
                assert_eq!(bits(k, n), long_bits[..n]);
                assert_eq!(uniform(k, n), long_uniform[..n]);
            }
        }
    }

    /// The count in integers is JAX's count in floats, ceil(3 ln n / ln(2^32 - 1)), for every n
    /// up to 100,000.
    #[test]
    fn a_shuffle_sorts_once_below_1626_elements_and_twice_from_there() {
        assert_eq!((shuffle_rounds(0), shuffle_rounds(1)), (0, 0));
        assert_eq!((shuffle_rounds(2), shuffle_rounds(1625)), (1, 1));
        assert_eq!((shuffle_rounds(1626), shuffle_rounds(100_000)), (2, 2));
        let ln_max = (u32::MAX as f64).ln();
        for n in 1..=100_000usize {
            let jax = (3.0 * (n as f64).ln() / ln_max).ceil() as u32;
            assert_eq!(shuffle_rounds(n), jax, "{n}");
        }
    }
}
