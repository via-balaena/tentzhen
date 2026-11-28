//! The fab verifier's judge (`docs/fab.md`, "Security"): each computer's TPM quote, checked
//! against the key it enrolled and the challenge the verifier sent, then compared with the snapshot
//! taken when the fab was left. There is no hardware and no OS here. The verifier's firmware on
//! Tock is to give it the hub's answers and show what it returns.
//!
//! The rules, step 1a of "Getting to v1.0":
//!
//! - A quote is refused unless the computer's enrolled key signed it (ECDSA P-256 over SHA-256),
//!   it is a TPM's quote (`TPM_GENERATED_VALUE`, `TPM_ST_ATTEST_QUOTE`), it carries the challenge
//!   just sent, it covers exactly the SHA-256 PCRs 0 to 10, and the PCR values the hub passed on
//!   hash to the digest inside it ([`check_quote`]).
//! - The boot event log the hub passes on is refused unless it is a TCG crypto-agile log that
//!   replays to the quoted PCRs 0 to 9; then each of those PCRs is the reference's if its entries
//!   are the reference's digests, in order, none missing and none more ([`check_log`], step 1b).
//! - A checked quote is judged against the snapshot by the findings table, in its order: a changed
//!   reset or restart count is red, a changed PCR, or a PCR from 0 to 9 that isn't the reference's,
//!   is red, a changed TPM firmware version is red, a clock gone backwards is amber, and anything
//!   else is green ([`judge`]).
//! - Every computer the verifier enrolled is judged, from its own list, whatever the hub sends: one
//!   with no answer is red, and so is one with more than one ([`check_in`]).
//!
//! IMA (step 1c) is not here yet.

#![cfg_attr(not(test), no_std)]

use p256::ecdsa::{Signature, VerifyingKey, signature::hazmat::PrehashVerifier};
use sha2::{Digest as _, Sha256};
use tpm2_protocol::TpmUnmarshal;
use tpm2_protocol::data::{TpmAlgId, TpmsAttest, TpmtSignature, TpmuAttest, TpmuSignature};

/// PCRs 0 to 10: the firmware's, the boot loader's and the kernel's (0 to 9), and IMA's (10).
pub const PCRS: usize = 11;

/// PCRs 0 to 9, the ones the boot event log replays.
pub const BOOT_PCRS: usize = 10;

/// A SHA-256 digest.
pub type Digest = [u8; 32];

/// The PCR selection every quote must carry: the SHA-256 bank, PCRs 0 to 10, as three bytes.
const SELECTION: [u8; 3] = [0xFF, 0x07, 0x00];

/// Why a quote is refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refused {
    /// The signature isn't a SHA-256 ECDSA signature by the computer's enrolled key over the quote.
    Signature,
    /// The signed bytes aren't a TPM's quote.
    NotAQuote,
    /// The quote carries some other challenge than the one just sent.
    Challenge,
    /// The quote covers other PCRs, or another bank, than SHA-256 PCRs 0 to 10.
    Selection,
    /// The PCR values passed on don't hash to the digest the quote signs.
    PcrValues,
    /// The boot event log isn't a TCG crypto-agile log the verifier reads, or doesn't replay to the
    /// quoted PCRs 0 to 9.
    EventLog,
}

/// What a checked quote says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Quoted {
    pub reset_count: u32,
    pub restart_count: u32,
    /// Milliseconds the TPM has been powered, as it counts them.
    pub clock: u64,
    pub firmware_version: u64,
    pub pcrs: [Digest; PCRS],
}

/// Checks one quote. `attest` is the TPMS_ATTEST the TPM signed, `signature` its TPMT_SIGNATURE,
/// and `pcrs` the values of PCRs 0 to 10 the hub passed on with them.
///
/// # Errors
///
/// Returns why the quote is refused, by the first rule it breaks in the module's order.
pub fn check_quote(
    key: &VerifyingKey,
    challenge: &[u8; 32],
    attest: &[u8],
    signature: &[u8],
    pcrs: &[Digest; PCRS],
) -> Result<Quoted, Refused> {
    let signature = ecdsa_sha256(signature).ok_or(Refused::Signature)?;
    key.verify_prehash(&Sha256::digest(attest), &signature)
        .map_err(|_| Refused::Signature)?;

    // tpm2-protocol checks the magic, and picks the body's variant by the type, so a body that is
    // a quote's is a TPM_ST_ATTEST_QUOTE.
    let (attest, rest) = TpmsAttest::unmarshal(attest).map_err(|_| Refused::NotAQuote)?;
    let TpmuAttest::Quote(quote) = &attest.attested else {
        return Err(Refused::NotAQuote);
    };
    if !rest.is_empty() {
        return Err(Refused::NotAQuote);
    }
    if &attest.extra_data[..] != challenge {
        return Err(Refused::Challenge);
    }
    let selection = &quote.pcr_select[..];
    if selection.len() != 1
        || selection[0].hash != TpmAlgId::Sha256
        || selection[0].pcr_select[..] != SELECTION
    {
        return Err(Refused::Selection);
    }
    let mut hash = Sha256::new();
    for value in pcrs {
        hash.update(value);
    }
    if hash.finalize()[..] != quote.pcr_digest[..] {
        return Err(Refused::PcrValues);
    }

    let clock = attest.clock_info;
    Ok(Quoted {
        reset_count: clock.reset_count.into(),
        restart_count: clock.restart_count.into(),
        clock: clock.clock.into(),
        firmware_version: attest.firmware_version.into(),
        pcrs: *pcrs,
    })
}

/// Reads a TPMT_SIGNATURE as an ECDSA signature with SHA-256, the only kind an enrolled key makes.
fn ecdsa_sha256(bytes: &[u8]) -> Option<Signature> {
    // As for the quote's body, the signature's variant is picked by its algorithm.
    let (signature, rest) = TpmtSignature::unmarshal(bytes).ok()?;
    let TpmuSignature::Ecdsa(ecc) = signature.signature else {
        return None;
    };
    if ecc.hash != TpmAlgId::Sha256 || !rest.is_empty() {
        return None;
    }
    Signature::from_scalars(scalar(&ecc.signature_r)?, scalar(&ecc.signature_s)?).ok()
}

/// A big-endian scalar of at most 32 bytes, padded on the left to 32.
fn scalar(bytes: &[u8]) -> Option<[u8; 32]> {
    let mut out = [0u8; 32];
    out.get_mut(32usize.checked_sub(bytes.len())?..)?
        .copy_from_slice(bytes);
    Some(out)
}

/// What a computer's boot should extend into each of PCRs 0 to 9: the SHA-256 digests, in order.
pub type Reference<'a> = [&'a [Digest]; BOOT_PCRS];

/// The event log's types and layout: the PC Client Platform Firmware Profile 1.05 r23, §10.
const EV_NO_ACTION: u32 = 0x0000_0003;
const SPEC_ID: &[u8; 16] = b"Spec ID Event03\0";
const ALG_SHA256: u16 = 0x000B;

/// Checks a boot event log, as the hub passed it on, against the quoted PCRs, and returns whether
/// PCRs 0 to 9 are all the reference's.
///
/// Of each entry, the answer rests on its PCR, whether its type is `EV_NO_ACTION` (never extended,
/// §10.4.5), and its SHA-256 digest; the rest need only be readable, and what an entry says it
/// measured is held by no PCR. Entries for PCRs 10 and up aren't this check's. PCR 0 is replayed
/// from zeros: the Startup Locality event, which a reader needs to start PCR 0 right (§10.4.5.3),
/// isn't read, so the log of a TPM started from locality 3 is refused.
///
/// With the reference an exact list for each PCR, the answer is whether the reference, replayed
/// from zeros, gives the quoted PCRs: the log's entries add only the refusal.
///
/// # Errors
///
/// `Refused::EventLog`, if the log can't be read or doesn't replay to `pcrs`.
pub fn check_log(
    log: &[u8],
    pcrs: &[Digest; PCRS],
    reference: &Reference,
) -> Result<bool, Refused> {
    match replay(log, reference) {
        Some((replayed, ours)) if replayed[..] == pcrs[..BOOT_PCRS] => Ok(ours),
        _ => Err(Refused::EventLog),
    }
}

/// The log's SHA-256 digests extended into PCRs 0 to 9 from zeros, and whether each PCR's entries
/// are the reference's; `None` if the log can't be read.
fn replay(log: &[u8], reference: &Reference) -> Option<([Digest; BOOT_PCRS], bool)> {
    let mut log = Bytes(log);
    // The first entry is in the SHA-1 log's form (§10.2.1), and its event is the Spec ID event,
    // which gives each algorithm's digest size (§10.4.5.1). Its PCR, type and digest aren't read.
    log.take(4 + 4 + 20)?;
    let mut spec = Bytes(log.sized()?);
    if spec.take(16)? != SPEC_ID {
        return None;
    }
    spec.take(4 + 4)?; // platformClass, the spec's version and errata, uintnSize
    let algorithms = spec.u32()?;
    let sizes = spec.take(usize::try_from(algorithms).ok()?.checked_mul(4)?)?;

    let mut pcrs = [[0u8; 32]; BOOT_PCRS];
    let mut seen = [0usize; BOOT_PCRS];
    let mut ours = true;
    while !log.0.is_empty() {
        // A TCG_PCR_EVENT2 (§10.2.2).
        let pcr = log.u32()?;
        let kind = log.u32()?;
        let mut sha256 = None;
        for _ in 0..log.u32()? {
            let algorithm = log.u16()?;
            let digest = log.take(digest_size(sizes, algorithm)?)?;
            if algorithm == ALG_SHA256 {
                if sha256.is_some() {
                    return None;
                }
                sha256 = Some(Digest::try_from(digest).ok()?);
            }
        }
        let digest = sha256?;
        log.sized()?;
        if kind == EV_NO_ACTION || pcr >= BOOT_PCRS as u32 {
            continue;
        }
        let pcr = pcr as usize;
        let mut hash = Sha256::new();
        hash.update(pcrs[pcr]);
        hash.update(digest);
        pcrs[pcr].copy_from_slice(&hash.finalize());
        ours &= reference[pcr].get(seen[pcr]) == Some(&digest);
        seen[pcr] += 1;
    }
    ours &= seen.iter().zip(reference).all(|(n, r)| *n == r.len());
    Some((pcrs, ours))
}

/// The digest size the Spec ID event gives `algorithm`, from its list of (algorithm, size) pairs.
fn digest_size(sizes: &[u8], algorithm: u16) -> Option<usize> {
    sizes
        .chunks_exact(4)
        .find(|s| s[..2] == algorithm.to_le_bytes())
        .map(|s| usize::from(u16::from_le_bytes([s[2], s[3]])))
}

/// The hub's bytes, read little-endian (§10.1, rule 11), refusing to read past their end.
struct Bytes<'a>(&'a [u8]);

impl<'a> Bytes<'a> {
    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        let (head, tail) = self.0.split_at_checked(n)?;
        self.0 = tail;
        Some(head)
    }
    fn u16(&mut self) -> Option<u16> {
        Some(u16::from_le_bytes(self.take(2)?.try_into().ok()?))
    }
    fn u32(&mut self) -> Option<u32> {
        Some(u32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }
    /// A 32-bit size, then that many bytes.
    fn sized(&mut self) -> Option<&'a [u8]> {
        let n = self.u32()?;
        self.take(usize::try_from(n).ok()?)
    }
}

/// A finding that shows red.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Red {
    /// The reset or restart count changed: the computer restarted.
    Restarted,
    /// A PCR changed since the snapshot, or one from 0 to 9 isn't the reference's.
    BootChainChanged,
    /// The TPM's firmware version changed.
    TpmFirmwareChanged,
    /// No valid answer: none, more than one, or a refused quote.
    Missing(Option<Refused>),
}

/// What the verifier shows for one computer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Green,
    /// The clock went backwards: look into it.
    Amber,
    Red(Red),
}

/// Judges a checked quote against the snapshot taken when the fab was left, and `ours`, whether
/// [`check_log`] found PCRs 0 to 9 the reference's, by the findings table in its order.
#[must_use]
pub fn judge(snapshot: &Quoted, now: &Quoted, ours: bool) -> Verdict {
    if now.reset_count != snapshot.reset_count || now.restart_count != snapshot.restart_count {
        Verdict::Red(Red::Restarted)
    } else if now.pcrs != snapshot.pcrs || !ours {
        Verdict::Red(Red::BootChainChanged)
    } else if now.firmware_version != snapshot.firmware_version {
        Verdict::Red(Red::TpmFirmwareChanged)
    } else if now.clock < snapshot.clock {
        Verdict::Amber
    } else {
        Verdict::Green
    }
}

/// A computer the verifier enrolled, with what it held when the fab was left.
pub struct Enrolled<'a> {
    /// The number the verifier gave it at enrollment.
    pub id: u16,
    pub key: VerifyingKey,
    pub snapshot: Quoted,
    pub reference: Reference<'a>,
    /// The challenge sent to it for this check.
    pub challenge: [u8; 32],
}

/// One answer the hub passed on, for the computer it names.
pub struct Answer<'a> {
    pub id: u16,
    pub attest: &'a [u8],
    pub signature: &'a [u8],
    pub pcrs: [Digest; PCRS],
    /// Its boot event log.
    pub log: &'a [u8],
}

/// Judges every enrolled computer, from the verifier's own list, writing each verdict into
/// `verdicts` in the list's order. An answer naming a computer that isn't enrolled changes nothing.
///
/// # Panics
///
/// If `verdicts` is not as long as `enrolled`.
pub fn check_in(enrolled: &[Enrolled], answers: &[Answer], verdicts: &mut [Verdict]) {
    assert_eq!(enrolled.len(), verdicts.len());
    for (computer, verdict) in enrolled.iter().zip(verdicts.iter_mut()) {
        let mut mine = answers.iter().filter(|a| a.id == computer.id);
        *verdict = match (mine.next(), mine.next()) {
            (Some(a), None) => match check_quote(
                &computer.key,
                &computer.challenge,
                a.attest,
                a.signature,
                &a.pcrs,
            )
            .and_then(|now| {
                check_log(a.log, &now.pcrs, &computer.reference).map(|ours| (now, ours))
            }) {
                Ok((now, ours)) => judge(&computer.snapshot, &now, ours),
                Err(why) => Verdict::Red(Red::Missing(Some(why))),
            },
            _ => Verdict::Red(Red::Missing(None)),
        };
    }
}
