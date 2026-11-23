//! The computer's side of LittleBit on the board: one layer (`gateware/bringup/littlebit_layer.py`)
//! and the whole network (`gateware/littlebit/network.py`). For each, the frame it sends for an
//! input, and the answer it reads back, checked against the golden model
//! ([`crate::integer::LittleBitLayer::trace`], [`crate::integer::Network::scores`]).
//!
//! A frame is 0xA5, then the inputs, a byte each: for the layer, its inputs; for the network, an
//! image's 784 pixels as MNIST stores them, which the network shifts itself.
//!
//! The layer's answer is 0x5A, then for each output, in order, y, a byte, and z, the int32 before
//! the shift and clamp, four bytes, least significant first. The network's is 0x5A, then each
//! score, an int32, least significant byte first; the digit, a byte; and the clocks the network
//! took, a uint32, least significant byte first. `cargo run -p tentzhen-littlebit -- layer` and
//! `... -- network` send them over a serial port.

use crate::integer::{Trace, argmax};

/// The byte a frame starts with.
pub const FRAME: u8 = 0xA5;
/// The byte an answer starts with.
pub const ANSWER: u8 = 0x5A;

/// The frame for one input vector.
pub fn frame(x: &[u8]) -> Vec<u8> {
    let mut f = Vec::with_capacity(1 + x.len());
    f.push(FRAME);
    f.extend_from_slice(x);
    f
}

/// What is wrong with an answer's length or header, if anything.
fn check(answer: &[u8], len: usize) -> Result<(), String> {
    if answer.len() != len {
        return Err(format!("an answer of {} bytes, not {len}", answer.len()));
    }
    if answer[0] != ANSWER {
        return Err(format!(
            "an answer starting {:#04x}, not {ANSWER:#04x}",
            answer[0]
        ));
    }
    Ok(())
}

/// How many bytes the answer for a layer of `outputs` outputs takes.
pub fn answer_len(outputs: usize) -> usize {
    1 + 5 * outputs
}

/// Each output's y and z, from an answer for a layer of `outputs` outputs.
pub fn parse_answer(answer: &[u8], outputs: usize) -> Result<Vec<(u8, i32)>, String> {
    check(answer, answer_len(outputs))?;
    Ok(answer[1..]
        .chunks_exact(5)
        .map(|c| (c[0], i32::from_le_bytes([c[1], c[2], c[3], c[4]])))
        .collect())
}

/// How the board's outputs differ from the golden model's, if they do: the first output whose y
/// or z is not the golden model's, and how many are not.
pub fn difference(board: &[(u8, i32)], golden: &Trace) -> Option<String> {
    if board.len() != golden.y.len() {
        return Some(format!("{} outputs, not {}", board.len(), golden.y.len()));
    }
    let golden: Vec<(u8, i32)> = golden
        .y
        .iter()
        .copied()
        .zip(golden.z.iter().copied())
        .collect();
    let differ: Vec<usize> = (0..board.len())
        .filter(|&o| board[o] != golden[o])
        .collect();
    let &o = differ.first()?;
    Some(format!(
        "{} of {} outputs differ; the first, output {o}: y {}, z {}, where the golden model's are \
         y {}, z {}",
        differ.len(),
        board.len(),
        board[o].0,
        board[o].1,
        golden[o].0,
        golden[o].1
    ))
}

/// What the whole network on the board answers for one image.
#[derive(Debug, PartialEq)]
pub struct Scored {
    pub scores: Vec<i32>,
    pub digit: usize,
    /// The clocks the network took, from the one that took the image's first pixel to the one
    /// that raised its answer, both counted, as the network counts them.
    pub clocks: u32,
}

/// How many bytes the network's answer takes, for a network of `outputs` scores.
pub fn scored_len(outputs: usize) -> usize {
    1 + 4 * outputs + 1 + 4
}

/// The scores, digit and clocks, from the network's answer for a network of `outputs` scores.
pub fn parse_scored(answer: &[u8], outputs: usize) -> Result<Scored, String> {
    check(answer, scored_len(outputs))?;
    let int = |at: usize| [answer[at], answer[at + 1], answer[at + 2], answer[at + 3]];
    let digit = 1 + 4 * outputs;
    Ok(Scored {
        scores: (0..outputs)
            .map(|o| i32::from_le_bytes(int(1 + 4 * o)))
            .collect(),
        digit: usize::from(answer[digit]),
        clocks: u32::from_le_bytes(int(digit + 1)),
    })
}

/// How the board's scores and digit differ from the golden model's scores, `golden`, and the
/// digit it picks from them, [`argmax`], if they do: the first score that is not the golden
/// model's, and how many are not, or the digit.
pub fn scored_difference(board: &Scored, golden: &[i32]) -> Option<String> {
    if board.scores.len() != golden.len() {
        return Some(format!(
            "{} scores, not {}",
            board.scores.len(),
            golden.len()
        ));
    }
    let differ: Vec<usize> = (0..golden.len())
        .filter(|&o| board.scores[o] != golden[o])
        .collect();
    let digit = argmax(golden);
    match differ.first() {
        Some(&o) => Some(format!(
            "{} of {} scores differ; the first, score {o}: {}, where the golden model's is {}; \
             digit {}, the golden model's {digit}",
            differ.len(),
            golden.len(),
            board.scores[o],
            golden[o],
            board.digit
        )),
        None if board.digit != digit => Some(format!(
            "every score is the golden model's, but the digit is {}, not {digit}",
            board.digit
        )),
        None => None,
    }
}
