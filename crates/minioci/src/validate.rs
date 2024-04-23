use anyhow::{anyhow, Result};
use regex::Regex;

const VALID_DIGEST_REGEX: &str = r"^(sha256:[a-f0-9]{64})|(sha512:[a-f0-9]{128})$";

pub fn check_valid_digest(digest: &str) -> Result<()> {
    let regex = Regex::new(VALID_DIGEST_REGEX)?;
    if !regex.is_match(digest) {
        return Err(anyhow!("invalid digest specified"));
    }
    Ok(())
}
