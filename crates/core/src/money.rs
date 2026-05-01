/// Monetary amount stored as integer minor units (e.g. cents for USD).
///
/// Using integers avoids floating-point rounding errors in financial calculations.
/// All arithmetic must be done at this scale; format only at display time.
pub type MinorUnits = i64;

/// Format minor units for display given an ISO 4217 currency code.
pub fn format_amount(amount: MinorUnits, currency: &str) -> String {
    // Most currencies use 2 decimal places; extend as needed.
    let divisor = minor_unit_divisor(currency);
    let whole = amount / divisor;
    let frac = (amount % divisor).abs();
    let decimals = (divisor as f64).log10() as usize;
    format!("{}{}.{:0>width$}", currency_symbol(currency), whole, frac, width = decimals)
}

fn minor_unit_divisor(currency: &str) -> i64 {
    match currency {
        "JPY" | "KRW" | "VND" => 1,       // zero-decimal currencies
        "KWD" | "BHD" | "OMR" => 1_000,   // three-decimal currencies
        _ => 100,
    }
}

fn currency_symbol(currency: &str) -> &str {
    match currency {
        "USD" => "$",
        "EUR" => "€",
        "GBP" => "£",
        "JPY" => "¥",
        _ => currency,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_usd() {
        assert_eq!(format_amount(10050, "USD"), "$100.50");
    }

    #[test]
    fn format_jpy() {
        assert_eq!(format_amount(1000, "JPY"), "¥1000");
    }
}
