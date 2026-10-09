use super::PrivacyError;
use argon2::{
    Algorithm, Argon2, Params, PasswordHash, PasswordHasher, PasswordVerifier, Version,
    password_hash::SaltString,
};
use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PinCredential {
    encoded: String,
}
impl std::fmt::Debug for PinCredential {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PinCredential([redacted])")
    }
}
impl PinCredential {
    pub fn valid_pin(pin: &str) -> bool {
        pin.len() == 4 && pin.bytes().all(|b| b.is_ascii_digit())
    }

    pub fn create(pin: &str) -> Result<Self, PrivacyError> {
        if !Self::valid_pin(pin) {
            return Err(PrivacyError::InvalidPin);
        }
        let mut bytes = [0_u8; 16];
        OsRng.try_fill_bytes(&mut bytes).map_err(|_| PrivacyError::Persistence)?;
        let salt = SaltString::encode_b64(&bytes).map_err(|_| PrivacyError::Persistence)?;
        let hash = Self::hasher()
            .hash_password(pin.as_bytes(), &salt)
            .map_err(|_| PrivacyError::Persistence)?;
        Ok(Self { encoded: hash.to_string() })
    }

    pub fn verify(&self, pin: &str) -> Result<bool, PrivacyError> {
        if !Self::valid_pin(pin) {
            return Err(PrivacyError::InvalidPin);
        }
        self.validate()?;
        let hash = PasswordHash::new(&self.encoded).map_err(|_| PrivacyError::CorruptConfig)?;
        match Self::hasher().verify_password(pin.as_bytes(), &hash) {
            Ok(()) => Ok(true),
            Err(argon2::password_hash::Error::Password) => Ok(false),
            Err(_) => Err(PrivacyError::CorruptConfig),
        }
    }

    pub(crate) fn validate(&self) -> Result<(), PrivacyError> {
        if self.encoded.len() > 256 {
            return Err(PrivacyError::CorruptConfig);
        }
        let hash = PasswordHash::new(&self.encoded).map_err(|_| PrivacyError::CorruptConfig)?;
        let mut salt = [0_u8; 64];
        let salt_len =
            hash.salt.and_then(|value| value.decode_b64(&mut salt).ok()).map(|value| value.len());
        // Only our bounded work factors are accepted. Configuration is untrusted
        // input and must not select enormous memory/CPU costs during verification.
        if hash.algorithm.as_str() != "argon2id"
            || hash.version != Some(19)
            || hash.params.iter().count() != 3
            || hash.params.get_decimal("m") != Some(19456)
            || hash.params.get_decimal("t") != Some(2)
            || hash.params.get_decimal("p") != Some(1)
            || salt_len != Some(16)
            || hash.hash.as_ref().map(|value| value.len()) != Some(32)
        {
            return Err(PrivacyError::CorruptConfig);
        }
        Ok(())
    }

    fn hasher() -> Argon2<'static> {
        Argon2::new(
            Algorithm::Argon2id,
            Version::V0x13,
            Params::new(19456, 2, 1, Some(32)).expect("fixed Argon2 parameters"),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn real_argon2_keeps_leading_zero_and_uses_unique_salt() {
        let a = PinCredential::create("0123").unwrap();
        let b = PinCredential::create("0123").unwrap();
        assert!(a.verify("0123").unwrap());
        assert!(!a.verify("1230").unwrap());
        assert!(a.encoded != b.encoded, "credentials must have independent random salts");
        assert!(a.validate().is_ok());
        let debug = format!("{a:?}");
        assert!(!debug.contains("0123"));
        assert!(!debug.contains(&a.encoded));
    }
    #[test]
    fn malformed_pins_are_rejected_without_trimming_or_unicode_digits() {
        for pin in ["", "123", "12345", " 123", "123 ", "１２３４", "١٢٣٤", "12a4", "123\n"]
        {
            assert!(PinCredential::create(pin).is_err());
        }
    }
    #[test]
    fn imported_hash_cannot_choose_unbounded_work_or_another_algorithm() {
        let a = PinCredential::create("0123").unwrap();
        for bad in [
            a.encoded.replace("argon2id", "argon2i"),
            a.encoded.replace("m=19456", "m=4294967295"),
            a.encoded.replace("t=2", "t=1000000"),
            "not a hash".into(),
        ] {
            let cred = PinCredential { encoded: bad };
            assert!(cred.validate().is_err());
            assert!(cred.verify("0123").is_err());
        }
    }
}
