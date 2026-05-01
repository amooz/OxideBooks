/// Monetary amount stored as integer minor units (e.g. cents for USD).
///
/// Using integers avoids floating-point rounding errors in financial calculations.
/// All arithmetic must be done at this scale; format only at display time.
pub type MinorUnits = i64;

/// Format minor units for display given an ISO 4217 currency code.
pub fn format_amount(amount: MinorUnits, currency: &str) -> String {
    let divisor = minor_unit_divisor(currency);
    let symbol = currency_symbol(currency);
    if divisor == 1 {
        return format!("{}{}", symbol, amount);
    }
    let whole = amount / divisor;
    let frac = (amount % divisor).abs();
    let decimals = (divisor as f64).log10() as usize;
    format!("{}{}.{:0>width$}", symbol, whole, frac, width = decimals)
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

    // ── Standard two-decimal currencies ───────────────────────────────────────

    #[test]
    fn format_usd() {
        assert_eq!(format_amount(10_050, "USD"), "$100.50");
    }

    #[test]
    fn format_usd_zero_cents() {
        assert_eq!(format_amount(10_000, "USD"), "$100.00");
    }

    #[test]
    fn format_usd_only_cents() {
        assert_eq!(format_amount(99, "USD"), "$0.99");
    }

    #[test]
    fn format_eur() {
        assert_eq!(format_amount(2_550, "EUR"), "€25.50");
    }

    #[test]
    fn format_gbp() {
        assert_eq!(format_amount(1_000, "GBP"), "£10.00");
    }

    #[test]
    fn format_unknown_two_decimal() {
        // Unknown currency uses the code itself as symbol.
        assert_eq!(format_amount(1_050, "CAD"), "CAD10.50");
    }

    // ── Zero-decimal currencies ───────────────────────────────────────────────

    #[test]
    fn format_jpy_whole() {
        assert_eq!(format_amount(1_000, "JPY"), "¥1000");
    }

    #[test]
    fn format_krw() {
        assert_eq!(format_amount(50_000, "KRW"), "KRW50000");
    }

    // ── Three-decimal currencies ───────────────────────────────────────────────

    #[test]
    fn format_kwd() {
        // 1.500 KWD = 1500 minor units
        assert_eq!(format_amount(1_500, "KWD"), "KWD1.500");
    }

    // ── Negative amounts ──────────────────────────────────────────────────────

    #[test]
    fn format_negative_usd() {
        // Negative amounts are supported (e.g. credit notes).
        assert_eq!(format_amount(-500, "USD"), "$-5.00");
    }

    // ── Zero ─────────────────────────────────────────────────────────────────

    #[test]
    fn format_zero() {
        assert_eq!(format_amount(0, "USD"), "$0.00");
    }
}
