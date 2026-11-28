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
//! - A checked quote is judged against the snapshot by the findings table, in its order: a changed
//!   reset or restart count is red, a changed PCR is red, a changed TPM firmware version is red, a
//!   clock gone backwards is amber, and anything else is green ([`judge`]).
//! - Every computer the verifier enrolled is judged, from its own list, whatever the hub sends: one
//!   with no answer is red, and so is one with more than one ([`check_in`]).
//!
//! Reference values (step 1b) and IMA (step 1c) are not here yet.

#![cfg_attr(not(test), no_std)]

use p256::ecdsa::{Signature, VerifyingKey, signature::hazmat::PrehashVerifier};
use sha2::{Digest as _, Sha256};
use tpm2_protocol::TpmUnmarshal;
use tpm2_protocol::data::{TpmAlgId, TpmsAttest, TpmtSignature, TpmuAttest, TpmuSignature};

/// PCRs 0 to 10: the firmware's, the boot loader's and the kernel's (0 to 9), and IMA's (10).
pub const PCRS: usize = 11;

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

/// A finding that shows red.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Red {
    /// The reset or restart count changed: the computer restarted.
    Restarted,
    /// A PCR changed.
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

/// Judges a checked quote against the snapshot taken when the fab was left, by the findings table
/// in its order.
#[must_use]
pub fn judge(snapshot: &Quoted, now: &Quoted) -> Verdict {
    if now.reset_count != snapshot.reset_count || now.restart_count != snapshot.restart_count {
        Verdict::Red(Red::Restarted)
    } else if now.pcrs != snapshot.pcrs {
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
pub struct Enrolled {
    /// The number the verifier gave it at enrollment.
    pub id: u16,
    pub key: VerifyingKey,
    pub snapshot: Quoted,
    /// The challenge sent to it for this check.
    pub challenge: [u8; 32],
}

/// One answer the hub passed on, for the computer it names.
pub struct Answer<'a> {
    pub id: u16,
    pub attest: &'a [u8],
    pub signature: &'a [u8],
    pub pcrs: [Digest; PCRS],
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
            ) {
                Ok(now) => judge(&computer.snapshot, &now),
                Err(why) => Verdict::Red(Red::Missing(Some(why))),
            },
            _ => Verdict::Red(Red::Missing(None)),
        };
    }
}
