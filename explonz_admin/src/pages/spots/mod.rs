pub mod addition;
pub mod detail;
pub mod edit;
pub mod list;
pub mod seasonal_picks;
pub mod seasonal_picks_list;

pub const MONTHS: [(&str, u8); 12] = [
    ("Jan", 1),
    ("Feb", 2),
    ("Mar", 3),
    ("Apr", 4),
    ("May", 5),
    ("Jun", 6),
    ("Jul", 7),
    ("Aug", 8),
    ("Sep", 9),
    ("Oct", 10),
    ("Nov", 11),
    ("Dec", 12),
];

pub fn month_short(m: i16) -> &'static str {
    MONTHS
        .iter()
        .find(|(_, n)| *n as i16 == m)
        .map(|(name, _)| *name)
        .unwrap_or("?")
}
