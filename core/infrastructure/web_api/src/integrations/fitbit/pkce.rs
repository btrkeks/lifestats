use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use sha2::{Digest, Sha256};

/// Generate a PKCE code verifier and challenge pair according to RFC 7636
///
/// Returns: (code_verifier, code_challenge)
///
/// - code_verifier: 43-128 character URL-safe string
/// - code_challenge: base64url(SHA256(code_verifier))
pub fn generate_pkce_pair() -> (String, String) {
    // Generate 32 random bytes (will produce 43 chars when base64url encoded)
    let random_bytes: [u8; 32] = rand::random();

    // Encode as base64url (no padding) to create the verifier
    let code_verifier = URL_SAFE_NO_PAD.encode(random_bytes);

    // Compute SHA256 hash of the verifier
    let mut hasher = Sha256::new();
    hasher.update(code_verifier.as_bytes());
    let hash = hasher.finalize();

    // Encode the hash as base64url (no padding) to create the challenge
    let code_challenge = URL_SAFE_NO_PAD.encode(hash);

    (code_verifier, code_challenge)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pkce_verifier_length() {
        let (verifier, _) = generate_pkce_pair();
        // 32 bytes encoded in base64url produces 43 characters
        assert_eq!(verifier.len(), 43);
    }

    #[test]
    fn test_pkce_challenge_length() {
        let (_, challenge) = generate_pkce_pair();
        // SHA256 produces 32 bytes, which encodes to 43 characters in base64url
        assert_eq!(challenge.len(), 43);
    }

    #[test]
    fn test_pkce_verifier_is_url_safe() {
        let (verifier, _) = generate_pkce_pair();
        // Should only contain base64url characters: A-Z, a-z, 0-9, -, _
        assert!(verifier
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_'));
    }

    #[test]
    fn test_pkce_challenge_is_url_safe() {
        let (_, challenge) = generate_pkce_pair();
        // Should only contain base64url characters
        assert!(challenge
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_'));
    }

    #[test]
    fn test_pkce_pairs_are_unique() {
        let (verifier1, challenge1) = generate_pkce_pair();
        let (verifier2, challenge2) = generate_pkce_pair();

        // Different calls should produce different values
        assert_ne!(verifier1, verifier2);
        assert_ne!(challenge1, challenge2);
    }

    #[test]
    fn test_pkce_challenge_deterministic_from_verifier() {
        let (verifier, challenge1) = generate_pkce_pair();

        // Manually compute challenge from verifier
        let mut hasher = Sha256::new();
        hasher.update(verifier.as_bytes());
        let hash = hasher.finalize();
        let challenge2 = URL_SAFE_NO_PAD.encode(hash);

        // Should match the generated challenge
        assert_eq!(challenge1, challenge2);
    }
}
