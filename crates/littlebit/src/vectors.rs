//! Test vectors for the gateware's cocotb tests (docs/littlebit.md, "cocotb tests"): inputs for a
//! LittleBit layer, and what the golden model computes from each, step by step, for the layer
//! engine in `gateware/cores/littlebit.py` to match bit for bit. `cargo run -p tentzhen-littlebit
//! -- vectors`, from the repo root, writes them to [`FILE`]; a test fails if the committed file is
//! not what it writes.
//!
//! No MNIST image is in them, since MNIST's terms are unread and its files are never committed:
//! the inputs are drawn from a fixed seed. The LittleBit layers' cases are r16's two hidden
//! layers, as `models/littlebit/runs/r16/integer.json` has them, a small convolution whose layer
//! is made up here, applied to each window of its grid ([`integer::windows`]), and a layer of
//! Clean Up's conv1's shape, made up too, on a grid of the values an observation holds. The output
//! layer's are r16's, and one made up here whose scores reach both ends of an int32 and tie. The
//! whole network's are r16's and those of [`SWEEP`], the rank sweep's others and E10's, each given
//! the same images.
//!
//! For MNIST's test images, [`mnist`] makes the whole network's case at the time it is needed,
//! into a file outside the repo, as the full-network cocotb test does.

use crate::integer::{self, DenseLayer, LittleBitLayer, Network, Signs};
use crate::mnist::Images;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Where the vectors are, relative to the repo root.
pub const FILE: &str = "gateware/cores/tests/littlebit_vectors.json";

/// Where r16's integer network is, relative to the repo root.
pub const R16: &str = "models/littlebit/runs/r16/integer.json";

/// Where the rank sweep's other integer networks are, relative to the repo root: r = 4, 8, 32 and
/// 64, E4, E5, E7 and E8, then r = 42, E11, the largest that fit, then E10, the wider network
/// (docs/littlebit.md, "Experiments"). Each has a whole-network case.
pub const SWEEP: [&str; 6] = [
    "models/littlebit/runs/r4/integer.json",
    "models/littlebit/runs/r8/integer.json",
    "models/littlebit/runs/r32/integer.json",
    "models/littlebit/runs/r64/integer.json",
    "models/littlebit/runs/r42/integer.json",
    "models/littlebit/runs/e10/integer.json",
];

/// The integer networks at [`R16`] and at each of [`SWEEP`], in its order, read from under the
/// repo root `root`.
pub fn networks(root: &Path) -> Result<(Network, Vec<Network>), String> {
    let read = |path: &str| {
        let text = std::fs::read_to_string(root.join(path)).map_err(|e| format!("{path}: {e}"))?;
        Network::parse(&text).map_err(|e| format!("{path}: {e}"))
    };
    let sweep = SWEEP.iter().map(|p| read(p)).collect::<Result<_, _>>()?;
    Ok((read(R16)?, sweep))
}

/// What [`write`] writes.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Vectors {
    /// How to write them again.
    pub written_by: String,
    /// LittleBit layers'.
    pub cases: Vec<Case>,
    /// Output layers'.
    pub dense_cases: Vec<DenseCase>,
    /// Whole networks'.
    pub network_cases: Vec<NetworkCase>,
}

/// One layer, the inputs it is given in order, and what the golden model makes of each.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Case {
    pub name: String,
    /// Where the layer is: a network file, relative to the repo root, and the layer's index in it.
    /// A layer made up for the test is here instead, in `layer`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub network: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layer_index: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layer: Option<LittleBitLayer>,
    /// For a convolution: the grid its inputs are the windows of.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub convolution: Option<Convolution>,
    pub inputs: Vec<Vec<u8>>,
    /// For each input, in order.
    pub steps: Vec<Steps>,
}

/// A grid `height` x `width` x `channels`, stored row by row and a position's channels together,
/// and the kernel its windows are taken with.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Convolution {
    pub height: usize,
    pub width: usize,
    pub channels: usize,
    pub kernel: usize,
    pub grid: Vec<u8>,
}

/// An output layer, the inputs it is given in order, and the golden model's scores for each and
/// the digit it picks from them ([`integer::argmax`]).
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DenseCase {
    pub name: String,
    /// A network file, relative to the repo root, whose output layer this is; or the layer, made
    /// up for the test, in `layer`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub network: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layer: Option<DenseLayer>,
    pub inputs: Vec<Vec<u8>>,
    pub scores: Vec<Vec<i32>>,
    pub digits: Vec<usize>,
}

/// A whole network, a network file relative to the repo root, the images it is given in order,
/// each 784 pixels as MNIST stores them, and the golden model's scores and digit for each.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NetworkCase {
    pub name: String,
    pub network: String,
    pub images: Vec<Vec<u8>>,
    pub scores: Vec<Vec<i32>>,
    pub digits: Vec<usize>,
}

/// A LittleBit layer's vectors for one input after V, ℓ, U and h ([`integer::Trace`], less p,
/// which the engine never holds), and how many latents were saturated.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Steps {
    pub a: Vec<i32>,
    pub t: Vec<i16>,
    pub s: Vec<i32>,
    pub z: Vec<i32>,
    pub y: Vec<u8>,
    pub saturated: usize,
}

/// SplitMix64 (Steele, Lea and Flood, 2014), as a fixed sequence of numbers to draw inputs from.
struct Draw(u64);

impl Draw {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A number from `low` to `high`, both included.
    fn between(&mut self, low: i64, high: i64) -> i64 {
        low + (self.next() % (high - low + 1) as u64) as i64
    }

    fn bytes(&mut self, n: usize, high: u8) -> Vec<u8> {
        (0..n).map(|_| self.between(0, high.into()) as u8).collect()
    }

    fn int8s(&mut self, n: usize) -> Vec<i8> {
        (0..n).map(|_| self.between(-128, 127) as i8).collect()
    }

    fn signs(&mut self, rows: usize, rank: usize) -> Vec<Signs> {
        (0..rows)
            .map(|_| {
                Signs(
                    (0..rank)
                        .map(|_| if self.next() & 1 == 0 { 1 } else { -1 })
                        .collect(),
                )
            })
            .collect()
    }
}

fn steps(layer: &LittleBitLayer, inputs: &[Vec<u8>]) -> Result<Vec<Steps>, String> {
    inputs
        .iter()
        .map(|x| {
            let t = layer.trace(x)?;
            Ok(Steps {
                a: t.a,
                t: t.t,
                s: t.s,
                z: t.z,
                y: t.y,
                saturated: t.saturated,
            })
        })
        .collect()
}

/// A hidden layer of `net` given all zeros, every input at `high`, and four inputs drawn from 0
/// to `high`.
fn hidden(net: &Network, index: usize, high: u8, draw: &mut Draw) -> Result<Case, String> {
    let layer = &net.layers[index];
    let mut inputs = vec![vec![0; layer.inputs], vec![high; layer.inputs]];
    inputs.extend((0..4).map(|_| draw.bytes(layer.inputs, high)));
    Ok(Case {
        name: format!("{}-layer-{}", net.run, index + 1),
        network: Some(R16.into()),
        layer_index: Some(index),
        layer: None,
        convolution: None,
        steps: steps(layer, &inputs)?,
        inputs,
    })
}

/// A 3 x 3 convolution from 2 channels to 4, at rank 3, over a 5 x 5 grid. Its scales and biases
/// are drawn across their whole range, and its shifts set so that some latents saturate and some
/// outputs clamp at each end.
fn convolution(draw: &mut Draw) -> Result<Case, String> {
    let (height, width, channels, kernel, rank, outputs) = (5, 5, 2, 3, 3, 4);
    let inputs = kernel * kernel * channels;
    let (latent_shift, output_shift) = (8, 13);
    let layer = LittleBitLayer {
        inputs,
        outputs,
        rank,
        input_exponent: 0,
        g: draw.int8s(inputs),
        g_exponent: 0,
        v: draw.signs(inputs, rank),
        l: draw.int8s(rank),
        l_exponent: 0,
        latent_shift,
        u: draw.signs(outputs, rank),
        h: draw.int8s(outputs),
        h_exponent: 0,
        bias: (0..outputs)
            .map(|_| draw.between(-(1 << 20), 1 << 20) as i32)
            .collect(),
        output_shift,
        output_exponent: (latent_shift + output_shift) as i32,
    };
    let grid = draw.bytes(height * width * channels, 255);
    let windows = integer::windows(&grid, height, width, channels, kernel)?;
    Ok(Case {
        name: "convolution-3x3".into(),
        network: None,
        layer_index: None,
        steps: steps(&layer, &windows)?,
        layer: Some(layer),
        convolution: Some(Convolution {
            height,
            width,
            channels,
            kernel,
            grid,
        }),
        inputs: windows,
    })
}

/// The channels of a Clean Up observation that hold a byte, its two inventory counts; the other 17
/// hold 0 or 1. SocialJax's `clean_up.py` (at 18e2171) builds each cell's 19 values from one-hots
/// and booleans, but for those two, `state.agent_invs`, int8s (`_get_obs` and `combine_channels`).
pub const CLEAN_UP_BYTE_CHANNELS: [usize; 2] = [16, 17];

/// A layer of Clean Up's conv1's shape, 5 x 5 over 19 channels to 8 at rank 4, applied to the 121
/// windows of an 11 x 11 grid, for the hardwired kernel (`gateware/cores/littlebit_kernel.py`).
/// Each grid value is 0 or 1 but in [`CLEAN_UP_BYTE_CHANNELS`], 0 to 255, as an observation's are.
/// The grid's top-left 5 x 5 cells are at their largest and its bottom-right 5 x 5 all 0, so one
/// window is all at its largest and one all 0. The layer's scales and biases are drawn across
/// their whole range, and its shifts are the smallest at which at most an eighth of the latents
/// saturate and an eighth of the outputs clamp at 255. It draws from a seed of its own, so the
/// other cases are as they were before it.
fn clean_up_conv1() -> Result<Case, String> {
    let mut draw = Draw(0xC1EA_4C01);
    let (height, width, channels, kernel, rank, outputs) = (11, 11, 19, 5, 4, 8);
    let inputs = kernel * kernel * channels;
    let mut layer = LittleBitLayer {
        inputs,
        outputs,
        rank,
        input_exponent: 0,
        g: draw.int8s(inputs),
        g_exponent: 0,
        v: draw.signs(inputs, rank),
        l: draw.int8s(rank),
        l_exponent: 0,
        latent_shift: 0,
        u: draw.signs(outputs, rank),
        h: draw.int8s(outputs),
        h_exponent: 0,
        bias: (0..outputs)
            .map(|_| draw.between(-(1 << 20), 1 << 20) as i32)
            .collect(),
        output_shift: 0,
        output_exponent: 0,
    };
    let mut grid = Vec::with_capacity(height * width * channels);
    for row in 0..height {
        for col in 0..width {
            for ch in 0..channels {
                let high = if CLEAN_UP_BYTE_CHANNELS.contains(&ch) {
                    255
                } else {
                    1
                };
                grid.push(if row < 5 && col < 5 {
                    high
                } else if row >= height - 5 && col >= width - 5 {
                    0
                } else {
                    draw.between(0, high.into()) as u8
                });
            }
        }
    }
    let windows = integer::windows(&grid, height, width, channels, kernel)?;
    let eighth = |count: usize, of: usize| count * 8 <= of;
    layer.latent_shift = (0..=40)
        .find(|&shift| {
            layer.latent_shift = shift;
            let saturated: usize = windows
                .iter()
                .map(|x| layer.trace(x).map(|t| t.saturated).unwrap_or(rank))
                .sum();
            eighth(saturated, windows.len() * rank)
        })
        .ok_or("no latent shift up to 40 saturates few enough latents")?;
    layer.output_shift = (0..=31)
        .find(|&shift| {
            layer.output_shift = shift;
            let clamped: usize = windows
                .iter()
                .map(|x| {
                    layer
                        .trace(x)
                        .map(|t| t.y.iter().filter(|&&y| y == 255).count())
                        .unwrap_or(outputs)
                })
                .sum();
            eighth(clamped, windows.len() * outputs)
        })
        .ok_or("no output shift up to 31 clamps few enough outputs")?;
    layer.output_exponent = (layer.latent_shift + layer.output_shift) as i32;
    Ok(Case {
        name: "clean-up-conv1".into(),
        network: None,
        layer_index: None,
        steps: steps(&layer, &windows)?,
        layer: Some(layer),
        convolution: Some(Convolution {
            height,
            width,
            channels,
            kernel,
            grid,
        }),
        inputs: windows,
    })
}

/// Each input's scores from `layer`, and the digit picked from them.
fn scored(layer: &DenseLayer, inputs: &[Vec<u8>]) -> Result<(Vec<Vec<i32>>, Vec<usize>), String> {
    let scores = inputs
        .iter()
        .map(|x| layer.scores(x))
        .collect::<Result<Vec<_>, _>>()?;
    let digits = scores.iter().map(|s| integer::argmax(s)).collect();
    Ok((scores, digits))
}

/// r16's output layer given all zeros, all 255, and four inputs drawn from 0 to 255.
fn output(net: &Network, draw: &mut Draw) -> Result<DenseCase, String> {
    let n = net.output.inputs;
    let mut inputs = vec![vec![0; n], vec![255; n]];
    inputs.extend((0..4).map(|_| draw.bytes(n, 255)));
    let (scores, digits) = scored(&net.output, &inputs)?;
    Ok(DenseCase {
        name: format!("{}-output", net.run),
        network: Some(R16.into()),
        layer: None,
        inputs,
        scores,
        digits,
    })
}

/// An output layer from 4 inputs to 5, made up so its scores reach both ends of an int32 and tie.
/// Output 0 reaches -2^31 when every input is 255, and output 3 reaches 2^31 - 1. Outputs 1 and 3
/// tie whenever the last input is 0, and outputs 1, 2 and 3 all tie when every input is, so the
/// first of the highest is not always the last.
fn extremes(draw: &mut Draw) -> Result<DenseCase, String> {
    let top = i32::MAX - 4 * 255 * 127;
    let layer = DenseLayer {
        inputs: 4,
        outputs: 5,
        input_exponent: 0,
        weight: vec![
            vec![-128; 4],
            vec![127, 127, 127, 0],
            vec![0; 4],
            vec![127; 4],
            vec![-128, 127, -128, 127],
        ],
        weight_exponent: 0,
        bias: vec![i32::MIN + 4 * 255 * 128, top, top, top, 0],
    };
    let mut inputs = vec![vec![0; 4], vec![255; 4], vec![5, 9, 200, 0]];
    inputs.extend((0..3).map(|_| draw.bytes(4, 255)));
    let (scores, digits) = scored(&layer, &inputs)?;
    Ok(DenseCase {
        name: "dense-extremes".into(),
        network: None,
        layer: Some(layer),
        inputs,
        scores,
        digits,
    })
}

/// The whole of `net`, from the network file at `path`, given `images`.
fn network_case(
    name: &str,
    net: &Network,
    path: &str,
    images: Vec<Vec<u8>>,
) -> Result<NetworkCase, String> {
    let scores = images
        .iter()
        .map(|image| Ok(net.scores(image)?.0))
        .collect::<Result<Vec<_>, String>>()?;
    let digits = scores.iter().map(|s| integer::argmax(s)).collect();
    Ok(NetworkCase {
        name: name.into(),
        network: path.into(),
        images,
        scores,
        digits,
    })
}

/// The whole of r16 given MNIST's first `count` test images, for the full-network cocotb test,
/// which writes it to a file outside the repo: MNIST is never committed.
pub fn mnist(r16: &Network, test: &Images, count: usize) -> Result<NetworkCase, String> {
    if count > test.len() {
        return Err(format!("{count} images, of the {} there are", test.len()));
    }
    let images = (0..count).map(|k| test.image(k).to_vec()).collect();
    network_case(&format!("{}-mnist-test", r16.run), r16, R16, images)
}

/// The vectors, from r16's integer network and the sweep's others, `sweep`, one for each of
/// [`SWEEP`] in its order. The sweep's networks are given r16's images, so adding them draws
/// nothing and changes no case before them.
pub fn vectors(r16: &Network, sweep: &[Network]) -> Result<Vectors, String> {
    if sweep.len() != SWEEP.len() {
        return Err(format!(
            "{} sweep networks for {} files",
            sweep.len(),
            SWEEP.len()
        ));
    }
    let mut draw = Draw(0x7E47_2E4E);
    // r16's first layer takes pixels shifted right by 1, so 0 to 127.
    let high = 255u8 >> r16.pixel_shift;
    let cases = vec![
        hidden(r16, 0, high, &mut draw)?,
        hidden(r16, 1, 255, &mut draw)?,
        convolution(&mut draw)?,
        clean_up_conv1()?,
    ];
    let dense_cases = vec![output(r16, &mut draw)?, extremes(&mut draw)?];
    let mut images = vec![vec![0; 784], vec![255; 784]];
    images.extend((0..4).map(|_| draw.bytes(784, 255)));
    Ok(Vectors {
        written_by: "cargo run -p tentzhen-littlebit -- vectors".into(),
        cases,
        dense_cases,
        network_cases: std::iter::once((R16, r16))
            .chain(SWEEP.into_iter().zip(sweep))
            .map(|(path, net)| network_case(&net.run, net, path, images.clone()))
            .collect::<Result<_, _>>()?,
    })
}

/// The vectors as [`FILE`] holds them, or a case as [`mnist`] makes it: JSON with one input or
/// step per line, and a newline at the end.
pub fn to_json<T: Serialize>(vectors: &T) -> String {
    let mut text = serde_json::to_string_pretty(vectors).expect("vectors serialize");
    // serde_json writes each number of an array on a line of its own; fold each array of numbers
    // onto one line, so a 784-input vector takes one line, not 786.
    let mut folded = String::with_capacity(text.len());
    let mut lines = text.lines().peekable();
    while let Some(line) = lines.next() {
        if line.trim_end().ends_with('[')
            && lines.peek().is_some_and(|next| {
                next.trim()
                    .starts_with(|c: char| c == '-' || c.is_ascii_digit())
            })
        {
            folded.push_str(line.trim_end());
            for item in lines.by_ref() {
                let item = item.trim();
                if item.starts_with(']') {
                    folded.push_str(item);
                    break;
                }
                folded.push_str(item);
                if item.ends_with(',') {
                    folded.push(' ');
                }
            }
        } else {
            folded.push_str(line);
        }
        folded.push('\n');
    }
    text = folded;
    text
}
