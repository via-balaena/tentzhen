//! The verifier's rules (`src/lib.rs`) held on real quotes from swtpm, made by
//! `examples/swtpm_quotes.rs` and committed as `tests/swtpm_quotes.txt`, on the event log of the
//! boot those quotes followed (`tests/boot/mod.rs`), and on quotes and logs changed the ways a hub
//! or a look-alike computer could change them.

mod boot;

use boot::{ALG_SHA1, ALG_SHA256, EV_NO_ACTION, Entry};
use p256::ecdsa::signature::hazmat::PrehashSigner;
use p256::ecdsa::{Signature, SigningKey, VerifyingKey};
use sha2::{Digest as _, Sha256};
use std::collections::BTreeMap;
use tentzhen_verifier::{
    Answer, BOOT_PCRS, Digest, Enrolled, PCRS, Quoted, Red, Reference, Refused, Verdict, check_in,
    check_log, check_quote, judge,
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

/// The banks the boot's log lists in its Spec ID event, with their digest sizes.
const BANKS: [(u16, u16); 2] = [(ALG_SHA1, 20), (ALG_SHA256, 32)];

/// Writes a TCG crypto-agile event log (PC Client Platform Firmware Profile 1.05 r23, §10): the
/// Spec ID event listing `algorithms`, in the SHA-1 log's form, then `entries`, all little-endian.
/// It is written here, not with the verifier's parser, so the logs don't come from what they test.
fn log_listing(algorithms: &[(u16, u16)], entries: &[Entry]) -> Vec<u8> {
    let le32 = |v: &mut Vec<u8>, x: u32| v.extend_from_slice(&x.to_le_bytes());
    let mut spec = b"Spec ID Event03\0".to_vec();
    le32(&mut spec, 0); // platformClass
    spec.extend_from_slice(&[0, 2, 0, 2]); // the spec's version 2.0, errata 0, an 8-byte UINTN
    le32(&mut spec, algorithms.len() as u32);
    for (algorithm, size) in algorithms {
        spec.extend_from_slice(&algorithm.to_le_bytes());
        spec.extend_from_slice(&size.to_le_bytes());
    }
    spec.push(0); // vendorInfoSize
    let mut v = Vec::new();
    le32(&mut v, 0);
    le32(&mut v, EV_NO_ACTION);
    v.extend_from_slice(&[0; 20]);
    le32(&mut v, spec.len() as u32);
    v.extend_from_slice(&spec);
    for e in entries {
        le32(&mut v, e.pcr);
        le32(&mut v, e.kind);
        le32(&mut v, e.digests.len() as u32);
        for (algorithm, digest) in &e.digests {
            v.extend_from_slice(&algorithm.to_le_bytes());
            v.extend_from_slice(digest);
        }
        le32(&mut v, e.data.len() as u32);
        v.extend_from_slice(&e.data);
    }
    v
}

fn log(entries: &[Entry]) -> Vec<u8> {
    log_listing(&BANKS, entries)
}

/// What `entries` extend into each of PCRs 0 to 9, in order: the reference the verifier would hold
/// for a computer that booted them.
fn reference(entries: &[Entry]) -> Vec<Vec<Digest>> {
    (0..BOOT_PCRS as u32)
        .map(|pcr| {
            entries
                .iter()
                .filter(|e| e.extended() && e.pcr == pcr)
                .map(Entry::sha256)
                .collect()
        })
        .collect()
}

fn held(r: &[Vec<Digest>]) -> Reference<'_> {
    std::array::from_fn(|pcr| r[pcr].as_slice())
}

fn position(entries: &[Entry], data: &str) -> usize {
    entries
        .iter()
        .position(|e| e.data == data.as_bytes())
        .unwrap()
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
    assert_eq!(judge(&out, &back, true), Verdict::Green);
}

#[test]
fn a_restart_is_red_even_with_every_pcr_as_it_was() {
    let t = quotes();
    let out = checked(&t.ak_a, t.get("out")).unwrap();
    let back = checked(&t.ak_a, t.get("restarted")).unwrap();
    assert_eq!(back.pcrs, out.pcrs, "the same extends after the restart");
    assert_ne!(back.reset_count, out.reset_count);
    assert_eq!(judge(&out, &back, true), Verdict::Red(Red::Restarted));
}

#[test]
fn a_pcr_extended_without_a_restart_is_red() {
    let t = quotes();
    let out = checked(&t.ak_a, t.get("out")).unwrap();
    let back = checked(&t.ak_a, t.get("extended")).unwrap();
    assert_eq!(back.reset_count, out.reset_count);
    assert_eq!(back.restart_count, out.restart_count);
    assert_eq!(
        judge(&out, &back, true),
        Verdict::Red(Red::BootChainChanged)
    );
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

#[test]
fn each_boot_s_log_replays_to_its_quote_and_is_the_reference() {
    let t = quotes();
    let boot = boot::boot();
    let r = reference(&boot);
    let booted = log(&boot);
    for name in ["out", "in", "restarted", "other"] {
        let q = t.get(name);
        assert_eq!(check_log(&booted, &q.pcrs(), &held(&r)), Ok(true), "{name}");
    }
    let q = t.get("extended");
    let mut then = boot.clone();
    then.push(boot::later());
    assert_eq!(
        check_log(&log(&then), &q.pcrs(), &held(&r)),
        Ok(false),
        "one more entry in PCR 9 than the reference holds"
    );
    assert_eq!(
        check_log(&booted, &q.pcrs(), &held(&r)),
        Err(Refused::EventLog),
        "the log from before that entry"
    );
}

/// A hub that changes the log, to hide what the quote holds or to show what it doesn't, is refused:
/// the log no longer replays to the quote.
#[test]
fn a_log_changed_by_the_hub_is_refused() {
    let t = quotes();
    let q = t.get("in");
    let boot = boot::boot();
    let r = reference(&boot);
    let (shim, grub) = (position(&boot, "shim"), position(&boot, "grub"));
    let changed = |change: &dyn Fn(&mut Vec<Entry>)| {
        let mut v = boot.clone();
        change(&mut v);
        log(&v)
    };
    for (what, log) in [
        (
            "a digest changed",
            changed(&|v| v[grub].digests[1].1[0] ^= 1),
        ),
        (
            "an entry dropped",
            changed(&|v| {
                v.remove(grub);
            }),
        ),
        (
            "an entry added",
            changed(&|v| v.insert(grub, Entry::measured(4, v[grub].kind, "another loader"))),
        ),
        (
            "two entries of a PCR swapped",
            changed(&|v| v.swap(shim, grub)),
        ),
        (
            "an entry moved to another PCR",
            changed(&|v| v[grub].pcr = 5),
        ),
        (
            "an entry said not to be extended",
            changed(&|v| v[grub].kind = EV_NO_ACTION),
        ),
    ] {
        assert_eq!(
            check_log(&log, &q.pcrs(), &held(&r)),
            Err(Refused::EventLog),
            "{what}"
        );
    }
}

/// What no quoted PCR holds, the hub can change without a finding, so the verifier must not judge
/// by it.
#[test]
fn what_no_quoted_pcr_holds_changes_nothing() {
    let t = quotes();
    let q = t.get("in");
    let boot = boot::boot();
    let r = reference(&boot);
    let (cmdline, kernel) = (
        position(&boot, "grub_cmd: linux /vmlinuz root=UUID=fab ro"),
        position(&boot, "/vmlinuz"),
    );
    let changed = |change: &dyn Fn(&mut Vec<Entry>)| {
        let mut v = boot.clone();
        change(&mut v);
        log(&v)
    };
    for (what, log) in [
        (
            "what each entry says it measured",
            changed(&|v| {
                v.iter_mut()
                    .for_each(|e| e.data = b"something else".to_vec())
            }),
        ),
        (
            "each extended entry's type",
            changed(&|v| {
                v.iter_mut()
                    .filter(|e| e.extended())
                    .for_each(|e| e.kind = 0x0000_0005) // EV_ACTION
            }),
        ),
        (
            "the SHA-1 digests",
            changed(&|v| v.iter_mut().for_each(|e| e.digests[0].1[0] ^= 1)),
        ),
        (
            "an entry that is only recorded",
            changed(&|v| v.insert(3, Entry::not_extended(b"another vendor\0\0"))),
        ),
        (
            "entries for PCRs 10 and 14",
            changed(&|v| {
                v.insert(
                    5,
                    Entry::measured(10, 0x0000_000D, "IMA's, not the boot log's"),
                );
                v.insert(5, Entry::measured(14, 0x0000_000D, "MokListX"));
            }),
        ),
        (
            "the order of entries for different PCRs",
            changed(&|v| v.swap(cmdline, kernel)),
        ),
    ] {
        assert_eq!(check_log(&log, &q.pcrs(), &held(&r)), Ok(true), "{what}");
    }
}

#[test]
fn a_log_the_verifier_cannot_read_is_refused() {
    let t = quotes();
    let q = t.get("in");
    let boot = boot::boot();
    let r = reference(&boot);
    let grub = position(&boot, "grub");
    let booted = log(&boot);
    let with = |change: &dyn Fn(&mut Entry)| {
        let mut v = boot.clone();
        change(&mut v[grub]);
        log(&v)
    };
    let mut signature = booted.clone();
    signature[32] ^= 1; // the Spec ID event's, after the first entry's 32 bytes (§10.1, Table 5)
    let mut longer = booted.clone();
    longer.push(0);
    let mut cases = vec![
        ("not a crypto-agile log", signature),
        (
            "no SHA-256 in the Spec ID event",
            log_listing(&[(ALG_SHA1, 20)], &boot),
        ),
        (
            "SHA-256 given another size",
            log_listing(&[(ALG_SHA1, 20), (ALG_SHA256, 20)], &boot),
        ),
        (
            "an entry with no SHA-256 digest",
            with(&|e| {
                e.digests.remove(1);
            }),
        ),
        (
            "an entry with two SHA-256 digests",
            with(&|e| e.digests.push(e.digests[1].clone())),
        ),
        (
            "an algorithm the Spec ID event doesn't list",
            with(&|e| e.digests.push((0x000C, vec![0; 48]))), // SHA-384
        ),
        (
            "the same, with no digest bytes, as the verifier can't know their size",
            with(&|e| e.digests.push((0x000C, vec![]))),
        ),
        ("a byte after the last entry", longer),
    ];
    let mut sha1_only = boot.clone();
    sha1_only.insert(
        grub,
        Entry::measured(4, sha1_only[grub].kind, "SHA-1 bank only"),
    );
    sha1_only[grub].digests.remove(1);
    cases.push(("an entry added with no SHA-256 digest", log(&sha1_only)));
    for n in 0..booted.len() {
        cases.push(("cut short", booted[..n].to_vec()));
    }
    for (what, log) in cases {
        assert_eq!(
            check_log(&log, &q.pcrs(), &held(&r)),
            Err(Refused::EventLog),
            "{what}, {} bytes",
            log.len()
        );
    }
}

/// Every byte of the log changed, one at a time: the log is refused, or it is a byte no quoted PCR
/// holds and the boot reads as before. The verifier never panics on it.
#[test]
fn any_byte_changed_in_the_log_is_refused_or_reads_as_the_same_boot() {
    let t = quotes();
    let q = t.get("in");
    let boot = boot::boot();
    let r = reference(&boot);
    let booted = log(&boot);
    for i in 0..booted.len() {
        for flip in [0x01, 0x80, 0xFF] {
            let mut v = booted.clone();
            v[i] ^= flip;
            let found = check_log(&v, &q.pcrs(), &held(&r));
            assert!(
                matches!(found, Ok(true) | Err(Refused::EventLog)),
                "byte {i} ^ {flip:#04x}: {found:?}"
            );
        }
    }
}

/// The computer booted something other than the reference: whichever entry differs, and however,
/// PCRs 0 to 9 aren't ours.
#[test]
fn a_boot_other_than_the_reference_is_not_ours() {
    let t = quotes();
    let q = t.get("in");
    let boot = boot::boot();
    let booted = log(&boot);
    let r = reference(&boot);
    let other = |change: &dyn Fn(&mut Vec<Vec<Digest>>)| {
        let mut r = r.clone();
        change(&mut r);
        r
    };
    assert_eq!(
        r[4].len(),
        4,
        "PCR 4: the boot option, the separator, shim, grub"
    );
    for (what, r) in [
        ("grub isn't the one built", other(&|r| r[4][3][0] ^= 1)),
        (
            "shim and grub in the other order",
            other(&|r| r[4].swap(2, 3)),
        ),
        (
            "one entry fewer than the boot",
            other(&|r| {
                r[9].pop();
            }),
        ),
        (
            "one more entry than the boot",
            other(&|r| r[9].push(boot::later().sha256())),
        ),
        (
            "the first entry missing",
            other(&|r| {
                r[7].remove(0);
            }),
        ),
        (
            "an entry first that the boot doesn't have",
            other(&|r| r[0].insert(0, [0; 32])),
        ),
        (
            "no entries for the kernel command line",
            other(&|r| r[8].clear()),
        ),
    ] {
        assert_eq!(
            check_log(&booted, &q.pcrs(), &held(&r)),
            Ok(false),
            "{what}"
        );
    }
}

/// Comparing with the last check alone would pass a computer that was already wrong when it was
/// left (`docs/fab.md`, "Reference values"): untouched since, it is red against the reference.
#[test]
fn a_computer_already_wrong_when_it_was_left_is_red() {
    let t = quotes();
    let boot = boot::boot();
    let booted = log(&boot);
    let mut r = reference(&boot);
    r[4][3][0] ^= 1; // the grub that was built isn't the one that booted
    let list = enrolled(&t, &r);
    let (a, b) = (t.get("in"), t.get("other"));
    let red = Verdict::Red(Red::BootChainChanged);
    assert_eq!(
        judge(&list[0].snapshot, &checked(&t.ak_a, a).unwrap(), true),
        Verdict::Green,
        "as it was left"
    );
    assert_eq!(
        verdicts(&list, &[answer(1, a, &booted), answer(2, b, &booted)]),
        [red, red]
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

/// `ours` is whether the event log found PCRs 0 to 9 the reference's.
#[test]
fn the_findings_table_in_its_order() {
    let out = quoted(5, 0, 1000, 7, 0);
    let red = |r| Verdict::Red(r);
    let (ours, not) = (true, false);
    for (now, boot, verdict) in [
        (quoted(5, 0, 2000, 7, 0), ours, Verdict::Green),
        (quoted(5, 0, 1000, 7, 0), ours, Verdict::Green),
        (quoted(6, 0, 2000, 7, 0), ours, red(Red::Restarted)),
        (quoted(5, 1, 2000, 7, 0), ours, red(Red::Restarted)),
        (quoted(5, 0, 2000, 7, 1), ours, red(Red::BootChainChanged)),
        (quoted(5, 0, 2000, 7, 0), not, red(Red::BootChainChanged)),
        (quoted(5, 0, 2000, 8, 0), ours, red(Red::TpmFirmwareChanged)),
        (quoted(5, 0, 999, 7, 0), ours, Verdict::Amber),
        (quoted(6, 0, 999, 8, 1), not, red(Red::Restarted)),
        (quoted(5, 0, 999, 8, 1), ours, red(Red::BootChainChanged)),
        (quoted(5, 0, 999, 8, 0), not, red(Red::BootChainChanged)),
        (quoted(5, 0, 999, 8, 0), ours, red(Red::TpmFirmwareChanged)),
    ] {
        assert_eq!(judge(&out, &now, boot), verdict, "{now:?}, ours: {boot}");
    }
}

/// Both computers booted the same software, so they have one reference.
fn enrolled<'a>(t: &Quotes, r: &'a [Vec<Digest>]) -> [Enrolled<'a>; 2] {
    [
        Enrolled {
            id: 1,
            key: t.ak_a,
            snapshot: checked(&t.ak_a, t.get("out")).unwrap(),
            reference: held(r),
            challenge: t.get("in").challenge,
        },
        Enrolled {
            id: 2,
            key: t.ak_b,
            snapshot: checked(&t.ak_b, t.get("other")).unwrap(),
            reference: held(r),
            challenge: t.get("other").challenge,
        },
    ]
}

fn answer<'a>(id: u16, q: &'a Quote, log: &'a [u8]) -> Answer<'a> {
    Answer {
        id,
        attest: &q.attest,
        signature: &q.signature,
        pcrs: q.pcrs(),
        log,
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
    let boot = boot::boot();
    let r = reference(&boot);
    let list = enrolled(&t, &r);
    let (a, b) = (t.get("in"), t.get("other"));
    let booted = log(&boot);
    let mut then = boot.clone();
    then.push(boot::later());
    let longer = log(&then);
    let missing = Verdict::Red(Red::Missing(None));
    assert_eq!(
        verdicts(&list, &[answer(1, a, &booted), answer(2, b, &booted)]),
        [Verdict::Green, Verdict::Green]
    );
    assert_eq!(
        verdicts(&list, &[answer(2, b, &booted), answer(1, a, &booted)]),
        [Verdict::Green, Verdict::Green],
        "in any order"
    );
    assert_eq!(
        verdicts(&list, &[answer(1, a, &booted)]),
        [Verdict::Green, missing],
        "the hub leaves one out"
    );
    assert_eq!(
        verdicts(&list, &[]),
        [missing, missing],
        "the hub sends nothing"
    );
    assert_eq!(
        verdicts(
            &list,
            &[
                answer(1, a, &booted),
                answer(1, a, &booted),
                answer(2, b, &booted)
            ]
        ),
        [missing, Verdict::Green],
        "two answers for one computer"
    );
    assert_eq!(
        verdicts(
            &list,
            &[
                answer(1, a, &booted),
                answer(2, b, &booted),
                answer(99, a, &booted)
            ]
        ),
        [Verdict::Green, Verdict::Green],
        "an answer for a computer not enrolled"
    );
    assert_eq!(
        verdicts(&list, &[answer(1, b, &booted), answer(2, b, &booted)]),
        [
            Verdict::Red(Red::Missing(Some(Refused::Signature))),
            Verdict::Green
        ],
        "another TPM's quote under the first computer's number"
    );
    assert_eq!(
        verdicts(&list, &[answer(1, a, &longer), answer(2, b, &booted)]),
        [
            Verdict::Red(Red::Missing(Some(Refused::EventLog))),
            Verdict::Green
        ],
        "a log that doesn't replay to its quote"
    );
}
