//! MNIST, read only as `models/littlebit/mnist.toml` pins it: each of its four files is refused
//! unless its sha256 is its pin, and each IDX header unless it gives the sizes MNIST has.

use flate2::read::GzDecoder;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::Path;

/// The pins, relative to the repo root.
pub const PINS: &str = "models/littlebit/mnist.toml";

/// MNIST's four files, as the mirror names them.
pub const FILES: [&str; 4] = [
    "train-images-idx3-ubyte.gz",
    "train-labels-idx1-ubyte.gz",
    "t10k-images-idx3-ubyte.gz",
    "t10k-labels-idx1-ubyte.gz",
];

/// `models/littlebit/mnist.toml`: where MNIST is fetched from and what each file's sha256 is.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pins {
    pub mirror: String,
    /// When and how the pins were taken, in words.
    pub pinned: String,
    pub sha256: BTreeMap<String, String>,
}

impl Pins {
    /// The pins under `root`, refused unless they pin exactly [`FILES`].
    pub fn load(root: &Path) -> Result<Self, String> {
        let path = root.join(PINS);
        let text = fs::read_to_string(&path).map_err(|e| format!("{PINS}: {e}"))?;
        Self::parse(&text)
    }

    pub fn parse(text: &str) -> Result<Self, String> {
        let pins: Pins = toml::from_str(text).map_err(|e| format!("{PINS}: {e}"))?;
        if !pins.sha256.keys().map(String::as_str).eq(sorted(FILES)) {
            return Err(format!(
                "{PINS}: pins {:?}, not {FILES:?}",
                pins.sha256.keys()
            ));
        }
        Ok(pins)
    }
}

fn sorted(files: [&str; 4]) -> impl Iterator<Item = &str> {
    let mut f = files;
    f.sort_unstable();
    f.into_iter()
}

/// The sha256 of `bytes` in lowercase hex.
pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// A set of images: 28 x 28 pixels each, row by row, and a digit for each.
pub struct Images {
    pub pixels: Vec<u8>,
    pub labels: Vec<u8>,
}

impl Images {
    pub fn len(&self) -> usize {
        self.labels.len()
    }

    pub fn is_empty(&self) -> bool {
        self.labels.is_empty()
    }

    /// Image `i`'s 784 pixels.
    pub fn image(&self, i: usize) -> &[u8] {
        &self.pixels[i * 784..(i + 1) * 784]
    }
}

/// The training set and the test set from the files in `folder`, each checked against its pin.
pub fn load(root: &Path, folder: &Path) -> Result<(Images, Images), String> {
    let pins = Pins::load(root)?;
    let read = |name: &str, magic: u32, sizes: &[u32]| -> Result<Vec<u8>, String> {
        let path = folder.join(name);
        let packed = fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let got = sha256_hex(&packed);
        if got != pins.sha256[name] {
            return Err(format!(
                "{name}: its sha256 is {got}, not the pinned {}",
                pins.sha256[name]
            ));
        }
        let mut data = Vec::new();
        GzDecoder::new(&packed[..])
            .read_to_end(&mut data)
            .map_err(|e| format!("{name}: {e}"))?;
        if data.len() < 4 * (sizes.len() + 1) {
            return Err(format!(
                "{name}: {} bytes, too few for its header",
                data.len()
            ));
        }
        let header: Vec<u32> = (0..=sizes.len())
            .map(|i| u32::from_be_bytes(data[4 * i..4 * i + 4].try_into().unwrap()))
            .collect();
        let want: Vec<u32> = std::iter::once(magic)
            .chain(sizes.iter().copied())
            .collect();
        let body = data.split_off(4 * (sizes.len() + 1));
        let count: u32 = sizes.iter().product();
        if header != want || body.len() != count as usize {
            return Err(format!(
                "{name}: header {header:?} and {} bytes, not {want:?} and {count}",
                body.len()
            ));
        }
        Ok(body)
    };
    let set = |images: &str, labels: &str, n: u32| -> Result<Images, String> {
        Ok(Images {
            pixels: read(images, 2051, &[n, 28, 28])?,
            labels: read(labels, 2049, &[n])?,
        })
    };
    Ok((
        set(FILES[0], FILES[1], 60_000)?,
        set(FILES[2], FILES[3], 10_000)?,
    ))
}
