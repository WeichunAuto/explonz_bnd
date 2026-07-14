use serde::{Deserialize, Deserializer, Serialize};
use std::fmt::Display;
use std::str::FromStr;
use validator::Validate;

/// Default page number constant
const DEFAULT_PAGE_NUMBER: u64 = 1;

/// Default page size constant
const DEFAULT_PAGE_SIZE: u64 = 15;

/// Pagination parameters structure
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Validate)]
pub struct Pagination {
    /// Current page number, starting from 1
    #[validate(range(min = 1, message = "page number must be greater than 0"))]
    #[serde(
        default = "default_page_number",
        deserialize_with = "deserialize_number"
    )]
    pub page: u64,

    /// Number of items per page
    #[validate(range(
        min = 1,
        max = 100,
        message = "page size must be greater than 0 and less than 100"
    ))]
    #[serde(default = "default_page_size", deserialize_with = "deserialize_number")]
    pub size: u64,
}

/// Paginated response wrapper
/// # Type Parameters
/// - `T`: The type of items in the data collection
#[derive(Debug, Serialize)]
pub struct Page<T> {
    /// Collection of items for the current page
    pub data: Vec<T>,
    /// Total number of items across all pages
    pub total: u64,
    /// Current page number
    pub page: u64,
    /// Number of items per page
    pub size: u64,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum StringOrNumber<T> {
    String(String),
    Number(T),
}

impl<T> Page<T> {
    pub fn new(data: Vec<T>, total: u64, page: u64, size: u64) -> Self {
        Self {
            data,
            total,
            page,
            size,
        }
    }

    pub fn from_pagination(pagination: &Pagination, total: u64, data: Vec<T>) -> Self {
        Self::new(data, total, pagination.page, pagination.size)
    }
}

/// Deserializes numbers from either string or numeric JSON values
///
/// This function enables flexible input handling by accepting numbers
/// in both string format (e.g., "123") and numeric format (e.g., 123).
pub fn deserialize_number<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: FromStr + Deserialize<'de>,
    T::Err: Display,
    D: Deserializer<'de>,
{
    let string_or_number = StringOrNumber::<T>::deserialize(deserializer)?;
    match string_or_number {
        StringOrNumber::String(s) => s.parse().map_err(serde::de::Error::custom),
        StringOrNumber::Number(n) => Ok(n),
    }
}

fn default_page_number() -> u64 {
    DEFAULT_PAGE_NUMBER
}

fn default_page_size() -> u64 {
    DEFAULT_PAGE_SIZE
}

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
        let password = "123456";
        // let hashed_password = hash_password(password).unwrap();
        let hashed_password = "$2b$12$155JdL0FeO7MPgIKC3OPZuKkhPiaok0ErA4g7.XQJSdLTjzGzP.bW";
        println!("hashed_password: {}", hashed_password);
        assert!(verify_password(password, hashed_password).unwrap());
    }
}
