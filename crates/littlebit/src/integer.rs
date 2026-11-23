//! The integer network a run exports, as `models/littlebit/runs/<run>/integer.json` holds it, and
//! the golden model that runs it (docs/littlebit.md, "Fixed-point formats" and "The golden
//! model"); and a baseline's integer network, E2's or E3's, which is not written out
//! ([`Baseline`]). Every value is an integer, every widening is written out, and an overflow is an error,
//! never a wrap. A right shift (`>>`) is arithmetic, so it rounds toward minus infinity, as the
//! gateware's will; nothing rounds to nearest.
//!
//! The layer is a matrix applied to one vector. An MLP layer applies it to one vector per image;
//! a convolution applies it to each window of its grid, [`windows`] (docs/littlebit.md, "After
//! MNIST").

use crate::mnist::Images;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// What [`crate::export::export`] writes: LittleBit layers, then one dense output layer.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Network {
    /// The run this was exported from, and its weights' sha256.
    pub run: String,
    pub weights_sha256: String,
    /// A pixel p, 0-255, enters as p >> pixel_shift, on a grid of 2^(pixel_shift - 8).
    pub pixel_shift: u32,
    pub layers: Vec<LittleBitLayer>,
    pub output: DenseLayer,
}

/// One LittleBit layer, from uint8 inputs x to uint8 outputs y:
///
/// 1. p_i = x_i * g_i, an int16;
/// 2. a_k = sum over i of V[i][k] p_i, an int32, where V's entries are +1 or -1;
/// 3. t_k = (a_k * l_k) >> latent_shift, saturated to an int16;
/// 4. s_o = sum over k of U[o][k] t_k, an int32;
/// 5. z_o = s_o * h_o + bias_o, an int32;
/// 6. y_o = z_o >> output_shift, clamped to 0..255.
///
/// g, l and h are int8, each on a grid of 2^exponent: g_i stands for g_i * 2^g_exponent. Along
/// the way the exponents add up, input_exponent + g_exponent + l_exponent + latent_shift +
/// h_exponent + output_shift = output_exponent, which [`Network::parse`] checks.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LittleBitLayer {
    pub inputs: usize,
    pub outputs: usize,
    pub rank: usize,
    pub input_exponent: i32,
    pub g: Vec<i8>,
    pub g_exponent: i32,
    /// V, inputs x rank: row i holds input i's sign for each of the rank factors.
    pub v: Vec<Signs>,
    pub l: Vec<i8>,
    pub l_exponent: i32,
    pub latent_shift: u32,
    /// U, outputs x rank.
    pub u: Vec<Signs>,
    pub h: Vec<i8>,
    pub h_exponent: i32,
    pub bias: Vec<i32>,
    pub output_shift: u32,
    pub output_exponent: i32,
}

/// The output layer: score_o = sum over i of weight[o][i] x_i, plus bias_o, an int32, from uint8
/// inputs. The weights are int8 on one grid, 2^weight_exponent.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DenseLayer {
    pub inputs: usize,
    pub outputs: usize,
    pub input_exponent: i32,
    /// outputs x inputs.
    pub weight: Vec<Vec<i8>>,
    pub weight_exponent: i32,
    pub bias: Vec<i32>,
}

/// A baseline's integer network, E2's or E3's ([`crate::export::baseline`]): dense hidden layers,
/// int8 or binary, then one dense output layer. The baselines run on the computer only, so no
/// gateware reads this and it is not written out: a test exports it from the run's committed
/// weights as it scores it.
#[derive(Debug, PartialEq)]
pub struct Baseline {
    pub run: String,
    pub weights_sha256: String,
    /// As a [`Network`]'s: a pixel p enters as p >> pixel_shift.
    pub pixel_shift: u32,
    pub layers: Vec<HiddenLayer>,
    pub output: DenseLayer,
}

/// One hidden layer of a baseline, from uint8 inputs x to uint8 outputs y:
///
/// 1. s_o = sum over i of weight[o][i] x_i, an int32;
/// 2. z_o = s_o * h_o + bias_o, an int32, or s_o + bias_o for a layer with no h;
/// 3. y_o = z_o >> output_shift, clamped to 0..255.
///
/// E2's weights are int8 on one grid, 2^weight_exponent, and it has no h. E3's are +1 or -1, with a
/// weight exponent of 0, and one int8 scale h per output, on one grid 2^h_exponent. The exponents
/// add up, input_exponent + weight_exponent + h_exponent + output_shift = output_exponent, which
/// [`Baseline::check`] checks.
#[derive(Debug, PartialEq)]
pub struct HiddenLayer {
    pub inputs: usize,
    pub outputs: usize,
    pub input_exponent: i32,
    /// outputs x inputs.
    pub weight: Vec<Vec<i8>>,
    pub weight_exponent: i32,
    pub h: Option<Scales>,
    pub bias: Vec<i32>,
    pub output_shift: u32,
    pub output_exponent: i32,
}

/// One int8 scale per output, each standing for h * 2^exponent.
#[derive(Debug, PartialEq)]
pub struct Scales {
    pub h: Vec<i8>,
    pub exponent: i32,
}

/// One row of a sign matrix, each entry +1 or -1, written as a string of `+` and `-`.
#[derive(Debug, Clone, PartialEq)]
pub struct Signs(pub Vec<i8>);

impl Serialize for Signs {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(
            &self
                .0
                .iter()
                .map(|&x| if x < 0 { '-' } else { '+' })
                .collect::<String>(),
        )
    }
}

impl<'de> Deserialize<'de> for Signs {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let text = String::deserialize(d)?;
        text.chars()
            .map(|c| match c {
                '+' => Ok(1),
                '-' => Ok(-1),
                other => Err(serde::de::Error::custom(format!(
                    "a sign is `+` or `-`, not {other:?}"
                ))),
            })
            .collect::<Result<_, _>>()
            .map(Signs)
    }
}

/// Every vector a LittleBit layer computes for one input, by step (docs/littlebit.md, "The golden
/// model"), and how many latents were saturated to fit an int16.
#[derive(Debug, PartialEq)]
pub struct Trace {
    pub p: Vec<i16>,
    pub a: Vec<i32>,
    pub t: Vec<i16>,
    pub s: Vec<i32>,
    pub z: Vec<i32>,
    pub y: Vec<u8>,
    pub saturated: usize,
}

/// Steps 1 and 2 of a LittleBit layer: p_i = x_i * g_i, and a_k = sum over i of V[i][k] p_i.
/// The exporter uses them to pick a layer's latent shift, so it measures what the layer computes.
pub fn v_stage(
    x: &[u8],
    g: &[i8],
    v: &[Signs],
    rank: usize,
) -> Result<(Vec<i16>, Vec<i32>), String> {
    if x.len() != g.len() || v.len() != g.len() {
        return Err(format!("{} inputs for a layer of {}", x.len(), g.len()));
    }
    let overflow = || "a V sum overflows an int32".to_string();
    // A uint8 times an int8 is at most 255 * 128 in size, inside an int16.
    let p: Vec<i16> = x
        .iter()
        .zip(g)
        .map(|(&x, &g)| i16::from(x) * i16::from(g))
        .collect();
    let mut a = vec![0i32; rank];
    for (row, &p) in v.iter().zip(&p) {
        for (a, &sign) in a.iter_mut().zip(&row.0) {
            let p = i32::from(p);
            *a = if sign < 0 {
                a.checked_sub(p)
            } else {
                a.checked_add(p)
            }
            .ok_or_else(overflow)?;
        }
    }
    Ok((p, a))
}

impl LittleBitLayer {
    /// The layer's vectors for input `x`, step by step.
    pub fn trace(&self, x: &[u8]) -> Result<Trace, String> {
        let (p, a) = v_stage(x, &self.g, &self.v, self.rank)?;
        let mut saturated = 0;
        let t: Vec<i16> = a
            .iter()
            .zip(&self.l)
            .map(|(&a, &l)| {
                // An int32 times an int8 fits an int64 with room to spare.
                let wide = (i64::from(a) * i64::from(l)) >> self.latent_shift;
                i16::try_from(wide).unwrap_or_else(|_| {
                    saturated += 1;
                    if wide < 0 { i16::MIN } else { i16::MAX }
                })
            })
            .collect();
        let overflow = |what: &str| format!("{what} overflows an int32");
        let s: Vec<i32> = self
            .u
            .iter()
            .map(|row| {
                row.0.iter().zip(&t).try_fold(0i32, |s, (&sign, &t)| {
                    let t = i32::from(t);
                    if sign < 0 {
                        s.checked_sub(t)
                    } else {
                        s.checked_add(t)
                    }
                    .ok_or_else(|| overflow("a U sum"))
                })
            })
            .collect::<Result<_, _>>()?;
        let z: Vec<i32> = s
            .iter()
            .zip(&self.h)
            .zip(&self.bias)
            .map(|((&s, &h), &b)| {
                let wide = i64::from(s) * i64::from(h) + i64::from(b);
                i32::try_from(wide).map_err(|_| overflow("s * h + bias"))
            })
            .collect::<Result<_, _>>()?;
        let y = z
            .iter()
            .map(|&z| (z >> self.output_shift).clamp(0, 255) as u8)
            .collect();
        Ok(Trace {
            p,
            a,
            t,
            s,
            z,
            y,
            saturated,
        })
    }
}

/// The vectors a convolution applies its layer to, one for each position of a grid `height` x
/// `width` of `channels`-deep uint8 values, stored row by row and a position's channels together:
/// the `kernel` x `kernel` patch centred there, flattened row by row and then by channel, with 0
/// wherever the patch runs past the grid's edge. That is Flax's default, a stride of 1 and "same"
/// padding (docs/littlebit.md, "After MNIST"), for an odd kernel. The windows come in the grid's
/// order, so a layer's output for window j is the output grid's position j.
pub fn windows(
    grid: &[u8],
    height: usize,
    width: usize,
    channels: usize,
    kernel: usize,
) -> Result<Vec<Vec<u8>>, String> {
    if grid.len() != height * width * channels {
        return Err(format!(
            "a grid of {} values is not {height} x {width} x {channels}",
            grid.len()
        ));
    }
    if kernel.is_multiple_of(2) {
        return Err(format!("a kernel of {kernel} has no centre"));
    }
    let half = kernel / 2;
    let mut out = Vec::with_capacity(height * width);
    for row in 0..height {
        for col in 0..width {
            let mut window = Vec::with_capacity(kernel * kernel * channels);
            for dr in 0..kernel {
                for dc in 0..kernel {
                    // The patch's row and column in the grid, if it is in the grid.
                    let r = (row + dr).checked_sub(half).filter(|&r| r < height);
                    let c = (col + dc).checked_sub(half).filter(|&c| c < width);
                    for ch in 0..channels {
                        window.push(match (r, c) {
                            (Some(r), Some(c)) => grid[(r * width + c) * channels + ch],
                            _ => 0,
                        });
                    }
                }
            }
            out.push(window);
        }
    }
    Ok(out)
}

impl DenseLayer {
    pub fn scores(&self, x: &[u8]) -> Result<Vec<i32>, String> {
        if x.len() != self.inputs {
            return Err(format!("{} inputs for a layer of {}", x.len(), self.inputs));
        }
        self.weight
            .iter()
            .zip(&self.bias)
            .map(|(row, &b)| {
                row.iter()
                    .zip(x)
                    .try_fold(b, |s, (&w, &x)| s.checked_add(i32::from(w) * i32::from(x)))
                    .ok_or_else(|| "a score overflows an int32".to_string())
            })
            .collect()
    }

    /// What is wrong with the output layer's shape, if anything, after layers whose outputs are
    /// `inputs` values on the grid 2^`exponent`.
    fn check_after(&self, inputs: usize, exponent: i32) -> Result<(), String> {
        if (self.inputs, self.input_exponent) != (inputs, exponent) {
            return Err("the output layer's inputs are not the outputs before it".into());
        }
        if self.weight.len() != self.outputs
            || self.bias.len() != self.outputs
            || self.weight.iter().any(|row| row.len() != self.inputs)
        {
            return Err("the output layer's weight or bias is not its shape".into());
        }
        Ok(())
    }
}

impl HiddenLayer {
    /// Steps 1 and 2 for input `x`: each output's z.
    pub fn z(&self, x: &[u8]) -> Result<Vec<i32>, String> {
        if x.len() != self.inputs {
            return Err(format!("{} inputs for a layer of {}", x.len(), self.inputs));
        }
        let overflow = |what: &str| format!("{what} overflows an int32");
        self.weight
            .iter()
            .zip(&self.bias)
            .enumerate()
            .map(|(o, (row, &b))| {
                // An int8 times a uint8 fits an int32 with room to spare; the sum may not.
                let s = row
                    .iter()
                    .zip(x)
                    .try_fold(0i32, |s, (&w, &x)| {
                        s.checked_add(i32::from(w) * i32::from(x))
                    })
                    .ok_or_else(|| overflow("a weighted sum"))?;
                let h = self.h.as_ref().map_or(1, |h| i64::from(h.h[o]));
                i32::try_from(i64::from(s) * h + i64::from(b)).map_err(|_| overflow("s * h + bias"))
            })
            .collect()
    }

    /// The layer's outputs for input `x`: step 3 on each z.
    pub fn y(&self, x: &[u8]) -> Result<Vec<u8>, String> {
        Ok(self
            .z(x)?
            .iter()
            .map(|&z| (z >> self.output_shift).clamp(0, 255) as u8)
            .collect())
    }
}

impl Baseline {
    /// What is wrong with the network's shapes or exponents, if anything: as a [`Network`]'s
    /// are checked, and a layer with an h has only +1 and -1 for weights, on a grid of 2^0.
    pub fn check(&self) -> Result<(), String> {
        if self.pixel_shift > 7 {
            return Err(format!(
                "a pixel shift of {} leaves no pixel",
                self.pixel_shift
            ));
        }
        let mut inputs = 784;
        let mut exponent = self.pixel_shift as i32 - 8;
        for (j, l) in self.layers.iter().enumerate() {
            let bad = |what: &str| Err(format!("layer {j}: {what}"));
            if (l.inputs, l.input_exponent) != (inputs, exponent) {
                return bad("its inputs are not the outputs before it");
            }
            if l.weight.len() != l.outputs
                || l.bias.len() != l.outputs
                || l.weight.iter().any(|row| row.len() != l.inputs)
            {
                return bad("its weight or bias is not its shape");
            }
            let h_exponent = match &l.h {
                None => 0,
                Some(h) => {
                    if h.h.len() != l.outputs {
                        return bad("h is not as long as its outputs");
                    }
                    if l.weight_exponent != 0 || l.weight.iter().flatten().any(|w| w.abs() != 1) {
                        return bad("it has an h, but its weights are not +1 and -1 on 2^0");
                    }
                    h.exponent
                }
            };
            if l.output_shift > 31 {
                return bad("a shift is wider than the value it shifts");
            }
            let sum = l.input_exponent + l.weight_exponent + h_exponent + l.output_shift as i32;
            if sum != l.output_exponent {
                return bad(&format!(
                    "its exponents add up to {sum}, not its output exponent {}",
                    l.output_exponent
                ));
            }
            (inputs, exponent) = (l.outputs, l.output_exponent);
        }
        self.output.check_after(inputs, exponent)
    }

    /// The scores for one image.
    pub fn scores(&self, pixels: &[u8]) -> Result<Vec<i32>, String> {
        let mut x: Vec<u8> = pixels.iter().map(|&p| p >> self.pixel_shift).collect();
        for layer in &self.layers {
            x = layer.y(&x)?;
        }
        self.output.scores(&x)
    }

    /// The digit each image of `set` scores highest, [`argmax`]. A baseline has no latents, so
    /// none is saturated.
    pub fn predict(&self, set: &Images) -> Result<Predictions, String> {
        let digits = (0..set.len())
            .map(|k| self.scores(set.image(k)).map(|s| argmax(&s)))
            .collect::<Result<_, _>>()?;
        Ok(Predictions {
            digits,
            saturated: 0,
        })
    }
}

/// The index of the highest score, the first if two tie, as the FPGA's argmax picks it.
pub fn argmax(scores: &[i32]) -> usize {
    (0..scores.len()).fold(0, |m, i| if scores[i] > scores[m] { i } else { m })
}

/// What a network makes of a set of images: each image's predicted digit, and how many latents,
/// over all of them, were saturated.
pub struct Predictions {
    pub digits: Vec<usize>,
    pub saturated: usize,
}

impl Predictions {
    pub fn correct(&self, set: &Images) -> usize {
        self.digits
            .iter()
            .zip(&set.labels)
            .filter(|&(&d, &l)| d == usize::from(l))
            .count()
    }
}

impl Network {
    /// `integer.json`, refused unless its shapes fit together and its exponents add up.
    pub fn parse(text: &str) -> Result<Self, String> {
        let net: Network = serde_json::from_str(text).map_err(|e| format!("integer.json: {e}"))?;
        net.check()?;
        Ok(net)
    }

    /// What is wrong with the network's shapes or exponents, if anything.
    pub fn check(&self) -> Result<(), String> {
        if self.pixel_shift > 7 {
            return Err(format!(
                "a pixel shift of {} leaves no pixel",
                self.pixel_shift
            ));
        }
        let mut inputs = 784;
        let mut exponent = self.pixel_shift as i32 - 8;
        for (j, l) in self.layers.iter().enumerate() {
            let bad = |what: &str| Err(format!("layer {j}: {what}"));
            if (l.inputs, l.input_exponent) != (inputs, exponent) {
                return bad("its inputs are not the outputs before it");
            }
            if l.g.len() != l.inputs || l.v.len() != l.inputs || l.l.len() != l.rank {
                return bad("g, V or l is not as long as its inputs or rank");
            }
            if l.u.len() != l.outputs || l.h.len() != l.outputs || l.bias.len() != l.outputs {
                return bad("U, h or its bias is not as long as its outputs");
            }
            if l.v.iter().chain(&l.u).any(|row| row.0.len() != l.rank) {
                return bad("a row of V or U is not as long as its rank");
            }
            if l.latent_shift > 62 || l.output_shift > 31 {
                return bad("a shift is wider than the value it shifts");
            }
            let sum = l.input_exponent
                + l.g_exponent
                + l.l_exponent
                + l.latent_shift as i32
                + l.h_exponent
                + l.output_shift as i32;
            if sum != l.output_exponent {
                return bad(&format!(
                    "its exponents add up to {sum}, not its output exponent {}",
                    l.output_exponent
                ));
            }
            (inputs, exponent) = (l.outputs, l.output_exponent);
        }
        self.output.check_after(inputs, exponent)
    }

    /// The scores for one image, and how many latents were saturated on the way.
    pub fn scores(&self, pixels: &[u8]) -> Result<(Vec<i32>, usize), String> {
        let mut x: Vec<u8> = pixels.iter().map(|&p| p >> self.pixel_shift).collect();
        let mut saturated = 0;
        for layer in &self.layers {
            let trace = layer.trace(&x)?;
            saturated += trace.saturated;
            x = trace.y;
        }
        Ok((self.output.scores(&x)?, saturated))
    }

    /// The digit each image of `set` scores highest, [`argmax`].
    pub fn predict(&self, set: &Images) -> Result<Predictions, String> {
        let mut digits = Vec::with_capacity(set.len());
        let mut saturated = 0;
        for k in 0..set.len() {
            let (scores, n) = self.scores(set.image(k))?;
            saturated += n;
            digits.push(argmax(&scores));
        }
        Ok(Predictions { digits, saturated })
    }
}
