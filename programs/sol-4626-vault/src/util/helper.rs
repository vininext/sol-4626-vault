use crate::util::Errors;
use anchor_lang::prelude::Result;

/// Converts a deposit amount of the base asset into vault shares.
/// Assumptions:
///  - The base asset mint and the shares mint MUST have the same number of decimals.
///  - This keeps the math simple and avoids cross-decimal normalization.
///  - rounded down
pub fn convert_to_shares(deposit_amount: u64, total_assets: u64, total_shares: u64) -> Result<u64> {
    // First deposit → mint 1:1
    if total_shares == 0 {
        return Ok(deposit_amount);
    }

    if total_assets == 0 {
        return Err(Errors::DivideByZero.into());
    }

    Ok((deposit_amount as u128)
        .checked_mul(total_shares as u128)
        .ok_or(Errors::MathOverflow)?
        .checked_div(total_assets as u128)
        .ok_or(Errors::MathOverflow)?
        .try_into()
        .map_err(|_| Errors::MathOverflow)?)
}

pub fn is_valid_ticker(ticker: &[u8; 16]) -> bool {
    if ticker[0] == 0 {
        return false;
    }

    let mut seen_ticker = false;
    let mut size = 0;

    for &b in ticker.iter() {
        if b == 0 {
            seen_ticker = true;
            continue;
        }

        // does not allow 0 in the middle of the ticker
        if seen_ticker {
            return false;
        }

        //allows tickers A-B 0-9 - _
        if !(b'A'..=b'Z').contains(&b) && !(b'0'..=b'9').contains(&b) && b'-' != b && b'_' != b {
            return false;
        }

        size += 1;
    }

    size >= 3
}

#[cfg(test)]
mod test_convert_to_shares {
    use super::*;

    #[test]
    fn first_deposit_mints_one_to_one() {
        let deposit = 1_000_000;
        let total_assets = 0;
        let total_shares = 0;

        let shares = convert_to_shares(deposit, total_assets, total_shares).unwrap();
        assert_eq!(shares, deposit);
    }

    #[test]
    fn simple_proportional_deposit_price_one() {
        let deposit = 1_000_000;
        let total_assets = 10_000_000;
        let total_shares = 10_000_000;

        let shares = convert_to_shares(deposit, total_assets, total_shares).unwrap();
        assert_eq!(shares, 1_000_000);
    }

    #[test]
    fn vault_with_yield_price_greater_than_one() {
        let deposit = 1_000_000;
        let total_assets = 4_000_000;
        let total_shares = 2_000_000;

        let shares = convert_to_shares(deposit, total_assets, total_shares).unwrap();
        assert_eq!(shares, 500_000);
    }

    #[test]
    fn vault_price_less_than_one() {
        let deposit = 1_000_000;
        let total_assets = 2_000_000;
        let total_shares = 4_000_000;

        let shares = convert_to_shares(deposit, total_assets, total_shares).unwrap();
        assert_eq!(shares, 2_000_000);
    }

    #[test]
    fn rounding_down_floor_behavior() {
        let deposit = 1;
        let total_assets = 3;
        let total_shares = 10;

        let shares = convert_to_shares(deposit, total_assets, total_shares).unwrap();
        assert_eq!(shares, 3);
    }

    #[test]
    fn error_when_total_assets_is_zero_but_shares_exist() {
        let deposit = 100;
        let total_assets = 0;
        let total_shares = 1_000;

        let res = convert_to_shares(deposit, total_assets, total_shares);
        assert!(res.is_err());
    }
}

#[cfg(test)]
mod test_validate_ticker {
    use crate::util::is_valid_ticker;

    #[test]
    fn valid_ticker() {
        let mut ticker = [0u8; 16];
        let src = b"MYTOKEN";
        ticker[..src.len()].copy_from_slice(src);
        assert!(is_valid_ticker(&ticker));
    }

    #[test]
    fn invalid_ticker_starting_with_zero() {
        let ticker = b"\0\0MYTOKEN\0\0\0\0\0\0\0";
        assert!(!is_valid_ticker(&ticker));
    }

    #[test]
    fn invalid_ticker_wrong_minimum_size() {
        let ticker = b"M\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0";
        assert!(!is_valid_ticker(&ticker));
    }

    #[test]
    fn invalid_ticker_invalid_characters() {
        let ticker = b"MYTOK&N\0\0\0\0\0\0\0\0\0";
        assert!(!is_valid_ticker(&ticker));
    }

    #[test]
    fn invalid_ticker_zero_in_the_middle() {
        let ticker = b"MYT\0OKEN\0\0\0\0\0\0\0\0";
        assert!(!is_valid_ticker(&ticker));
    }

    #[test]
    fn invalid_ticker_lower_case() {
        let ticker = b"mytoken\0\0\0\0\0\0\0\0\0";
        assert!(!is_valid_ticker(&ticker));
    }
}
