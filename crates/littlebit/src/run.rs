//! A training run, as `models/littlebit/runs/<run>/` holds it: `summary.json`, what was run and
//! what it scored, and `weights.f32`, its parameters as little-endian float32 in the order and
//! shapes the summary's `layout` gives, whose sha256 the summary names.

use crate::mnist::{Images, sha256_hex};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

/// What `models/littlebit/runfiles.py` writes as `summary.json`, for `e1_fp32.py`, for
/// `train_rank.py`, whose runs alone have `littlebit`, and for `train_baseline.py`, whose runs alone
/// have `quantized`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Summary {
    pub run: String,
    pub model: String,
    /// The weight matrices' entries, biases left out: for a LittleBit run, those of the dense
    /// matrices its layers stand for.
    pub weights: usize,
    /// Every parameter, biases too.
    pub parameters: usize,
    pub recipe: Recipe,
    pub littlebit: Option<LittleBit>,
    pub quantized: Option<Quantized>,
    pub mnist_sha256: BTreeMap<String, String>,
    pub train_correct: usize,
    pub train_images: usize,
    pub test_correct: usize,
    pub test_images: usize,
    pub curve: Vec<Epoch>,
    pub layout: Vec<Tensor>,
    pub weights_sha256: String,
    pub seconds: f64,
    pub device: String,
    pub versions: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Recipe {
    pub optimizer: String,
    pub learning_rate: f64,
    pub schedule: String,
    pub epochs: usize,
    pub batch: usize,
    pub seed: u64,
    /// How a pixel p, 0-255, enters the network: [`Run::correct`] knows `x / 255` and
    /// `floor(p / 2) / 128`, and refuses any other.
    pub pixels: String,
    pub deterministic_algorithms: bool,
}

/// What a LittleBit run adds: its rank, the run it started from, and the grid its activations
/// are on.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LittleBit {
    pub rank: usize,
    pub start: Start,
    pub init: String,
    pub sign: String,
    pub activations: String,
    /// For each layer but the last, in order, the e of its outputs' grid: each output is
    /// floor(y / 2^e), clamped to 0..255, times 2^e.
    pub activation_exponents: Vec<i32>,
    /// PyTorch's count on the test images after Dual-SVID, before fine-tuning. Nothing here
    /// re-checks it: the weights it was scored with are not committed.
    pub init_test_correct: usize,
}

/// What a baseline run, E2 or E3, adds: how its hidden layers' weights are kept, the run it
/// started from, and the grid its activations are on.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Quantized {
    /// `int8`, dense layers whose weights are float in training and int8 at export, or `binary`,
    /// h * (sign(W) x) + b with one scale per output ([`Run::predict`] refuses any other).
    pub weights: String,
    pub start: Start,
    pub init: String,
    /// How a binary run's signs are taken; an int8 run has none.
    pub sign: Option<String>,
    pub activations: String,
    /// As a LittleBit run's are: for each layer but the last, the e of its outputs' grid.
    pub activation_exponents: Vec<i32>,
    /// PyTorch's count on the test images as the run started, before fine-tuning. Nothing here
    /// re-checks it.
    pub init_test_correct: usize,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Start {
    pub run: String,
    pub weights_sha256: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Epoch {
    pub epoch: usize,
    pub train_loss: f64,
    pub test_correct: usize,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tensor {
    pub name: String,
    pub shape: Vec<usize>,
}

impl Summary {
    pub fn parse(text: &str) -> Result<Self, String> {
        serde_json::from_str(text).map_err(|e| format!("summary.json: {e}"))
    }
}

/// A run's summary and its parameters, by name, each checked against the summary.
pub struct Run {
    pub summary: Summary,
    pub tensors: BTreeMap<String, Vec<f32>>,
}

/// One layer of a run's MLP, with its parameters.
enum Layer<'a> {
    /// `w` is out x in, row-major.
    Dense { w: &'a [f32], b: &'a [f32] },
    /// y = h * (S x) + b: `s` is the signs of the latent weights, out x in, each +1 where the
    /// latent weight is >= 0 and -1 elsewhere, as `littlebit.Sign` has them.
    Binary {
        s: Vec<f32>,
        h: &'a [f32],
        b: &'a [f32],
    },
    /// y = h * (U_s (l * (V_s^T (g * x)))) + b: `v` is V's signs, in x r, and `u` U's, out x r,
    /// each +1 where the latent weight is >= 0 and -1 elsewhere, as `littlebit.Sign` has them.
    LittleBit {
        g: &'a [f32],
        v: Vec<f32>,
        l: &'a [f32],
        u: Vec<f32>,
        h: &'a [f32],
        b: &'a [f32],
    },
}

fn signs(latent: &[f32]) -> Vec<f32> {
    latent
        .iter()
        .map(|&w| if w >= 0.0 { 1.0 } else { -1.0 })
        .collect()
}

impl Layer<'_> {
    /// The layer's outputs for `x`, every product and sum in f32 and in the order the PyTorch
    /// model takes them; or what does not fit, if `x` is not as long as the layer's inputs.
    fn apply(&self, x: &[f32]) -> Result<Vec<f32>, String> {
        match self {
            Layer::Dense { w, b } => {
                let n_in = x.len();
                if w.len() != b.len() * n_in {
                    return Err(format!("takes {} inputs, not {n_in}", w.len() / b.len()));
                }
                Ok(b.iter()
                    .enumerate()
                    .map(|(o, &bias)| {
                        w[o * n_in..(o + 1) * n_in]
                            .iter()
                            .zip(x)
                            .fold(0.0f32, |s, (&wi, &xi)| s + wi * xi)
                            + bias
                    })
                    .collect())
            }
            Layer::Binary { s, h, b } => {
                let n_in = x.len();
                if s.len() != h.len() * n_in {
                    return Err(format!("takes {} inputs, not {n_in}", s.len() / h.len()));
                }
                Ok(h.iter()
                    .zip(b.iter())
                    .enumerate()
                    .map(|(o, (&h, &bias))| {
                        s[o * n_in..(o + 1) * n_in]
                            .iter()
                            .zip(x)
                            .fold(0.0f32, |sum, (&si, &xi)| sum + xi * si)
                            * h
                            + bias
                    })
                    .collect())
            }
            Layer::LittleBit { g, v, l, u, h, b } => {
                if x.len() != g.len() {
                    return Err(format!("takes {} inputs, not {}", g.len(), x.len()));
                }
                let r = l.len();
                let scaled: Vec<f32> = x.iter().zip(g.iter()).map(|(&xi, &gi)| xi * gi).collect();
                let latent: Vec<f32> = (0..r)
                    .map(|k| {
                        scaled
                            .iter()
                            .enumerate()
                            .fold(0.0f32, |s, (i, &xi)| s + xi * v[i * r + k])
                            * l[k]
                    })
                    .collect();
                Ok((0..h.len())
                    .map(|o| {
                        u[o * r..(o + 1) * r]
                            .iter()
                            .zip(&latent)
                            .fold(0.0f32, |s, (&ui, &ti)| s + ti * ui)
                            * h[o]
                            + b[o]
                    })
                    .collect())
            }
        }
    }
}

impl Run {
    /// The run in `folder`: its weights refused unless their sha256 is the summary's and their size
    /// is the layout's.
    pub fn load(folder: &Path) -> Result<Self, String> {
        let text = fs::read_to_string(folder.join("summary.json"))
            .map_err(|e| format!("{}: {e}", folder.display()))?;
        let summary = Summary::parse(&text)?;
        let bytes = fs::read(folder.join("weights.f32"))
            .map_err(|e| format!("{}: {e}", folder.display()))?;
        let got = sha256_hex(&bytes);
        if got != summary.weights_sha256 {
            return Err(format!(
                "weights.f32: its sha256 is {got}, not the summary's {}",
                summary.weights_sha256
            ));
        }
        let mut values = bytes
            .chunks_exact(4)
            .map(|b| f32::from_le_bytes(b.try_into().unwrap()));
        let mut tensors = BTreeMap::new();
        for t in &summary.layout {
            let n: usize = t.shape.iter().product();
            let v: Vec<f32> = values.by_ref().take(n).collect();
            if v.len() != n {
                return Err(format!("weights.f32 ends inside {}", t.name));
            }
            tensors.insert(t.name.clone(), v);
        }
        if values.next().is_some() || bytes.len() % 4 != 0 {
            return Err("weights.f32 is longer than the layout".into());
        }
        Ok(Run { summary, tensors })
    }

    /// Layer `i`: a binary layer if the run has `<i>.weight`, `.h` and `.bias` and its summary
    /// says its weights are binary; a Linear if it has `<i>.weight` and `<i>.bias` alone; a
    /// LittleBit layer if it has `<i>.g`, `.v`, `.l`, `.u`, `.h` and `.bias` and the summary gives
    /// its rank.
    fn layer(&self, i: usize) -> Result<Layer<'_>, String> {
        let get = |name: &str| self.tensors.get(&format!("{i}.{name}")).map(Vec::as_slice);
        let missing = || format!("no layer {i} in the layout");
        let b = get("bias").filter(|b| !b.is_empty()).ok_or_else(missing)?;
        if let Some(w) = get("weight") {
            if w.len() % b.len() != 0 {
                return Err(format!("layer {i}'s weight is not a whole number of rows"));
            }
            let binary = self
                .summary
                .quantized
                .as_ref()
                .is_some_and(|q| q.weights == "binary");
            return match get("h") {
                None => Ok(Layer::Dense { w, b }),
                Some(h) if binary && h.len() == b.len() => Ok(Layer::Binary { s: signs(w), h, b }),
                Some(_) => Err(format!(
                    "layer {i} has a weight and an h, but is not a binary layer as the summary \
                     gives it"
                )),
            };
        }
        let (g, v, l, u, h) = match (get("g"), get("v"), get("l"), get("u"), get("h")) {
            (Some(g), Some(v), Some(l), Some(u), Some(h)) => (g, v, l, u, h),
            _ => return Err(missing()),
        };
        let rank = match &self.summary.littlebit {
            Some(lb) => lb.rank,
            None => {
                return Err(format!(
                    "layer {i} is LittleBit, but the summary gives no rank"
                ));
            }
        };
        if l.len() != rank
            || v.len() != g.len() * rank
            || u.len() != h.len() * rank
            || b.len() != h.len()
        {
            return Err(format!(
                "layer {i}'s shapes are not a LittleBit layer at r = {rank}"
            ));
        }
        Ok(Layer::LittleBit {
            g,
            v: signs(v),
            l,
            u: signs(u),
            h,
            b,
        })
    }

    /// The images the run's MLP gets right ([`Run::predict`]).
    pub fn correct(&self, linears: &[usize], set: &Images) -> Result<usize, String> {
        let digits = self.predict(linears, set)?;
        Ok(digits
            .iter()
            .zip(&set.labels)
            .filter(|&(&d, &l)| d == usize::from(l))
            .count())
    }

    /// The digit the run's MLP predicts for each image: layers `linears`, in order, each a Linear,
    /// a binary layer or a LittleBit layer ([`Layer`]), with pixels entering as the recipe's
    /// `pixels` says. Between layers, a ReLU, or for a LittleBit run or a baseline its activations'
    /// grid. The predicted digit is the first of the largest scores. Every product and sum is f32.
    pub fn predict(&self, linears: &[usize], set: &Images) -> Result<Vec<usize>, String> {
        let pixel: fn(u8) -> f32 = match self.summary.recipe.pixels.as_str() {
            "x / 255" => |p| p as f32 / 255.0,
            "floor(p / 2) / 128" => |p| (p >> 1) as f32 / 128.0,
            other => return Err(format!("pixels {other:?} is no mapping this crate knows")),
        };
        let exponents = match (&self.summary.littlebit, &self.summary.quantized) {
            (None, None) => None,
            (Some(lb), None) => Some(&lb.activation_exponents),
            (None, Some(q)) if q.weights == "int8" || q.weights == "binary" => {
                Some(&q.activation_exponents)
            }
            (None, Some(q)) => {
                return Err(format!(
                    "weights {:?} is no kind this crate knows",
                    q.weights
                ));
            }
            (Some(_), Some(_)) => {
                return Err("the summary is both a LittleBit run and a baseline".into());
            }
        };
        let steps: Option<Vec<f32>> = match exponents {
            None => None,
            Some(e) if e.len() + 1 == linears.len() => {
                Some(e.iter().map(|&e| 2f32.powi(e)).collect())
            }
            Some(e) => {
                return Err(format!(
                    "{} activation exponents for the {} gaps between {} layers",
                    e.len(),
                    linears.len().saturating_sub(1),
                    linears.len()
                ));
            }
        };
        let layers: Vec<Layer> = linears
            .iter()
            .map(|&i| self.layer(i))
            .collect::<Result<_, _>>()?;
        let mut digits = Vec::with_capacity(set.len());
        for k in 0..set.len() {
            let mut a: Vec<f32> = set.image(k).iter().map(|&p| pixel(p)).collect();
            for (j, layer) in layers.iter().enumerate() {
                let mut out = layer
                    .apply(&a)
                    .map_err(|e| format!("layer {} {e}", linears[j]))?;
                if j + 1 < layers.len() {
                    match &steps {
                        None => out.iter_mut().for_each(|y| *y = y.max(0.0)),
                        Some(steps) => out.iter_mut().for_each(|y| {
                            *y = (*y / steps[j]).floor().clamp(0.0, 255.0) * steps[j]
                        }),
                    }
                }
                a = out;
            }
            let best = (0..a.len()).fold(0, |m, i| if a[i] > a[m] { i } else { m });
            digits.push(best);
        }
        Ok(digits)
    }
}
