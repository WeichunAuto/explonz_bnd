/// Hash password with bcrypt
pub fn hash_password(password: &str) -> anyhow::Result<String> {
    Ok(bcrypt::hash(password, bcrypt::DEFAULT_COST)?)
}

/// Verify that a password is equivalent to the hash provided
pub fn verify_password(password: &str, hashed_password: &str) -> anyhow::Result<bool> {
    Ok(bcrypt::verify(password, hashed_password)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_password() {
        let password = "bobbybobby";
        let hashed_password = hash_password(password).unwrap();
        // let hashed_password = "$2b$12$155JdL0FeO7MPgIKC3OPZuKkhPiaok0/ErA4g7.XQJSdLTjzGzP.bW";
        println!("hashed_password: {}", hashed_password);
        assert!(verify_password(password, &hashed_password).unwrap());
    }
}
