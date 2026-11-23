//! Project #3's golden model (docs/littlebit.md, "Verification"): MNIST read only as its pins
//! allow ([`mnist`]); a training run's committed weights, dense or LittleBit, re-scored here in f32
//! ([`run`]); the exporter, which makes a LittleBit run an integer network ([`export`]); the integer
//! golden model that runs it ([`integer`]); the test vectors it writes for the gateware
//! ([`vectors`]); the computer's side of a layer on the board, checked against it ([`board`]); and
//! Gate 4's two plots, drawn from the experiments table ([`plots`]).

pub mod board;
pub mod export;
pub mod integer;
pub mod mnist;
pub mod plots;
pub mod run;
pub mod vectors;

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::{Path, PathBuf};

    fn root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn e1() -> PathBuf {
        root().join("models/littlebit/runs/e1")
    }

    fn r16() -> PathBuf {
        root().join("models/littlebit/runs/r16")
    }

    fn e2() -> PathBuf {
        root().join("models/littlebit/runs/e2")
    }

    fn e3() -> PathBuf {
        root().join("models/littlebit/runs/e3")
    }

    /// The folder MNIST was fetched into, from MNIST_DIR. A test that needs MNIST fails without
    /// it rather than passing unchecked; the Quality Gate fetches MNIST and sets it.
    fn mnist_dir() -> PathBuf {
        PathBuf::from(std::env::var_os("MNIST_DIR").expect(
            "MNIST_DIR names the folder MNIST is fetched into: in models/littlebit, \
             `uv run --locked python mnist.py <folder>`",
        ))
    }

    /// A fresh folder for one test, under the system's temp folder.
    fn scratch(test: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("tentzhen-littlebit-{}-{test}", std::process::id()));
        if dir.exists() {
            fs::remove_dir_all(&dir).unwrap();
        }
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn the_mnist_pins_name_its_four_files() {
        let pins = mnist::Pins::load(&root()).unwrap();
        assert_eq!(pins.sha256.len(), 4);
        let text = fs::read_to_string(root().join(mnist::PINS)).unwrap();
        let extra = text.replace("[sha256]", "extra = 1\n\n[sha256]");
        assert!(
            mnist::Pins::parse(&extra)
                .unwrap_err()
                .contains("unknown field")
        );
        let renamed = text.replace("\"t10k-labels-idx1-ubyte.gz\"", "\"t10k-labels.gz\"");
        assert!(mnist::Pins::parse(&renamed).unwrap_err().contains("not"));
    }

    #[test]
    fn a_run_summary_refuses_a_key_it_does_not_know() {
        let text = fs::read_to_string(e1().join("summary.json")).unwrap();
        run::Summary::parse(&text).unwrap();
        let top = text.replacen('{', "{\"extra\": 1,", 1);
        assert!(
            run::Summary::parse(&top)
                .unwrap_err()
                .contains("unknown field")
        );
        let nested = text.replacen("\"optimizer\"", "\"extra\": 1, \"optimizer\"", 1);
        assert!(
            run::Summary::parse(&nested)
                .unwrap_err()
                .contains("unknown field")
        );
        let text = fs::read_to_string(r16().join("summary.json")).unwrap();
        assert!(run::Summary::parse(&text).unwrap().littlebit.is_some());
        for (key, with_extra) in [
            ("\"rank\"", "\"extra\": 1, \"rank\""),
            (
                "\"weights_sha256\": \"92",
                "\"extra\": 1, \"weights_sha256\": \"92",
            ),
        ] {
            let nested = text.replacen(key, with_extra, 1);
            assert_ne!(nested, text);
            assert!(
                run::Summary::parse(&nested)
                    .unwrap_err()
                    .contains("unknown field")
            );
        }
        let text = fs::read_to_string(e3().join("summary.json")).unwrap();
        assert!(run::Summary::parse(&text).unwrap().quantized.is_some());
        let nested = text.replacen("\"sign\"", "\"extra\": 1, \"sign\"", 1);
        assert_ne!(nested, text);
        assert!(
            run::Summary::parse(&nested)
                .unwrap_err()
                .contains("unknown field")
        );
    }

    /// A run is scored only by arithmetic this crate knows: a summary naming another pixel
    /// mapping, or activation grids for other than the gaps between its layers, is refused.
    #[test]
    fn a_run_is_scored_only_as_its_summary_says() {
        let run = run::Run::load(&r16()).unwrap();
        let text = fs::read_to_string(r16().join("summary.json")).unwrap();
        let one = mnist::Images {
            pixels: vec![0; 784],
            labels: vec![0],
        };
        let refused = |text: String| {
            let changed = run::Run {
                summary: run::Summary::parse(&text).unwrap(),
                tensors: run.tensors.clone(),
            };
            changed.correct(&[0, 2, 4], &one).unwrap_err()
        };
        assert!(run.correct(&[0, 2, 4], &one).is_ok());
        let pixels = text.replace("floor(p / 2) / 128", "p / 255");
        assert!(refused(pixels).contains("no mapping this crate knows"));
        let grids = text.replace("-5,\n      -4\n", "-5\n");
        assert!(refused(grids).contains("activation exponents"));
        let e1 = run::Run::load(&e1()).unwrap();
        assert!(e1.correct(&[0, 2], &one).is_ok());
        assert!(run.correct(&[0, 2], &one).unwrap_err().contains("gaps"));
        // A baseline's weights are scored only as the kind its summary names.
        let binary = run::Run::load(&e3()).unwrap();
        assert!(binary.correct(&[0, 2, 4], &one).is_ok());
        let text = fs::read_to_string(e3().join("summary.json")).unwrap();
        let kind = |k: &str| {
            let changed = text.replace("\"weights\": \"binary\"", &format!("\"weights\": \"{k}\""));
            assert_ne!(changed, text);
            run::Run {
                summary: run::Summary::parse(&changed).unwrap(),
                tensors: binary.tensors.clone(),
            }
            .correct(&[0, 2, 4], &one)
            .unwrap_err()
        };
        assert!(kind("int8").contains("not a binary layer"));
        assert!(kind("ternary").contains("no kind this crate knows"));
    }

    /// E1's committed weights are the ones its summary names, in the shapes of the 784 -> 256 ->
    /// 128 -> 10 MLP docs/littlebit.md gives (234,752 weights), trained on the MNIST the pins
    /// name. A weights file changed by one bit is refused.
    #[test]
    fn e1_weights_are_the_ones_its_summary_names() {
        let run = run::Run::load(&e1()).unwrap();
        let s = &run.summary;
        let shapes: Vec<(&str, &[usize])> = s
            .layout
            .iter()
            .map(|t| (t.name.as_str(), t.shape.as_slice()))
            .collect();
        let want: [(&str, &[usize]); 6] = [
            ("0.weight", &[256, 784]),
            ("0.bias", &[256]),
            ("2.weight", &[128, 256]),
            ("2.bias", &[128]),
            ("4.weight", &[10, 128]),
            ("4.bias", &[10]),
        ];
        assert_eq!(shapes, want);
        assert_eq!((s.weights, s.parameters), (234_752, 235_146));
        assert_eq!(s.mnist_sha256, mnist::Pins::load(&root()).unwrap().sha256);

        let dir = scratch("e1-weights");
        fs::copy(e1().join("summary.json"), dir.join("summary.json")).unwrap();
        let mut bytes = fs::read(e1().join("weights.f32")).unwrap();
        bytes[1000] ^= 1;
        fs::write(dir.join("weights.f32"), bytes).unwrap();
        let refused = run::Run::load(&dir).err().unwrap();
        fs::remove_dir_all(&dir).unwrap();
        assert!(refused.contains("its sha256 is"), "{refused}");
    }

    /// Each MNIST file is checked against its pin: one changed by a bit is refused.
    #[test]
    fn mnist_refuses_a_changed_file() {
        let dir = scratch("mnist");
        for f in mnist::FILES {
            fs::copy(mnist_dir().join(f), dir.join(f)).unwrap();
        }
        mnist::load(&root(), &dir).unwrap();
        let path = dir.join(mnist::FILES[3]);
        let mut bytes = fs::read(&path).unwrap();
        let n = bytes.len();
        bytes[n - 9] ^= 1;
        fs::write(&path, bytes).unwrap();
        let refused = mnist::load(&root(), &dir).err().unwrap();
        fs::remove_dir_all(&dir).unwrap();
        assert!(refused.contains("its sha256 is"), "{refused}");
    }

    /// E1, re-scored here from its committed weights, gets right the images its summary says:
    /// 9,831 of the 10,000 test images (98.31 %, docs/littlebit.md) and all 60,000 training
    /// images. The summary's counts came from PyTorch on a GPU; these come from this crate's own
    /// f32 arithmetic on the CPU.
    #[test]
    fn e1_scores_what_its_summary_says() {
        let run = run::Run::load(&e1()).unwrap();
        let (train, test) = mnist::load(&root(), &mnist_dir()).unwrap();
        let linears = [0, 2, 4];
        assert_eq!(
            run.correct(&linears, &test).unwrap(),
            run.summary.test_correct
        );
        assert_eq!(run.summary.test_correct, 9_831);
        assert_eq!(
            run.correct(&linears, &train).unwrap(),
            run.summary.train_correct
        );
        assert_eq!(run.summary.train_correct, 60_000);
    }

    /// r16's committed weights are the ones its summary names, in the shapes of the LittleBit MLP
    /// at r = 16 (docs/littlebit.md, "Model"): layers 0 and 2 as g, V (in x r), l, U (out x r), h
    /// and a bias, and layer 4 dense. It started from E1's committed weights, trained on the
    /// MNIST the pins name.
    #[test]
    fn r16_weights_are_the_ones_its_summary_names() {
        let run = run::Run::load(&r16()).unwrap();
        let s = &run.summary;
        let shapes: Vec<(&str, &[usize])> = s
            .layout
            .iter()
            .map(|t| (t.name.as_str(), t.shape.as_slice()))
            .collect();
        let want: [(&str, &[usize]); 14] = [
            ("0.g", &[784]),
            ("0.v", &[784, 16]),
            ("0.l", &[16]),
            ("0.u", &[256, 16]),
            ("0.h", &[256]),
            ("0.bias", &[256]),
            ("2.g", &[256]),
            ("2.v", &[256, 16]),
            ("2.l", &[16]),
            ("2.u", &[128, 16]),
            ("2.h", &[128]),
            ("2.bias", &[128]),
            ("4.weight", &[10, 128]),
            ("4.bias", &[10]),
        ];
        assert_eq!(shapes, want);
        assert_eq!((s.weights, s.parameters), (234_752, 25_914));
        let lb = s.littlebit.as_ref().unwrap();
        assert_eq!(lb.rank, 16);
        let e1 = run::Run::load(&e1()).unwrap();
        assert_eq!(lb.start.run, e1.summary.run);
        assert_eq!(lb.start.weights_sha256, e1.summary.weights_sha256);
        assert_eq!(s.mnist_sha256, mnist::Pins::load(&root()).unwrap().sha256);
    }

    /// r16, the LittleBit MLP at r = 16, re-scored here from its committed weights, gets right
    /// the images its summary says: 9,713 of the 10,000 test images (97.13 %) and 58,798 of the
    /// 60,000 training images. The summary's counts came from PyTorch on a GPU; these come from
    /// this crate's own f32 arithmetic on the CPU, with the factors' signs and the activations'
    /// uint8 grid as `models/littlebit/littlebit.py` has them.
    #[test]
    fn r16_scores_what_its_summary_says() {
        let run = run::Run::load(&r16()).unwrap();
        let (train, test) = mnist::load(&root(), &mnist_dir()).unwrap();
        let layers = [0, 2, 4];
        assert_eq!(
            run.correct(&layers, &test).unwrap(),
            run.summary.test_correct
        );
        assert_eq!(run.summary.test_correct, 9_713);
        assert_eq!(
            run.correct(&layers, &train).unwrap(),
            run.summary.train_correct
        );
        assert_eq!(run.summary.train_correct, 58_798);
    }

    /// E2's and E3's committed weights are the ones their summaries name, in the shapes of E1's
    /// MLP: E2's layers 0, 2 and 4 dense, as E1's are; E3's layers 0 and 2 binary, a latent
    /// weight (out x in), one h per output and a bias, and layer 4 dense. Each started from E1's
    /// committed weights, trained on the MNIST the pins name.
    #[test]
    fn e2_and_e3_weights_are_the_ones_their_summaries_name() {
        let e1 = run::Run::load(&e1()).unwrap();
        let dense: [(&str, &[usize]); 6] = [
            ("0.weight", &[256, 784]),
            ("0.bias", &[256]),
            ("2.weight", &[128, 256]),
            ("2.bias", &[128]),
            ("4.weight", &[10, 128]),
            ("4.bias", &[10]),
        ];
        let binary: [(&str, &[usize]); 8] = [
            ("0.weight", &[256, 784]),
            ("0.h", &[256]),
            ("0.bias", &[256]),
            ("2.weight", &[128, 256]),
            ("2.h", &[128]),
            ("2.bias", &[128]),
            ("4.weight", &[10, 128]),
            ("4.bias", &[10]),
        ];
        for (folder, name, kind, want, parameters) in [
            (e2(), "E2", "int8", &dense[..], 235_146),
            (e3(), "E3", "binary", &binary[..], 235_530),
        ] {
            let run = run::Run::load(&folder).unwrap();
            let s = &run.summary;
            let shapes: Vec<(&str, &[usize])> = s
                .layout
                .iter()
                .map(|t| (t.name.as_str(), t.shape.as_slice()))
                .collect();
            assert_eq!(shapes, want, "{name}");
            assert_eq!(
                (s.run.as_str(), s.weights, s.parameters),
                (name, 234_752, parameters)
            );
            assert!(s.littlebit.is_none(), "{name}");
            let q = s.quantized.as_ref().unwrap();
            assert_eq!(q.weights, kind);
            assert_eq!(q.sign.is_some(), kind == "binary", "{name}");
            assert_eq!(q.start.run, e1.summary.run);
            assert_eq!(q.start.weights_sha256, e1.summary.weights_sha256);
            assert_eq!(s.mnist_sha256, mnist::Pins::load(&root()).unwrap().sha256);
        }
    }

    /// E2, E1's MLP fine-tuned on the hardware's pixels and uint8 activations, its weights float,
    /// re-scored here from its committed weights, gets right the images its summary says: 9,835
    /// of the 10,000 test images (98.35 %) and all 60,000 training images. The summary's counts
    /// came from PyTorch on a GPU; these come from this crate's own f32 arithmetic on the CPU.
    #[test]
    fn e2_scores_what_its_summary_says() {
        let run = run::Run::load(&e2()).unwrap();
        let (train, test) = mnist::load(&root(), &mnist_dir()).unwrap();
        let layers = [0, 2, 4];
        let s = &run.summary;
        assert_eq!(run.correct(&layers, &test).unwrap(), s.test_correct);
        assert_eq!(run.correct(&layers, &train).unwrap(), s.train_correct);
        assert_eq!((s.test_correct, s.train_correct), (9_835, 60_000));
    }

    /// E3, binary hidden layers with one scale per output, re-scored here from its committed
    /// weights, gets right the images its summary says: 9,824 of the 10,000 test images (98.24 %)
    /// and all 60,000 training images, with the latent weights' signs as
    /// `models/littlebit/littlebit.py` has them.
    #[test]
    fn e3_scores_what_its_summary_says() {
        let run = run::Run::load(&e3()).unwrap();
        let (train, test) = mnist::load(&root(), &mnist_dir()).unwrap();
        let layers = [0, 2, 4];
        let s = &run.summary;
        assert_eq!(run.correct(&layers, &test).unwrap(), s.test_correct);
        assert_eq!(run.correct(&layers, &train).unwrap(), s.train_correct);
        assert_eq!((s.test_correct, s.train_correct), (9_824, 60_000));
    }

    /// Check 1 on the baselines: exported as integers, E2 gets 9,833 of the 10,000 test images
    /// right against its float run's 9,835, and E3 gets 9,824 against 9,824, each within 0.5
    /// points. The integer and float models pick the same digit on 9,992 and 9,991 of them.
    #[test]
    fn e2_and_e3_integer_are_within_half_a_point_of_float() {
        let (_, test) = mnist::load(&root(), &mnist_dir()).unwrap();
        for (folder, want, want_same) in
            [(e2(), (9_833, 9_835), 9_992), (e3(), (9_824, 9_824), 9_991)]
        {
            let run = run::Run::load(&folder).unwrap();
            let net = export::baseline(&run, &[0, 2], 4).unwrap();
            assert_eq!(net.weights_sha256, run.summary.weights_sha256);
            let p = net.predict(&test).unwrap();
            let (integer, float) = (p.correct(&test), run.summary.test_correct);
            assert!(
                integer + 50 >= float,
                "{}: {integer} against {float}",
                run.summary.run
            );
            assert_eq!((integer, float), want, "{}", run.summary.run);
            let float_digits = run.predict(&[0, 2, 4], &test).unwrap();
            let same = float_digits
                .iter()
                .zip(&p.digits)
                .filter(|(f, i)| f == i)
                .count();
            assert_eq!(same, want_same, "{}", run.summary.run);
        }
    }

    /// Two small baseline layers, worked by hand: an int8 one, z = W x + b, and a binary one,
    /// z = h (S x) + b; each output z >> shift, clamped to 0..255.
    #[test]
    fn a_baseline_layer_computes_what_its_steps_say() {
        let int8 = integer::HiddenLayer {
            inputs: 3,
            outputs: 2,
            input_exponent: 0,
            weight: vec![vec![127, -128, 2], vec![-1, 0, 1]],
            weight_exponent: 0,
            h: None,
            bias: vec![-5, 40],
            output_shift: 3,
            output_exponent: 3,
        };
        // 127 * 10 - 128 * 2 + 2 * 255 - 5 = 1,519; -10 + 255 + 40 = 285.
        assert_eq!(int8.z(&[10, 2, 255]).unwrap(), [1_519, 285]);
        // 1,519 >> 3 = 189; 285 >> 3 = 35.
        assert_eq!(int8.y(&[10, 2, 255]).unwrap(), [189, 35]);
        // -128 * 10 - 5 = -1,285, >> 3 = -161, clamped to 0; 0 + 40 = 40, >> 3 = 5.
        assert_eq!(int8.z(&[0, 10, 0]).unwrap(), [-1_285, 40]);
        assert_eq!(int8.y(&[0, 10, 0]).unwrap(), [0, 5]);
        let binary = integer::HiddenLayer {
            inputs: 3,
            outputs: 2,
            input_exponent: 0,
            weight: vec![vec![1, -1, 1], vec![-1, -1, 1]],
            weight_exponent: 0,
            h: Some(integer::Scales {
                h: vec![3, -2],
                exponent: 0,
            }),
            bias: vec![-1, 7],
            output_shift: 1,
            output_exponent: 1,
        };
        // (255 - 1 + 4) * 3 - 1 = 773; (-255 - 1 + 4) * -2 + 7 = 511.
        assert_eq!(binary.z(&[255, 1, 4]).unwrap(), [773, 511]);
        // 773 >> 1 = 386, clamped to 255; 511 >> 1 = 255.
        assert_eq!(binary.y(&[255, 1, 4]).unwrap(), [255, 255]);
        // (0 - 3 + 0) * 3 - 1 = -10, >> 1 = -5, clamped to 0; (0 - 3 + 0) * -2 + 7 = 13, >> 1 = 6.
        assert_eq!(binary.z(&[0, 3, 0]).unwrap(), [-10, 13]);
        assert_eq!(binary.y(&[0, 3, 0]).unwrap(), [0, 6]);
        assert!(binary.z(&[1, 2]).is_err());
    }

    /// A baseline's integer network is refused unless its shapes fit together, its exponents add
    /// up, and a layer with an h has only +1 and -1 for weights.
    #[test]
    fn a_baseline_network_refuses_what_does_not_fit() {
        let run = run::Run::load(&e3()).unwrap();
        let fresh = || export::baseline(&run, &[0, 2], 4).unwrap();
        fresh().check().unwrap();
        let mut not_binary = fresh();
        not_binary.layers[0].weight[3][5] = 2;
        let mut shift = fresh();
        shift.layers[1].output_shift += 1;
        let mut short = fresh();
        short.layers[0].h.as_mut().unwrap().h.pop();
        let mut pixels = fresh();
        pixels.pixel_shift = 2;
        let mut output = fresh();
        output.output.bias.pop();
        for (bad, why) in [
            (not_binary, "not +1 and -1"),
            (shift, "add up"),
            (short, "not as long as its outputs"),
            (pixels, "not the outputs before it"),
            (output, "not its shape"),
        ] {
            let refused = bad.check().unwrap_err();
            assert!(refused.contains(why), "{refused}");
        }
        let int8 = export::baseline(&run::Run::load(&e2()).unwrap(), &[0, 2], 4).unwrap();
        int8.check().unwrap();
        assert!(int8.layers.iter().all(|l| l.h.is_none()));
        assert!(export::baseline(&run::Run::load(&r16()).unwrap(), &[0, 2], 4).is_err());
    }

    fn r16_integer() -> integer::Network {
        integer::Network::parse(&fs::read_to_string(r16().join(export::FILE)).unwrap()).unwrap()
    }

    /// One small LittleBit layer, worked by hand: each step is what `integer::LittleBitLayer`
    /// says, a right shift rounds toward minus infinity, a latent too large for an int16 is
    /// saturated and counted, and the outputs are clamped to 0..255.
    #[test]
    fn a_littlebit_layer_computes_what_its_steps_say() {
        use integer::Signs;
        let layer = integer::LittleBitLayer {
            inputs: 2,
            outputs: 3,
            rank: 2,
            input_exponent: 0,
            g: vec![3, -2],
            g_exponent: 0,
            v: vec![Signs(vec![1, -1]), Signs(vec![1, 1])],
            l: vec![5, 127],
            l_exponent: 0,
            latent_shift: 1,
            u: vec![Signs(vec![1, 1]), Signs(vec![-1, 1]), Signs(vec![1, -1])],
            h: vec![-2, 1, -1],
            h_exponent: 0,
            bias: vec![1, 0, 31_318],
            output_shift: 2,
            output_exponent: 3,
        };
        // p = (1 * 3, 255 * -2); a_0 = 3 - 510, a_1 = -3 - 510; t_0 = -2,535 >> 1 = -1,268, not
        // the -1,267 truncating would give; t_1 = -65,151 >> 1 = -32,576, which fits.
        let t = layer.trace(&[1, 255]).unwrap();
        assert_eq!(t.p, [3, -510]);
        assert_eq!(t.a, [-507, -513]);
        assert_eq!(t.t, [-1_268, -32_576]);
        assert_eq!(t.s, [-33_844, -31_308, 31_308]);
        assert_eq!(t.z, [67_689, -31_308, 10]);
        // 67,689 >> 2 = 16,922, clamped to 255; -7,827 clamped to 0; 10 >> 2 = 2.
        assert_eq!(t.y, [255, 0, 2]);
        assert_eq!(t.saturated, 0);
        // a_1 = -765 - 510; -1,275 * 127 >> 1 = -80,963, saturated to -32,768.
        let t = layer.trace(&[255, 255]).unwrap();
        assert_eq!(t.a, [255, -1_275]);
        assert_eq!(t.t, [637, -32_768]);
        assert_eq!(t.saturated, 1);
        assert_eq!(t.y, [255, 0, 0]);
    }

    /// A 2 x 3 grid of 2 channels, worked by hand: each window is the 3 x 3 patch around its
    /// position, row by row and then by channel, with zeros past the edge; and the windows come
    /// in the grid's order.
    #[test]
    fn windows_take_each_position_s_patch_with_zeros_past_the_edge() {
        // Position (row, column) holds channel 0 = 10 * row + column + 1, channel 1 = 100 + that.
        let grid: Vec<u8> = (0..2)
            .flat_map(|r| (0..3).flat_map(move |c| [10 * r + c + 1, 110 + 10 * r + c + 1]))
            .collect();
        let w = integer::windows(&grid, 2, 3, 2, 3).unwrap();
        assert_eq!(w.len(), 6);
        assert!(w.iter().all(|w| w.len() == 18));
        // Position (0, 0): the patch's top row and left column are past the edge.
        #[rustfmt::skip]
        assert_eq!(w[0], [
            0, 0,   0, 0,     0, 0,
            0, 0,   1, 111,   2, 112,
            0, 0,   11, 121,  12, 122,
        ]);
        // Position (1, 2): its bottom row and right column are.
        #[rustfmt::skip]
        assert_eq!(w[5], [
            2, 112,   3, 113,   0, 0,
            12, 122,  13, 123,  0, 0,
            0, 0,     0, 0,     0, 0,
        ]);
        assert!(integer::windows(&grid, 3, 3, 2, 3).is_err());
        assert!(integer::windows(&grid, 2, 3, 2, 2).is_err());
    }

    /// A frame is 0xA5 and the inputs; an answer is 0x5A and each output's y and z, z an int32
    /// least significant byte first. An answer of the wrong length or header is refused, and one
    /// that differs from the golden model says where.
    #[test]
    fn a_board_answer_is_read_as_its_frame_says() {
        assert_eq!(board::frame(&[1, 2, 255]), [0xA5, 1, 2, 255]);
        assert_eq!(board::answer_len(256), 1_281);
        let answer = [0x5A, 7, 0x10, 0x00, 0x00, 0x00, 0, 0xFE, 0xFF, 0xFF, 0xFF];
        let outputs = board::parse_answer(&answer, 2).unwrap();
        assert_eq!(outputs, [(7, 16), (0, -2)]);
        assert!(board::parse_answer(&answer[..10], 2).is_err());
        let mut wrong = answer;
        wrong[0] = 0x5B;
        assert!(board::parse_answer(&wrong, 2).is_err());
        let golden = integer::Trace {
            p: vec![],
            a: vec![],
            t: vec![],
            s: vec![],
            z: vec![16, -2],
            y: vec![7, 0],
            saturated: 0,
        };
        assert_eq!(board::difference(&outputs, &golden), None);
        let d = board::difference(&[(7, 16), (0, -3)], &golden).unwrap();
        assert!(
            d.contains("1 of 2 outputs differ") && d.contains("output 1"),
            "{d}"
        );
        assert!(board::difference(&outputs[..1], &golden).is_some());
    }

    /// The network's answer is 0x5A, each score an int32 least significant byte first, the
    /// digit, and the clocks, a uint32 least significant byte first: 46 bytes for 10 scores. An
    /// answer of the wrong length or header is refused, and one whose scores or digit differ from
    /// the golden model's says where.
    #[test]
    fn a_network_answer_is_read_as_its_frame_says() {
        assert_eq!(board::scored_len(10), 46);
        let answer = [
            0x5A, 0x10, 0x00, 0x00, 0x00, 0xFE, 0xFF, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0x80, 1, 0xC3,
            0x04, 0x00, 0x00,
        ];
        let scored = board::parse_scored(&answer, 3).unwrap();
        assert_eq!(
            scored,
            board::Scored {
                scores: vec![16, -2, i32::MIN],
                digit: 1,
                clocks: 1_219
            }
        );
        assert!(board::parse_scored(&answer[..17], 3).is_err());
        let mut wrong = answer;
        wrong[0] = 0x5B;
        assert!(board::parse_scored(&wrong, 3).is_err());
        // The golden model's digit is the first of the highest scores.
        assert_eq!(
            board::scored_difference(&scored, &[16, -2, i32::MIN]),
            Some("every score is the golden model's, but the digit is 1, not 0".into())
        );
        let tie = board::Scored {
            scores: vec![5, 9, 9],
            digit: 1,
            clocks: 1_219,
        };
        assert_eq!(board::scored_difference(&tie, &[5, 9, 9]), None);
        let d = board::scored_difference(&tie, &[5, 9, 8]).unwrap();
        assert!(
            d.contains("1 of 3 scores differ") && d.contains("score 2"),
            "{d}"
        );
        assert!(board::scored_difference(&tie, &[5, 9]).is_some());
    }

    /// The gateware's test vectors are what the golden model writes, and load as their schema.
    /// Each case exercises the engine's edges: some latent saturates, and some output clamps to
    /// 0, some to 255, and some falls between.
    #[test]
    fn littlebit_vectors_are_what_the_golden_model_writes() {
        let (r16, sweep) = vectors::networks(&root()).unwrap();
        let v = vectors::vectors(&r16, &sweep).unwrap();
        let committed = fs::read_to_string(root().join(vectors::FILE)).unwrap();
        assert!(
            committed == vectors::to_json(&v),
            "{} is stale: run `cargo run -p tentzhen-littlebit -- vectors`",
            vectors::FILE
        );
        let loaded: vectors::Vectors = serde_json::from_str(&committed).unwrap();
        assert_eq!(loaded, v);
        let names: Vec<&str> = v.cases.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, ["r16-layer-1", "r16-layer-2", "convolution-3x3"]);
        for case in &v.cases {
            assert_eq!(case.inputs.len(), case.steps.len(), "{}", case.name);
            let saturated: usize = case.steps.iter().map(|s| s.saturated).sum();
            let y: Vec<u8> = case.steps.iter().flat_map(|s| s.y.clone()).collect();
            assert!(saturated > 0, "{}: no latent saturates", case.name);
            assert!(
                y.contains(&0) && y.contains(&255),
                "{}: no clamp",
                case.name
            );
            assert!(y.iter().any(|&y| 0 < y && y < 255), "{}", case.name);
        }
        let conv = &v.cases[2];
        let c = conv.convolution.as_ref().unwrap();
        let windows = integer::windows(&c.grid, c.height, c.width, c.channels, c.kernel).unwrap();
        assert_eq!(conv.inputs, windows);
        assert_eq!(conv.inputs.len(), 25);
    }

    /// The argmax picks the highest score, and the first of the highest when two or more tie, as
    /// the FPGA's does.
    #[test]
    fn argmax_picks_the_first_of_the_highest() {
        assert_eq!(integer::argmax(&[3, 9, -1, 4]), 1);
        assert_eq!(integer::argmax(&[3, 9, -1, 9]), 1);
        assert_eq!(integer::argmax(&[5, 5, 5]), 0);
        assert_eq!(integer::argmax(&[i32::MIN, i32::MIN, i32::MAX]), 2);
    }

    /// The output layer's and the whole networks' test vectors: each input's scores and digit are
    /// the golden model's, and the made-up output layer's scores reach both ends of an int32 and
    /// tie, the first of the highest picked over a later one. The sweep's networks, r4, r8, r32
    /// and r64, E11's r42 and E10, the wider network, each have a case on r16's images.
    #[test]
    fn output_and_network_vectors_are_what_the_golden_model_computes() {
        let (net, sweep) = vectors::networks(&root()).unwrap();
        let v = vectors::vectors(&net, &sweep).unwrap();
        let names: Vec<&str> = v.network_cases.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, ["r16", "r4", "r8", "r32", "r64", "r42", "E10"]);
        for (case, other) in v.network_cases[1..].iter().zip(&sweep) {
            assert_eq!(case.images, v.network_cases[0].images, "{}", case.name);
            for ((image, scores), &digit) in case.images.iter().zip(&case.scores).zip(&case.digits)
            {
                assert_eq!(&other.scores(image).unwrap().0, scores, "{}", case.name);
                assert_eq!(digit, integer::argmax(scores), "{}", case.name);
            }
        }
        let names: Vec<&str> = v.dense_cases.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, ["r16-output", "dense-extremes"]);
        for case in &v.dense_cases {
            let layer = case.layer.as_ref().unwrap_or(&net.output);
            assert_eq!(case.inputs.len(), case.scores.len(), "{}", case.name);
            for ((x, scores), &digit) in case.inputs.iter().zip(&case.scores).zip(&case.digits) {
                assert_eq!(&layer.scores(x).unwrap(), scores, "{}", case.name);
                assert_eq!(digit, integer::argmax(scores), "{}", case.name);
            }
        }
        let extremes = &v.dense_cases[1];
        let all: Vec<i32> = extremes.scores.iter().flatten().copied().collect();
        assert!(all.contains(&i32::MIN) && all.contains(&i32::MAX));
        let tied_low = extremes.scores.iter().zip(&extremes.digits).any(|(s, &d)| {
            let highest = s[d];
            s.iter().skip(d + 1).any(|&t| t == highest)
        });
        assert!(
            tied_low,
            "no input ties a later output with the digit picked"
        );
        let r16 = &v.network_cases[0];
        assert_eq!((r16.name.as_str(), r16.images.len()), ("r16", 6));
        for ((image, scores), &digit) in r16.images.iter().zip(&r16.scores).zip(&r16.digits) {
            assert_eq!(&net.scores(image).unwrap().0, scores);
            assert_eq!(digit, integer::argmax(scores));
        }
        let digits: std::collections::BTreeSet<usize> = v
            .dense_cases
            .iter()
            .flat_map(|c| c.digits.clone())
            .chain(r16.digits.clone())
            .collect();
        assert!(digits.len() >= 3, "the cases pick only digits {digits:?}");
    }

    /// `int8` takes the finest power-of-two grid that holds the largest magnitude in 127 steps,
    /// and rounds each value to the nearest step, a half away from zero.
    #[test]
    fn int8_takes_the_finest_grid_that_holds_the_largest() {
        assert_eq!(
            export::int8(&[1.0, -0.5, 0.01]).unwrap(),
            (vec![64, -32, 1], -6)
        );
        assert_eq!(
            export::int8(&[127.0, 2.5, -2.5]).unwrap(),
            (vec![127, 3, -3], 0)
        );
        assert_eq!(export::int8(&[127.5]).unwrap(), (vec![64], 1));
        assert!(export::int8(&[0.0, 0.0]).is_err());
    }

    /// integer.json is refused unless its keys are known, its signs are signs, its shapes fit
    /// together and its exponents add up.
    #[test]
    fn an_integer_network_refuses_what_does_not_fit() {
        let text = fs::read_to_string(r16().join(export::FILE)).unwrap();
        integer::Network::parse(&text).unwrap();
        let v = text.find("\"v\": [").unwrap();
        let sign = v + 6 + text[v + 6..].find('"').unwrap() + 1;
        let mut not_a_sign = text.clone();
        not_a_sign.replace_range(sign..sign + 1, "x");
        let mut short = text.clone();
        short.remove(sign);
        let cases = [
            (
                text.replacen("\"rank\"", "\"extra\": 1, \"rank\"", 1),
                "unknown field",
            ),
            (not_a_sign, "a sign is"),
            (short, "not as long as its rank"),
            (
                text.replacen("\"latent_shift\": 10,", "\"latent_shift\": 11,", 1),
                "add up",
            ),
            (
                text.replacen("\"pixel_shift\": 1,", "\"pixel_shift\": 2,", 1),
                "not the outputs before it",
            ),
        ];
        for (bad, why) in cases {
            assert_ne!(bad, text);
            let refused = integer::Network::parse(&bad).unwrap_err();
            assert!(refused.contains(why), "{refused}");
        }
    }

    /// r16's integer.json is what the exporter writes from r16's committed weights and the
    /// 60,000 training images.
    #[test]
    fn r16_integer_json_is_what_the_exporter_writes() {
        let run = run::Run::load(&r16()).unwrap();
        let (train, _) = mnist::load(&root(), &mnist_dir()).unwrap();
        let net = export::export(&run, &[0, 2], 4, &train).unwrap();
        assert_eq!(net.weights_sha256, run.summary.weights_sha256);
        let committed = fs::read_to_string(r16().join(export::FILE)).unwrap();
        assert!(
            committed == export::to_json(&net),
            "runs/r16/{} is stale: run `cargo run -p tentzhen-littlebit -- export \
             models/littlebit/runs/r16`",
            export::FILE
        );
    }

    /// On the training images, which chose its latent shifts, r16's integer network saturates no
    /// latent, and gets 58,809 of the 60,000 right, against the float run's 58,798.
    #[test]
    fn r16_integer_saturates_no_latent_on_the_training_images() {
        let (train, _) = mnist::load(&root(), &mnist_dir()).unwrap();
        let p = r16_integer().predict(&train).unwrap();
        assert_eq!(p.saturated, 0);
        assert_eq!(p.correct(&train), 58_809);
    }

    /// Check 1 at r = 16 (docs/littlebit.md, "Verification"): the integer golden model, run from
    /// r16's integer.json, gets at most 0.5 points fewer of the 10,000 test images right than
    /// the float run. It gets 9,717 against the float's 9,713, and the two pick the same digit on
    /// 9,980 of them. No latent is saturated.
    #[test]
    fn r16_integer_is_within_half_a_point_of_float() {
        let run = run::Run::load(&r16()).unwrap();
        let (_, test) = mnist::load(&root(), &mnist_dir()).unwrap();
        let p = r16_integer().predict(&test).unwrap();
        let (integer, float) = (p.correct(&test), run.summary.test_correct);
        assert!(
            integer + 50 >= float,
            "{integer} test images right, more than 50 under the float run's {float}"
        );
        assert_eq!((integer, float), (9_717, 9_713));
        assert_eq!(p.saturated, 0);
        let float_digits = run.predict(&[0, 2, 4], &test).unwrap();
        let same = float_digits
            .iter()
            .zip(&p.digits)
            .filter(|(f, i)| f == i)
            .count();
        assert_eq!(same, 9_980);
    }

    /// The rank sweep's other runs, E4, E5, E7 and E8, and the ranks E11's search tried: 40, 42,
    /// 43, 44, 48 and 56 (docs/littlebit.md, "Experiments").
    const SWEEP: [usize; 10] = [4, 8, 32, 40, 42, 43, 44, 48, 56, 64];

    fn rank(r: usize) -> PathBuf {
        root().join(format!("models/littlebit/runs/r{r}"))
    }

    /// Each of the sweep's committed runs is the one its summary names, in the shapes of the
    /// LittleBit MLP at its rank, as r16's is; each started from E1's committed weights, trained
    /// on the MNIST the pins name.
    #[test]
    fn the_sweep_s_weights_are_the_ones_their_summaries_name() {
        let e1 = run::Run::load(&e1()).unwrap();
        let pins = mnist::Pins::load(&root()).unwrap().sha256;
        for r in SWEEP {
            let run = run::Run::load(&rank(r)).unwrap();
            let s = &run.summary;
            let shapes: Vec<(&str, Vec<usize>)> = s
                .layout
                .iter()
                .map(|t| (t.name.as_str(), t.shape.clone()))
                .collect();
            let want: Vec<(&str, Vec<usize>)> = vec![
                ("0.g", vec![784]),
                ("0.v", vec![784, r]),
                ("0.l", vec![r]),
                ("0.u", vec![256, r]),
                ("0.h", vec![256]),
                ("0.bias", vec![256]),
                ("2.g", vec![256]),
                ("2.v", vec![256, r]),
                ("2.l", vec![r]),
                ("2.u", vec![128, r]),
                ("2.h", vec![128]),
                ("2.bias", vec![128]),
                ("4.weight", vec![10, 128]),
                ("4.bias", vec![10]),
            ];
            assert_eq!(shapes, want, "r{r}");
            // Each layer's g (one per input), h and bias (one per output), V, U and l (r each per
            // input, per output and once), then the output layer: 3,098 + 1,426 r, r16's 25,914.
            let parameters =
                784 + 2 * 256 + 256 + 2 * 128 + r * (784 + 256 + 1 + 256 + 128 + 1) + 10 * 128 + 10;
            assert_eq!(
                (s.run.clone(), s.weights, s.parameters),
                (format!("r{r}"), 234_752, parameters)
            );
            let lb = s.littlebit.as_ref().unwrap();
            assert_eq!(lb.rank, r);
            assert_eq!(lb.start.run, e1.summary.run);
            assert_eq!(lb.start.weights_sha256, e1.summary.weights_sha256);
            assert_eq!(s.mnist_sha256, pins, "r{r}");
        }
    }

    /// Each of the sweep's runs, re-scored here from its committed weights in f32, gets right the
    /// test and training images its summary says, as `want` lists them by rank: at r = 4, 8,802 and
    /// 52,869; at r = 64, 9,814 and 59,887.
    #[test]
    fn the_sweep_scores_what_its_summaries_say() {
        let (train, test) = mnist::load(&root(), &mnist_dir()).unwrap();
        let mut got = Vec::new();
        for r in SWEEP {
            let run = run::Run::load(&rank(r)).unwrap();
            let s = &run.summary;
            assert_eq!(
                run.correct(&[0, 2, 4], &test).unwrap(),
                s.test_correct,
                "r{r}"
            );
            assert_eq!(
                run.correct(&[0, 2, 4], &train).unwrap(),
                s.train_correct,
                "r{r}"
            );
            got.push((r, s.test_correct, s.train_correct));
        }
        let want = [
            (4, 8_802, 52_869),
            (8, 9_464, 57_036),
            (32, 9_772, 59_602),
            (40, 9_771, 59_695),
            (42, 9_796, 59_742),
            (43, 9_802, 59_749),
            (44, 9_803, 59_747),
            (48, 9_792, 59_793),
            (56, 9_795, 59_847),
            (64, 9_814, 59_887),
        ];
        assert_eq!(got, want);
    }

    /// Each of the sweep's integer.json files is what the exporter writes from its committed
    /// weights and the 60,000 training images.
    #[test]
    fn the_sweep_s_integer_json_is_what_the_exporter_writes() {
        let (train, _) = mnist::load(&root(), &mnist_dir()).unwrap();
        for r in SWEEP {
            let run = run::Run::load(&rank(r)).unwrap();
            let net = export::export(&run, &[0, 2], 4, &train).unwrap();
            assert_eq!(net.weights_sha256, run.summary.weights_sha256);
            let committed = fs::read_to_string(rank(r).join(export::FILE)).unwrap();
            assert!(
                committed == export::to_json(&net),
                "runs/r{r}/{} is stale: run `cargo run -p tentzhen-littlebit -- export \
                 models/littlebit/runs/r{r}`",
                export::FILE
            );
        }
    }

    /// Check 1 across the sweep: at each rank the integer golden model gets at most 0.5 points
    /// fewer of the 10,000 test images right than the float run, and on the training images, which
    /// chose its latent shifts, it saturates no latent. `want` lists, by rank, the integer and float
    /// test images right, the test images on which the two pick the same digit, the integer
    /// training images right, and the latents saturated on the test images: one, at r = 4 only.
    #[test]
    fn the_sweep_s_integer_is_within_half_a_point_of_float() {
        let (train, test) = mnist::load(&root(), &mnist_dir()).unwrap();
        let mut got = Vec::new();
        for r in SWEEP {
            let run = run::Run::load(&rank(r)).unwrap();
            let text = fs::read_to_string(rank(r).join(export::FILE)).unwrap();
            let net = integer::Network::parse(&text).unwrap();
            let p = net.predict(&test).unwrap();
            let (integer, float) = (p.correct(&test), run.summary.test_correct);
            assert!(integer + 50 >= float, "r{r}: {integer} against {float}");
            let float_digits = run.predict(&[0, 2, 4], &test).unwrap();
            let same = float_digits
                .iter()
                .zip(&p.digits)
                .filter(|(f, i)| f == i)
                .count();
            let t = net.predict(&train).unwrap();
            assert_eq!(
                t.saturated, 0,
                "r{r}: latents saturated on the training images"
            );
            got.push((r, integer, float, same, t.correct(&train), p.saturated));
        }
        let want = [
            (4, 8_791, 8_802, 9_932, 52_850, 1),
            (8, 9_463, 9_464, 9_974, 57_018, 0),
            (32, 9_776, 9_772, 9_993, 59_597, 0),
            (40, 9_771, 9_771, 9_993, 59_696, 0),
            (42, 9_795, 9_796, 9_989, 59_735, 0),
            (43, 9_805, 9_802, 9_991, 59_741, 0),
            (44, 9_803, 9_803, 9_994, 59_751, 0),
            (48, 9_791, 9_792, 9_991, 59_791, 0),
            (56, 9_794, 9_795, 9_992, 59_852, 0),
            (64, 9_815, 9_814, 9_993, 59_886, 0),
        ];
        assert_eq!(got, want);
    }

    /// E10's start: the 784 -> 1024 -> 512 -> 10 MLP, dense FP32, trained as E1 was.
    fn e10_start() -> PathBuf {
        root().join("models/littlebit/runs/e10-start")
    }

    /// E10: that MLP's hidden layers made LittleBit layers at r = 9, fine-tuned as the sweep was.
    fn e10() -> PathBuf {
        root().join("models/littlebit/runs/e10")
    }

    /// The bits a LittleBit layer of `run` holds, its signs and its 8-bit scales: r (in + out) +
    /// 8 (in + out + r) (docs/littlebit.md, "Bit accounting"), from the layer's g, h and l.
    fn littlebit_bits(run: &run::Run, layer: usize) -> usize {
        let len = |name: &str| run.tensors[&format!("{layer}.{name}")].len();
        let (inputs, outputs, r) = (len("g"), len("h"), len("l"));
        r * (inputs + outputs) + 8 * (inputs + outputs + r)
    }

    /// E10's start and E10 are the runs their summaries name. E10's start is the wider MLP,
    /// 784 -> 1024 -> 512 -> 10, dense (1,332,224 weights). E10 started from it, its hidden layers
    /// LittleBit layers at r = 9, and its output layer dense. Both trained on the MNIST the pins
    /// name. E10's two hidden layers hold 56,992 bits, signs and 8-bit scales, within E7's
    /// 57,472 (r32's), and at r = 10 they would hold 60,352, past it: r = 9 is the largest rank
    /// whose hidden layers fit E7's bits.
    #[test]
    fn e10_weights_are_the_ones_their_summaries_name() {
        let pins = mnist::Pins::load(&root()).unwrap().sha256;
        let start = run::Run::load(&e10_start()).unwrap();
        let s = &start.summary;
        let shapes: Vec<(&str, &[usize])> = s
            .layout
            .iter()
            .map(|t| (t.name.as_str(), t.shape.as_slice()))
            .collect();
        let want: [(&str, &[usize]); 6] = [
            ("0.weight", &[1024, 784]),
            ("0.bias", &[1024]),
            ("2.weight", &[512, 1024]),
            ("2.bias", &[512]),
            ("4.weight", &[10, 512]),
            ("4.bias", &[10]),
        ];
        assert_eq!(shapes, want);
        assert_eq!(
            (s.run.as_str(), s.weights, s.parameters),
            ("E10-start", 1_332_224, 1_333_770)
        );
        assert_eq!(s.mnist_sha256, pins);

        let run = run::Run::load(&e10()).unwrap();
        let s = &run.summary;
        let shapes: Vec<(&str, &[usize])> = s
            .layout
            .iter()
            .map(|t| (t.name.as_str(), t.shape.as_slice()))
            .collect();
        let want: [(&str, &[usize]); 14] = [
            ("0.g", &[784]),
            ("0.v", &[784, 9]),
            ("0.l", &[9]),
            ("0.u", &[1024, 9]),
            ("0.h", &[1024]),
            ("0.bias", &[1024]),
            ("2.g", &[1024]),
            ("2.v", &[1024, 9]),
            ("2.l", &[9]),
            ("2.u", &[512, 9]),
            ("2.h", &[512]),
            ("2.bias", &[512]),
            ("4.weight", &[10, 512]),
            ("4.bias", &[10]),
        ];
        assert_eq!(shapes, want);
        assert_eq!(
            (s.run.as_str(), s.weights, s.parameters),
            ("E10", 1_332_224, 40_124)
        );
        let lb = s.littlebit.as_ref().unwrap();
        assert_eq!(lb.rank, 9);
        assert_eq!(lb.start.run, start.summary.run);
        assert_eq!(lb.start.weights_sha256, start.summary.weights_sha256);
        assert_eq!(s.mnist_sha256, pins);

        let e7 = run::Run::load(&rank(32)).unwrap();
        let bits = |run: &run::Run| littlebit_bits(run, 0) + littlebit_bits(run, 2);
        assert_eq!((bits(&run), bits(&e7)), (56_992, 57_472));
        // At r = 10, each layer's signs grow by in + out, and its scales by 8.
        let more = bits(&run) + (784 + 1024) + 8 + (1024 + 512) + 8;
        assert_eq!(more, 60_352);
        assert!(bits(&run) <= bits(&e7) && more > bits(&e7));
    }

    /// E10's start and E10, re-scored here from their committed weights in f32, get right the test
    /// and training images their summaries say: E10's start, 9,856 and all 60,000; E10, 9,555
    /// and 57,827.
    #[test]
    fn e10_scores_what_its_summaries_say() {
        let (train, test) = mnist::load(&root(), &mnist_dir()).unwrap();
        let mut got = Vec::new();
        for folder in [e10_start(), e10()] {
            let run = run::Run::load(&folder).unwrap();
            let s = &run.summary;
            assert_eq!(run.correct(&[0, 2, 4], &test).unwrap(), s.test_correct);
            assert_eq!(run.correct(&[0, 2, 4], &train).unwrap(), s.train_correct);
            got.push((s.run.clone(), s.test_correct, s.train_correct));
        }
        let want = [
            ("E10-start".to_string(), 9_856, 60_000),
            ("E10".to_string(), 9_555, 57_827),
        ];
        assert_eq!(got, want);
    }

    /// E10's integer.json is what the exporter writes from its committed weights and the 60,000
    /// training images.
    #[test]
    fn e10_integer_json_is_what_the_exporter_writes() {
        let run = run::Run::load(&e10()).unwrap();
        let (train, _) = mnist::load(&root(), &mnist_dir()).unwrap();
        let net = export::export(&run, &[0, 2], 4, &train).unwrap();
        assert_eq!(net.weights_sha256, run.summary.weights_sha256);
        let committed = fs::read_to_string(e10().join(export::FILE)).unwrap();
        assert!(
            committed == export::to_json(&net),
            "runs/e10/{} is stale: run `cargo run -p tentzhen-littlebit -- export \
             models/littlebit/runs/e10`",
            export::FILE
        );
    }

    /// Check 1 for E10: the integer golden model gets at most 0.5 points fewer of the 10,000 test
    /// images right than the float run, and saturates no latent on the training images, which
    /// chose its latent shifts. It gets 9,546 test images right, against the float run's 9,555;
    /// the two pick the same digit on 9,976; it gets 57,803 training images right; and it
    /// saturates no latent on the test images either.
    #[test]
    fn e10_integer_is_within_half_a_point_of_float() {
        let (train, test) = mnist::load(&root(), &mnist_dir()).unwrap();
        let run = run::Run::load(&e10()).unwrap();
        let text = fs::read_to_string(e10().join(export::FILE)).unwrap();
        let net = integer::Network::parse(&text).unwrap();
        let p = net.predict(&test).unwrap();
        let (integer, float) = (p.correct(&test), run.summary.test_correct);
        assert!(integer + 50 >= float, "{integer} against {float}");
        let float_digits = run.predict(&[0, 2, 4], &test).unwrap();
        let same = float_digits
            .iter()
            .zip(&p.digits)
            .filter(|(f, i)| f == i)
            .count();
        let t = net.predict(&train).unwrap();
        assert_eq!(t.saturated, 0, "latents saturated on the training images");
        let got = (integer, float, same, t.correct(&train), p.saturated);
        assert_eq!(got, (9_546, 9_555, 9_976, 57_803, 0));
    }

    /// Gate 4's two plots, under docs/img/, are what `plots` draws from docs/littlebit.md's
    /// experiments table.
    #[test]
    fn the_plots_are_what_the_experiments_table_draws() {
        let doc = fs::read_to_string(root().join(plots::DOC)).unwrap();
        let rows = plots::experiments(&doc).unwrap();
        for (file, svg) in [
            (plots::ACCURACY, plots::accuracy(&rows).unwrap()),
            (plots::CELLS, plots::cells(&rows).unwrap()),
        ] {
            let committed = fs::read_to_string(root().join(file)).unwrap();
            assert!(
                committed == svg,
                "{file} is stale: run `cargo run -p tentzhen-littlebit -- plots`"
            );
        }
    }

    /// The experiments table's test accuracies, which the accuracy plot draws, are what the runs
    /// get on MNIST's 10,000 test images: each run's integer network as the golden model scores
    /// it, then in brackets its float run's, from its summary, which the tests above re-score.
    /// E1's is its float run's alone. E9 and E12 are E6's network, r16, in other builds.
    #[test]
    fn the_experiments_table_s_accuracies_are_what_the_runs_get() {
        let doc = fs::read_to_string(root().join(plots::DOC)).unwrap();
        let rows = plots::experiments(&doc).unwrap();
        let (_, test) = mnist::load(&root(), &mnist_dir()).unwrap();
        let runs = root().join("models/littlebit/runs");
        let folders = [
            ("E1", "e1"),
            ("E2", "e2"),
            ("E3", "e3"),
            ("E4", "r4"),
            ("E5", "r8"),
            ("E6", "r16"),
            ("E7", "r32"),
            ("E8", "r64"),
            ("E9", "r16"),
            ("E10", "e10"),
            ("E11", "r42"),
            ("E12", "r16"),
        ];
        let names: Vec<&str> = rows.iter().map(|e| e.run.as_str()).collect();
        assert_eq!(names, folders.map(|(run, _)| run));
        let percent = |n: usize| format!("{:.2}", n as f64 / 100.0);
        for (row, (name, folder)) in rows.iter().zip(folders) {
            let run = run::Run::load(&runs.join(folder)).unwrap();
            let float = percent(run.summary.test_correct);
            let want = if name == "E1" {
                format!("{float} (float)")
            } else {
                let integer = if run.summary.quantized.is_some() {
                    let net = export::baseline(&run, &[0, 2], 4).unwrap();
                    net.predict(&test).unwrap().correct(&test)
                } else {
                    let text = fs::read_to_string(runs.join(folder).join(export::FILE)).unwrap();
                    let net = integer::Network::parse(&text).unwrap();
                    net.predict(&test).unwrap().correct(&test)
                };
                format!("{} (float {float})", percent(integer))
            };
            assert!(
                row.accuracy_cell.starts_with(&want),
                "{name}: the table has {:?}, the runs give {want:?}",
                row.accuracy_cell
            );
        }
    }
}
