//! The verifier's rules (`src/lib.rs`) held on real quotes from swtpm, made by
//! `examples/swtpm_quotes.rs` and committed as `tests/swtpm_quotes.txt`, and on quotes changed the
//! ways a hub or a look-alike computer could change them.

use p256::ecdsa::signature::hazmat::PrehashSigner;
use p256::ecdsa::{Signature, SigningKey, VerifyingKey};
use sha2::{Digest as _, Sha256};
use std::collections::BTreeMap;
use tentzhen_verifier::{
    Answer, Digest, Enrolled, PCRS, Quoted, Red, Refused, Verdict, check_in, check_quote, judge,
};

/// The quotes file's names, all of them and no others.
const NAMES: [&str; 2] = ["ak_a", "ak_b"];
const QUOTES: [&str; 8] = [
    "out",
    "in",
    "extended",
    "restarted",
    "eight",
    "sha1",
    "two",
    "other",
];
const PARTS: [&str; 4] = ["challenge", "attest", "signature", "pcrs"];

struct Quote {
    challenge: [u8; 32],
    attest: Vec<u8>,
    signature: Vec<u8>,
    pcrs: Vec<Digest>,
}

impl Quote {
    fn pcrs(&self) -> [Digest; PCRS] {
        self.pcrs.clone().try_into().unwrap()
    }
}

struct Quotes {
    ak_a: VerifyingKey,
    ak_b: VerifyingKey,
    q: BTreeMap<String, Quote>,
}

impl Quotes {
    fn get(&self, name: &str) -> &Quote {
        &self.q[name]
    }
}

fn unhex(s: &str) -> Vec<u8> {
    assert!(s.len().is_multiple_of(2), "odd hex length");
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

fn key(xy: &[u8]) -> VerifyingKey {
    VerifyingKey::from_sec1_bytes(&[&[0x04], xy].concat()).unwrap()
}

/// Loads `tests/swtpm_quotes.txt`, refusing a name it doesn't know and a name missing.
fn quotes() -> Quotes {
    let text = include_str!("swtpm_quotes.txt");
    let mut found: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    for line in text.lines() {
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }
        let (name, hex) = line.split_once(" = ").expect("`name = hex`");
        assert!(
            found.insert(name.to_string(), unhex(hex)).is_none(),
            "{name} twice"
        );
    }
    let mut expected: Vec<String> = NAMES.iter().map(|n| (*n).to_string()).collect();
    for q in QUOTES {
        for p in PARTS {
            expected.push(format!("{q}.{p}"));
        }
    }
    expected.sort();
    assert_eq!(found.keys().cloned().collect::<Vec<_>>(), expected);
    let mut q = BTreeMap::new();
    for name in QUOTES {
        let part = |p: &str| found[&format!("{name}.{p}")].clone();
        q.insert(
            name.to_string(),
            Quote {
                challenge: part("challenge").try_into().unwrap(),
                attest: part("attest"),
                signature: part("signature"),
                pcrs: part("pcrs")
                    .chunks(32)
                    .map(|c| c.try_into().unwrap())
                    .collect(),
            },
        );
    }
    Quotes {
        ak_a: key(&found["ak_a"]),
        ak_b: key(&found["ak_b"]),
        q,
    }
}

fn checked(k: &VerifyingKey, q: &Quote) -> Result<Quoted, Refused> {
    check_quote(k, &q.challenge, &q.attest, &q.signature, &q.pcrs())
}

/// A TPMT_SIGNATURE: ECDSA with SHA-256, r and s as TPM2B_ECC_PARAMETERs.
fn tpmt_signature(sig: &Signature) -> Vec<u8> {
    let (r, s) = sig.split_bytes();
    let mut v = vec![0x00, 0x18, 0x00, 0x0B, 0x00, 0x20];
    v.extend_from_slice(&r);
    v.extend_from_slice(&[0x00, 0x20]);
    v.extend_from_slice(&s);
    v
}

/// A key outside any TPM, and its signature over `bytes`.
fn software_signed(bytes: &[u8]) -> (VerifyingKey, Vec<u8>) {
    let sk = SigningKey::from_slice(&[0x42; 32]).unwrap();
    let sig: Signature = sk.sign_prehash(&Sha256::digest(bytes)).unwrap();
    (*sk.verifying_key(), tpmt_signature(&sig))
}

#[test]
fn the_tpm_s_own_quotes_pass_the_checks() {
    let t = quotes();
    for name in ["out", "in", "extended", "restarted"] {
        let q = checked(&t.ak_a, t.get(name)).unwrap_or_else(|e| panic!("{name}: {e:?}"));
        assert_eq!(q.pcrs, t.get(name).pcrs(), "{name}");
    }
    assert!(checked(&t.ak_b, t.get("other")).is_ok());
}

#[test]
fn coming_back_to_an_untouched_computer_is_green() {
    let t = quotes();
    let out = checked(&t.ak_a, t.get("out")).unwrap();
    let back = checked(&t.ak_a, t.get("in")).unwrap();
    assert_eq!(judge(&out, &back), Verdict::Green);
}

#[test]
fn a_restart_is_red_even_with_every_pcr_as_it_was() {
    let t = quotes();
    let out = checked(&t.ak_a, t.get("out")).unwrap();
    let back = checked(&t.ak_a, t.get("restarted")).unwrap();
    assert_eq!(back.pcrs, out.pcrs, "the same extends after the restart");
    assert_ne!(back.reset_count, out.reset_count);
    assert_eq!(judge(&out, &back), Verdict::Red(Red::Restarted));
}

#[test]
fn a_pcr_extended_without_a_restart_is_red() {
    let t = quotes();
    let out = checked(&t.ak_a, t.get("out")).unwrap();
    let back = checked(&t.ak_a, t.get("extended")).unwrap();
    assert_eq!(back.reset_count, out.reset_count);
    assert_eq!(back.restart_count, out.restart_count);
    assert_eq!(judge(&out, &back), Verdict::Red(Red::BootChainChanged));
}

#[test]
fn an_old_answer_sent_again_is_refused() {
    let t = quotes();
    for (old, now) in [("out", "in"), ("in", "extended")] {
        let q = t.get(old);
        let challenge = t.get(now).challenge;
        assert_eq!(
            check_quote(&t.ak_a, &challenge, &q.attest, &q.signature, &q.pcrs()),
            Err(Refused::Challenge),
            "{old} sent again as {now}"
        );
    }
}

#[test]
fn another_tpm_s_quote_is_refused() {
    let t = quotes();
    assert_eq!(checked(&t.ak_a, t.get("other")), Err(Refused::Signature));
}

#[test]
fn pcr_values_other_than_the_quoted_ones_are_refused() {
    let t = quotes();
    let q = t.get("in");
    let mut pcrs = q.pcrs();
    pcrs[9][0] ^= 1;
    assert_eq!(
        check_quote(&t.ak_a, &q.challenge, &q.attest, &q.signature, &pcrs),
        Err(Refused::PcrValues)
    );
    let swapped = t.get("extended").pcrs();
    assert_eq!(
        check_quote(&t.ak_a, &q.challenge, &q.attest, &q.signature, &swapped),
        Err(Refused::PcrValues)
    );
}

#[test]
fn any_byte_changed_in_a_quote_or_its_signature_is_refused() {
    let t = quotes();
    let q = t.get("in");
    for i in 0..q.attest.len() {
        let mut attest = q.attest.clone();
        attest[i] ^= 0x01;
        assert!(
            check_quote(&t.ak_a, &q.challenge, &attest, &q.signature, &q.pcrs()).is_err(),
            "attest byte {i}"
        );
    }
    for i in 0..q.signature.len() {
        let mut signature = q.signature.clone();
        signature[i] ^= 0x01;
        assert!(
            check_quote(&t.ak_a, &q.challenge, &q.attest, &signature, &q.pcrs()).is_err(),
            "signature byte {i}"
        );
    }
    let mut longer = q.signature.clone();
    longer.push(0);
    assert_eq!(
        check_quote(&t.ak_a, &q.challenge, &q.attest, &longer, &q.pcrs()),
        Err(Refused::Signature)
    );
}

#[test]
fn a_quote_of_other_pcrs_or_another_bank_is_refused() {
    let t = quotes();
    let eight = t.get("eight");
    assert_eq!(eight.pcrs.len(), 8);
    let extended = t.get("extended").pcrs();
    assert_eq!(
        check_quote(
            &t.ak_a,
            &eight.challenge,
            &eight.attest,
            &eight.signature,
            &extended
        ),
        Err(Refused::Selection),
        "PCRs 0 to 7"
    );
    for name in ["sha1", "two"] {
        let q = t.get(name);
        assert_eq!(checked(&t.ak_a, q), Err(Refused::Selection), "{name}");
    }
}

/// What a key outside any TPM can sign, the checks refuse unless it is a quote: a wrong magic, a
/// wrong type, or bytes after the quote.
#[test]
fn signed_bytes_that_are_not_a_tpm_s_quote_are_refused() {
    let t = quotes();
    let q = t.get("in");
    let mut magic = q.attest.clone();
    magic[0] ^= 0x01;
    let mut kind = q.attest.clone();
    kind[5] = 0x17; // TPM_ST_ATTEST_CERTIFY
    let mut trailing = q.attest.clone();
    trailing.push(0);
    for (what, attest) in [("magic", magic), ("type", kind), ("trailing", trailing)] {
        let (k, signature) = software_signed(&attest);
        assert_eq!(
            check_quote(&k, &q.challenge, &attest, &signature, &q.pcrs()),
            Err(Refused::NotAQuote),
            "{what}"
        );
    }
}

/// The checks alone cannot tell a TPM's key from any other: a copy of a real quote, signed by a key
/// outside any TPM, passes under that key. That is why enrollment must prove the key is the TPM's
/// own (`docs/fab.md`, "Enrolling a computer").
#[test]
fn a_quote_signed_outside_any_tpm_passes_under_its_own_key() {
    let t = quotes();
    let q = t.get("in");
    let (k, signature) = software_signed(&q.attest);
    assert!(check_quote(&k, &q.challenge, &q.attest, &signature, &q.pcrs()).is_ok());
    assert_eq!(
        check_quote(&t.ak_a, &q.challenge, &q.attest, &signature, &q.pcrs()),
        Err(Refused::Signature)
    );
}

fn quoted(reset: u32, restart: u32, clock: u64, firmware: u64, pcr9: u8) -> Quoted {
    let mut pcrs = [[0u8; 32]; PCRS];
    pcrs[9][0] = pcr9;
    Quoted {
        reset_count: reset,
        restart_count: restart,
        clock,
        firmware_version: firmware,
        pcrs,
    }
}

#[test]
fn the_findings_table_in_its_order() {
    let out = quoted(5, 0, 1000, 7, 0);
    let red = |r| Verdict::Red(r);
    for (now, verdict) in [
        (quoted(5, 0, 2000, 7, 0), Verdict::Green),
        (quoted(5, 0, 1000, 7, 0), Verdict::Green),
        (quoted(6, 0, 2000, 7, 0), red(Red::Restarted)),
        (quoted(5, 1, 2000, 7, 0), red(Red::Restarted)),
        (quoted(5, 0, 2000, 7, 1), red(Red::BootChainChanged)),
        (quoted(5, 0, 2000, 8, 0), red(Red::TpmFirmwareChanged)),
        (quoted(5, 0, 999, 7, 0), Verdict::Amber),
        (quoted(6, 0, 999, 8, 1), red(Red::Restarted)),
        (quoted(5, 0, 999, 8, 1), red(Red::BootChainChanged)),
        (quoted(5, 0, 999, 8, 0), red(Red::TpmFirmwareChanged)),
    ] {
        assert_eq!(judge(&out, &now), verdict, "{now:?}");
    }
}

fn enrolled(t: &Quotes) -> [Enrolled; 2] {
    [
        Enrolled {
            id: 1,
            key: t.ak_a,
            snapshot: checked(&t.ak_a, t.get("out")).unwrap(),
            challenge: t.get("in").challenge,
        },
        Enrolled {
            id: 2,
            key: t.ak_b,
            snapshot: checked(&t.ak_b, t.get("other")).unwrap(),
            challenge: t.get("other").challenge,
        },
    ]
}

fn answer(id: u16, q: &Quote) -> Answer<'_> {
    Answer {
        id,
        attest: &q.attest,
        signature: &q.signature,
        pcrs: q.pcrs(),
    }
}

fn verdicts(list: &[Enrolled], answers: &[Answer]) -> Vec<Verdict> {
    let mut v = vec![Verdict::Green; list.len()];
    check_in(list, answers, &mut v);
    v
}

#[test]
fn every_enrolled_computer_is_judged_from_the_verifier_s_own_list() {
    let t = quotes();
    let list = enrolled(&t);
    let (a, b) = (t.get("in"), t.get("other"));
    let missing = Verdict::Red(Red::Missing(None));
    assert_eq!(
        verdicts(&list, &[answer(1, a), answer(2, b)]),
        [Verdict::Green, Verdict::Green]
    );
    assert_eq!(
        verdicts(&list, &[answer(2, b), answer(1, a)]),
        [Verdict::Green, Verdict::Green],
        "in any order"
    );
    assert_eq!(
        verdicts(&list, &[answer(1, a)]),
        [Verdict::Green, missing],
        "the hub leaves one out"
    );
    assert_eq!(
        verdicts(&list, &[]),
        [missing, missing],
        "the hub sends nothing"
    );
    assert_eq!(
        verdicts(&list, &[answer(1, a), answer(1, a), answer(2, b)]),
        [missing, Verdict::Green],
        "two answers for one computer"
    );
    assert_eq!(
        verdicts(&list, &[answer(1, a), answer(2, b), answer(99, a)]),
        [Verdict::Green, Verdict::Green],
        "an answer for a computer not enrolled"
    );
    assert_eq!(
        verdicts(&list, &[answer(1, b), answer(2, b)]),
        [
            Verdict::Red(Red::Missing(Some(Refused::Signature))),
            Verdict::Green
        ],
        "another TPM's quote under the first computer's number"
    );
}
