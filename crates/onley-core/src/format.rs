//! Renders values for people.

/// Renders `n` in binary units with one decimal place past a kilobyte.
pub fn bytes(n: i64) -> String {
    const UNIT: i64 = 1024;
    if n < UNIT {
        return format!("{n} B");
    }
    let mut div: i64 = UNIT;
    let mut exp = 0usize;
    let mut v = n / UNIT;
    while v >= UNIT {
        div *= UNIT;
        exp += 1;
        v /= UNIT;
    }
    let units = ["K", "M", "G", "T", "P", "E"];
    format!("{:.1} {}B", n as f64 / div as f64, units[exp])
}

#[cfg(test)]
mod tests {
    use super::bytes;

    #[test]
    fn below_a_kilobyte_is_bytes() {
        assert_eq!(bytes(0), "0 B");
        assert_eq!(bytes(1023), "1023 B");
    }

    #[test]
    fn kilobytes_and_megabytes() {
        assert_eq!(bytes(1024), "1.0 KB");
        assert_eq!(bytes(1536), "1.5 KB");
        assert_eq!(bytes(1024 * 1024), "1.0 MB");
        assert_eq!(bytes(3 * 1024 * 1024 + 512 * 1024), "3.5 MB");
    }
}
