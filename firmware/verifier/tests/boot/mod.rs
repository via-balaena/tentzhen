//! The boot the verifier's tests stand for: what a computer's firmware and boot loader measure, in
//! order, as the entries of its TCG event log. `examples/swtpm_quotes.rs` extends each entry into
//! swtpm before it quotes, and `tests/judge.rs` writes the log of these entries, and changes it as
//! a hub could.
//!
//! It is made up: each entry's name stands for a part of a boot through shim and GRUB, each SHA-256
//! digest is of that name, not of a file, and the SHA-1 digests are 20 made-up bytes (the verifier
//! reads only SHA-256). Which PCRs a real boot uses, and a real firmware's log, come in step 3, from
//! OVMF under QEMU.

use sha2::{Digest as _, Sha256};

pub const ALG_SHA1: u16 = 0x0004;
pub const ALG_SHA256: u16 = 0x000B;

/// Event types, from the PC Client Platform Firmware Profile 1.05 r23, Table 14. Only
/// `EV_NO_ACTION` means anything to the verifier: such an entry is never extended (§10.4.5).
pub const EV_NO_ACTION: u32 = 0x0000_0003;
const EV_SEPARATOR: u32 = 0x0000_0004;
const EV_S_CRTM_VERSION: u32 = 0x0000_0008;
const EV_IPL: u32 = 0x0000_000D;
const EV_EFI_VARIABLE_DRIVER_CONFIG: u32 = 0x8000_0001;
const EV_EFI_BOOT_SERVICES_APPLICATION: u32 = 0x8000_0003;
const EV_EFI_ACTION: u32 = 0x8000_0007;

/// One entry of the log.
#[derive(Clone)]
pub struct Entry {
    pub pcr: u32,
    pub kind: u32,
    /// Each bank's digest: the algorithm, and the bytes extended into that bank.
    pub digests: Vec<(u16, Vec<u8>)>,
    /// What the entry says it measured. No PCR holds it.
    pub data: Vec<u8>,
}

impl Entry {
    /// An entry the firmware extends: `what`'s SHA-256, and 20 made-up bytes for the SHA-1 bank.
    pub fn measured(pcr: u32, kind: u32, what: &str) -> Entry {
        let sha1 = Sha256::digest(format!("SHA-1 bank: {what}"))[..20].to_vec();
        Entry {
            pcr,
            kind,
            digests: vec![
                (ALG_SHA1, sha1),
                (ALG_SHA256, Sha256::digest(what).to_vec()),
            ],
            data: what.as_bytes().to_vec(),
        }
    }

    /// An entry that is only recorded: zeros for each bank's digest (§10.4.5, rule 3).
    pub fn not_extended(data: &[u8]) -> Entry {
        Entry {
            pcr: 0,
            kind: EV_NO_ACTION,
            digests: vec![(ALG_SHA1, vec![0; 20]), (ALG_SHA256, vec![0; 32])],
            data: data.to_vec(),
        }
    }

    /// Whether the firmware extends it into its PCR, as swtpm is made to.
    pub fn extended(&self) -> bool {
        self.kind != EV_NO_ACTION
    }

    /// The SHA-256 digest the entry extends.
    pub fn sha256(&self) -> [u8; 32] {
        let (_, d) = self
            .digests
            .iter()
            .find(|(alg, _)| *alg == ALG_SHA256)
            .expect("a SHA-256 digest");
        d.as_slice().try_into().unwrap()
    }
}

/// The boot, from the firmware to the initramfs. It ends with an entry for PCR 9, so every log cut
/// short loses an entry the quote holds.
pub fn boot() -> Vec<Entry> {
    let mut v = vec![
        // A vendor's event that is only recorded; its data starts with a 16-byte signature (§10.1).
        Entry::not_extended(b"Tentzhen tests\0\0 vendor data"),
        Entry::measured(0, EV_S_CRTM_VERSION, "firmware version"),
        Entry::measured(7, EV_EFI_VARIABLE_DRIVER_CONFIG, "SecureBoot"),
        Entry::measured(7, EV_EFI_VARIABLE_DRIVER_CONFIG, "PK"),
        Entry::measured(7, EV_EFI_VARIABLE_DRIVER_CONFIG, "KEK"),
        Entry::measured(7, EV_EFI_VARIABLE_DRIVER_CONFIG, "db"),
        Entry::measured(7, EV_EFI_VARIABLE_DRIVER_CONFIG, "dbx"),
        Entry::measured(4, EV_EFI_ACTION, "Calling EFI Application from Boot Option"),
    ];
    for pcr in 0..=7 {
        v.push(Entry::measured(
            pcr,
            EV_SEPARATOR,
            &format!("separator {pcr}"),
        ));
    }
    v.extend([
        Entry::measured(4, EV_EFI_BOOT_SERVICES_APPLICATION, "shim"),
        // An entry for PCR 14, which the quote doesn't cover.
        Entry::measured(14, EV_IPL, "MokList"),
        Entry::measured(4, EV_EFI_BOOT_SERVICES_APPLICATION, "grub"),
        Entry::measured(8, EV_IPL, "grub_cmd: linux /vmlinuz root=UUID=fab ro"),
        Entry::measured(9, EV_IPL, "/vmlinuz"),
        Entry::measured(9, EV_IPL, "/initramfs.img"),
    ]);
    v
}

/// What is measured into PCR 9 after the boot, before the quote named `extended`.
pub fn later() -> Entry {
    Entry::measured(9, EV_IPL, "/boot/read-after-the-boot.cfg")
}
