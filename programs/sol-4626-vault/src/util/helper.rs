use crate::libraries::MulDiv;
use crate::util::Errors;
use anchor_lang::error;

/// Converts a deposit amount of the base asset into vault shares.
/// Assumptions:
///  - The base asset mint and the shares mint MUST have the same number of decimals.
///  - This keeps the math simple and avoids cross-decimal normalization.
///  - rounded down
pub fn convert_to_shares(
    deposit_amount: u64,
    total_assets: u64,
    total_shares: u64,
) -> Result<u64, error::Error> {
    // First deposit → mint 1:1
    if total_shares == 0 {
        return Ok(deposit_amount);
    }

    if total_assets == 0 {
        return Err(Errors::DivideByZero.into());
    }

    // Standard ERC-4626: proportional minting
    Ok(deposit_amount
        .mul_div_floor(total_shares, total_assets)
        .ok_or(Errors::MathOverflow)?)
}

#[cfg(test)]
mod tests {
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
