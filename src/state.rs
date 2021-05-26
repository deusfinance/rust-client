use solana_program::{program_error::ProgramError, program_pack::{IsInitialized, Pack, Sealed}, pubkey::Pubkey};
use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};

/// Synchronizer data.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SynchronizerData {
    pub is_initialized: bool,
    /// USDC Token address
    pub collateral_token_key: Pubkey,
    pub remaining_dollar_cap: u64,
    pub withdrawable_fee_amount: u64,
}
impl Sealed for SynchronizerData {}
impl IsInitialized for SynchronizerData {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}
impl Pack for SynchronizerData {
    const LEN: usize = 49; // 1 + 32 + 8 + 8
    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let src = array_ref![src, 0, 49];
        let (
            is_initialized,
            collateral_token_key,
            remaining_dollar_cap,
            withdrawable_fee_amount
        ) = array_refs![src, 1, 32, 8, 8];

        let is_initialized = match is_initialized {
            [0] => false,
            [1] => true,
            _ => return Err(ProgramError::InvalidAccountData),
        };

        Ok(SynchronizerData {
            is_initialized,
            collateral_token_key: Pubkey::new_from_array(*collateral_token_key),
            remaining_dollar_cap: u64::from_le_bytes(*remaining_dollar_cap),
            withdrawable_fee_amount: u64::from_le_bytes(*withdrawable_fee_amount),
        })
    }

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst = array_mut_ref![dst, 0, 49];
        let (
            is_initialized_dst,
            collateral_token_key_dst,
            remaining_dollar_cap_dst,
            withdrawable_fee_amount_dst,
        ) = mut_array_refs![dst, 1, 32, 8, 8];

        let &SynchronizerData {
            is_initialized,
            collateral_token_key,
            remaining_dollar_cap,
            withdrawable_fee_amount
        } = self;

        is_initialized_dst[0] = is_initialized as u8;
        collateral_token_key_dst.copy_from_slice(collateral_token_key.as_ref());
        *remaining_dollar_cap_dst = remaining_dollar_cap.to_le_bytes();
        *withdrawable_fee_amount_dst = withdrawable_fee_amount.to_le_bytes();
    }
}
