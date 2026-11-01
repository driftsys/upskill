use super::*;
use anyhow::{Context, Result, ensure};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub(super) type Blobs = BTreeMap<(EvidenceKind, String), EvidenceBlob>;
fn reference(kind: EvidenceKind, bytes: &[u8]) -> Result<EvidenceReference> {
    let tag = match kind {
        EvidenceKind::NativePrestate => "native-prestate",
        EvidenceKind::ConfigurationSnapshot => "configuration-snapshot",
    };
    let length = u64::try_from(bytes.len()).context("private evidence length")?;
    let mut hash = Sha256::new();
    hash.update(b"upskill:native-evidence:v1\0");
    hash.update((tag.len() as u64).to_be_bytes());
    hash.update(tag.as_bytes());
    hash.update(length.to_be_bytes());
    hash.update(bytes);
    Ok(EvidenceReference {
        kind,
        digest: hash.finalize().iter().map(|b| format!("{b:02x}")).collect(),
        length,
    })
}
pub(super) fn add(
    blobs: &mut Blobs,
    kind: EvidenceKind,
    bytes: Vec<u8>,
) -> Result<EvidenceReference> {
    let reference = reference(kind, &bytes)?;
    let key = (reference.kind.clone(), reference.digest.clone());
    if let Some(previous) = blobs.get(&key) {
        ensure!(
            previous.bytes == bytes,
            "conflicting private evidence digest"
        );
    }
    blobs.insert(
        key,
        EvidenceBlob {
            reference: reference.clone(),
            bytes,
        },
    );
    Ok(reference)
}
pub(super) fn index(blobs: &[EvidenceBlob]) -> Result<Blobs> {
    let mut indexed = Blobs::new();
    for blob in blobs {
        ensure!(
            reference(blob.reference.kind.clone(), &blob.bytes)? == blob.reference,
            "private evidence kind, digest or length differs from bytes"
        );
        ensure!(
            indexed
                .insert(
                    (blob.reference.kind.clone(), blob.reference.digest.clone()),
                    blob.clone()
                )
                .is_none(),
            "duplicate private evidence"
        );
    }
    Ok(indexed)
}
pub(super) fn resolve<'a>(
    blobs: &'a Blobs,
    reference: &EvidenceReference,
    kind: EvidenceKind,
) -> Result<&'a [u8]> {
    ensure!(
        reference.kind == kind,
        "private evidence has the wrong kind"
    );
    let blob = blobs
        .get(&(reference.kind.clone(), reference.digest.clone()))
        .context("required private evidence is missing")?;
    ensure!(
        blob.reference == *reference,
        "private evidence reference differs"
    );
    Ok(&blob.bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evidence_protocol_pins_kind_framing_and_binary_bytes() {
        // Independently calculated with Python hashlib and big-endian u64 fields.
        for (kind, expected) in [
            (
                EvidenceKind::NativePrestate,
                "5eadfd350ace3da982d31480c7aa2f9f5d3b9fda1838b35196e9068c83f969bc",
            ),
            (
                EvidenceKind::ConfigurationSnapshot,
                "f62d66ed31f531d97ca03fe1e2a675e57581ebbb90024c8e1235fc4754c7b18e",
            ),
        ] {
            let result = reference(kind.clone(), &[0, 255, 1]).unwrap();
            assert_eq!(result.kind, kind);
            assert_eq!(result.length, 3);
            assert_eq!(result.digest, expected);
        }
    }
}
