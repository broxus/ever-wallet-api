use std::convert::TryFrom;

use anyhow::Result;
use ed25519_dalek::PublicKey;
use ton_block::{MsgAddrStd, MsgAddressInt, Serializable};
use ton_types::{BuilderData, Cell, IBitstring, SliceData, UInt256};

use nekoton_utils::*;

use super::{Gift, TonWalletDetails, TransferAction};
use crate::core::models::{Expiration, ExpireAt};
use crate::crypto::{SignedMessage, UnsignedMessage};

const SIGNED_EXTERNAL_PREFIX: u32 = 0x7369676E;
const SIGNED_INTERNAL_PREFIX: u32 = 0x73696E74;

pub fn prepare_deploy(
    clock: &dyn Clock,
    public_key: &PublicKey,
    workchain: i8,
    expiration: Expiration,
) -> Result<Box<dyn UnsignedMessage>> {
    let init_data = make_init_data(public_key);
    let dst = compute_contract_address(public_key, workchain);
    let mut message =
        ton_block::Message::with_ext_in_header(ton_block::ExternalInboundMessageHeader {
            dst,
            ..Default::default()
        });

    message.set_state_init(init_data.make_state_init()?);

    let expire_at = ExpireAt::new(clock, expiration);
    let (hash, payload) = init_data.make_transfer_payload(None, expire_at.timestamp, false)?;

    Ok(Box::new(UnsignedWalletV5 {
        init_data,
        gifts: Vec::new(),
        payload,
        message,
        expire_at,
        hash,
    }))
}

pub fn prepare_state_init(public_key: &PublicKey) -> Result<ton_block::StateInit> {
    let init_data = make_init_data(public_key);
    init_data.make_state_init()
}

pub fn make_init_data(public_key: &PublicKey) -> InitData {
    InitData::from_key(public_key)
        .with_wallet_id(WALLET_ID)
        .with_is_signature_allowed(true)
}

pub fn get_init_data(
    current_state: &ton_block::AccountState,
    public_key: &PublicKey,
) -> Result<(InitData, bool)> {
    match current_state {
        ton_block::AccountState::AccountActive { state_init, .. } => match &state_init.data {
            Some(data) => Ok((InitData::try_from(data)?, false)),
            None => Err(WalletV5Error::InvalidInitData.into()),
        },
        ton_block::AccountState::AccountFrozen { .. } => Err(WalletV5Error::AccountIsFrozen.into()),
        ton_block::AccountState::AccountUninit => Ok((make_init_data(public_key), true)),
    }
}

pub fn get_init_data_from_state_init(init: &ton_block::StateInit) -> Result<InitData> {
    match &init.data {
        Some(data) => Ok(InitData::try_from(data)?),
        None => Err(WalletV5Error::InvalidInitData.into()),
    }
}

pub fn prepare_transfer(
    clock: &dyn Clock,
    public_key: &PublicKey,
    current_state: &ton_block::AccountStuff,
    seqno_offset: u32,
    gifts: Vec<Gift>,
    expiration: Expiration,
) -> Result<TransferAction> {
    if gifts.len() > MAX_MESSAGES {
        return Err(WalletV5Error::TooManyGifts.into());
    }
    let (mut init_data, with_state_init) =
        get_init_data(current_state.storage.state(), public_key)?;

    init_data.seqno += seqno_offset;

    let mut message =
        ton_block::Message::with_ext_in_header(ton_block::ExternalInboundMessageHeader {
            dst: current_state.addr.clone(),
            ..Default::default()
        });

    if with_state_init {
        message.set_state_init(init_data.make_state_init()?);
    }

    let expire_at = ExpireAt::new(clock, expiration);
    let (hash, payload) =
        init_data.make_transfer_payload(gifts.clone(), expire_at.timestamp, false)?;

    Ok(TransferAction::Sign(Box::new(UnsignedWalletV5 {
        init_data,
        gifts,
        payload,
        hash,
        expire_at,
        message,
    })))
}

#[derive(Clone)]
struct UnsignedWalletV5 {
    init_data: InitData,
    gifts: Vec<Gift>,
    payload: BuilderData,
    hash: UInt256,
    expire_at: ExpireAt,
    message: ton_block::Message,
}

impl UnsignedMessage for UnsignedWalletV5 {
    fn refresh_timeout(&mut self, clock: &dyn Clock) {
        if !self.expire_at.refresh(clock) {
            return;
        }

        let (hash, payload) = self
            .init_data
            .make_transfer_payload(self.gifts.clone(), self.expire_at(), false)
            .trust_me();
        self.hash = hash;
        self.payload = payload;
    }

    fn expire_at(&self) -> u32 {
        self.expire_at.timestamp
    }

    fn hash(&self) -> &[u8] {
        self.hash.as_slice()
    }

    fn sign(&self, signature: &[u8; ed25519_dalek::SIGNATURE_LENGTH]) -> Result<SignedMessage> {
        let mut payload = self.payload.clone();
        payload.append_raw(signature, signature.len() * 8)?;

        let mut message = self.message.clone();
        message.set_body(SliceData::load_builder(payload)?);

        Ok(SignedMessage {
            message,
            expire_at: self.expire_at(),
        })
    }

    fn sign_with_pruned_payload(
        &self,
        signature: &[u8; ed25519_dalek::SIGNATURE_LENGTH],
        prune_after_depth: u16,
    ) -> Result<SignedMessage> {
        let mut payload = self.payload.clone();
        payload.append_raw(signature, signature.len() * 8)?;
        let body = payload.into_cell()?;

        let mut message = self.message.clone();
        message.set_body(prune_deep_cells(&body, prune_after_depth)?);

        Ok(SignedMessage {
            message,
            expire_at: self.expire_at(),
        })
    }
}

pub static CODE_HASH: &[u8; 32] = &[
    0x20, 0x83, 0x4b, 0x7b, 0x72, 0xb1, 0x12, 0x14, 0x7e, 0x1b, 0x2f, 0xb4, 0x57, 0xb8, 0x4e, 0x74,
    0xd1, 0xa3, 0x0f, 0x04, 0xf7, 0x37, 0xd4, 0xf6, 0x2a, 0x66, 0x8e, 0x95, 0x52, 0xd2, 0xb7, 0x2f,
];

pub fn is_wallet_v5r1(code_hash: &UInt256) -> bool {
    code_hash.as_slice() == CODE_HASH
}

pub fn compute_contract_address(public_key: &PublicKey, workchain_id: i8) -> MsgAddressInt {
    make_init_data(public_key)
        .compute_addr(workchain_id)
        .trust_me()
}

pub static DETAILS: TonWalletDetails = TonWalletDetails {
    requires_separate_deploy: false,
    min_amount: 1, // 0.000000001 TON
    max_messages: MAX_MESSAGES,
    supports_payload: true,
    supports_state_init: true,
    supports_multiple_owners: false,
    supports_code_update: false,
    expiration_time: 0,
    required_confirmations: None,
};

const MAX_MESSAGES: usize = 250;

/// `WalletV5` init data
#[derive(Clone)]
pub struct InitData {
    pub is_signature_allowed: bool,
    pub seqno: u32,
    pub wallet_id: u32,
    pub public_key: UInt256,
    pub extensions: Option<Cell>,
}

impl InitData {
    pub fn public_key(&self) -> &[u8; 32] {
        self.public_key.as_slice()
    }

    pub fn from_key(key: &PublicKey) -> Self {
        Self {
            is_signature_allowed: false,
            seqno: 0,
            wallet_id: 0,
            public_key: key.as_bytes().into(),
            extensions: Default::default(),
        }
    }

    pub fn with_is_signature_allowed(mut self, is_allowed: bool) -> Self {
        self.is_signature_allowed = is_allowed;
        self
    }

    pub fn with_wallet_id(mut self, id: u32) -> Self {
        self.wallet_id = id;
        self
    }

    pub fn compute_addr(&self, workchain_id: i8) -> Result<MsgAddressInt> {
        let init_state = self.make_state_init()?.serialize()?;
        let hash = init_state.repr_hash();
        Ok(MsgAddressInt::AddrStd(MsgAddrStd {
            anycast: None,
            workchain_id,
            address: hash.into(),
        }))
    }

    pub fn make_state_init(&self) -> Result<ton_block::StateInit> {
        Ok(ton_block::StateInit {
            code: Some(nekoton_contracts::wallets::code::wallet_v5r1()),
            data: Some(self.serialize()?),
            ..Default::default()
        })
    }

    pub fn serialize(&self) -> Result<Cell> {
        let mut data = BuilderData::new();
        data.append_bit_bool(self.is_signature_allowed)?
            .append_u32(self.seqno)?
            .append_u32(self.wallet_id)?
            .append_raw(self.public_key.as_slice(), 256)?;

        if let Some(extensions) = &self.extensions {
            data.append_bit_one()?
                .checked_append_reference(extensions.clone())?;
        } else {
            data.append_bit_zero()?;
        }

        data.into_cell()
    }

    pub fn make_transfer_payload(
        &self,
        gifts: impl IntoIterator<Item = Gift>,
        expire_at: u32,
        is_internal_flow: bool,
    ) -> Result<(UInt256, BuilderData)> {
        // Check if signatures are allowed
        if !self.is_signature_allowed {
            return if self.extensions.is_none() {
                Err(WalletV5Error::WalletLocked.into())
            } else {
                Err(WalletV5Error::SignaturesDisabled.into())
            };
        }

        let mut payload = BuilderData::new();

        // insert prefix
        if is_internal_flow {
            payload.append_u32(SIGNED_INTERNAL_PREFIX)?;
        } else {
            payload.append_u32(SIGNED_EXTERNAL_PREFIX)?;
        };

        payload
            .append_u32(self.wallet_id)?
            .append_u32(expire_at)?
            .append_u32(self.seqno)?;

        let mut actions = ton_block::OutActions::new();

        for gift in gifts {
            let mut internal_message =
                ton_block::Message::with_int_header(ton_block::InternalMessageHeader {
                    ihr_disabled: true,
                    bounce: gift.bounce,
                    dst: gift.destination,
                    value: ton_block::CurrencyCollection::from_grams(ton_block::Grams::new(
                        gift.amount,
                    )?),
                    ..Default::default()
                });

            if let Some(body) = gift.body {
                internal_message.set_body(body);
            }

            if let Some(state_init) = gift.state_init {
                internal_message.set_state_init(state_init);
            }

            let action = ton_block::OutAction::SendMsg {
                mode: gift.flags,
                out_msg: internal_message,
            };

            actions.push_back(action);
        }

        payload.append_bit_one()?;
        payload.checked_append_reference(actions.serialize()?)?;

        // has_other_actions
        payload.append_bit_zero()?;

        let hash = payload.clone().into_cell()?.repr_hash();

        Ok((hash, payload))
    }
}

impl TryFrom<&Cell> for InitData {
    type Error = anyhow::Error;

    fn try_from(data: &Cell) -> Result<Self, Self::Error> {
        let mut cs = SliceData::load_cell_ref(data)?;
        Ok(Self {
            is_signature_allowed: cs.get_next_bit()?,
            seqno: cs.get_next_u32()?,
            wallet_id: cs.get_next_u32()?,
            public_key: UInt256::from_be_bytes(&cs.get_next_bytes(32)?),
            extensions: cs.get_next_dictionary()?,
        })
    }
}

const WALLET_ID: u32 = 0x7FFFFF11;

#[derive(thiserror::Error, Debug)]
enum WalletV5Error {
    #[error("Invalid init data")]
    InvalidInitData,
    #[error("Account is frozen")]
    AccountIsFrozen,
    #[error("Too many outgoing messages")]
    TooManyGifts,
    #[error("Signatures are disabled")]
    SignaturesDisabled,
    #[error("Wallet locked")]
    WalletLocked,
}

#[cfg(test)]
mod tests {
    use crate::core::ton_wallet::wallet_v5r1::{
        compute_contract_address, is_wallet_v5r1, InitData, WALLET_ID,
    };
    use crate::crypto::extend_with_signature_id;
    use ed25519_dalek::{PublicKey, Signature, Verifier};
    use nekoton_contracts::wallets;
    use ton_block::AccountState;
    use ton_types::SliceData;

    #[test]
    fn state_init() -> anyhow::Result<()> {
        let cell = ton_types::deserialize_tree_of_cells(&mut base64::decode("te6ccgECFgEAAucAAm6ADZRqTnEksRaYvpXRMbgzB92SzFv/19WbfQQgdDo7lYwEWQnKBnPzD1AAAXPmjwdAEj9i9OgmAgEAUYAAAAG///+IyIPTKTihvw1MFdzCAl7NQWIaeY9xhjENsss4FdrN+FAgART/APSkE/S88sgLAwIBIAYEAQLyBQEeINcLH4IQc2lnbrry4Ip/EQIBSBAHAgEgCQgAGb5fD2omhAgKDrkPoCwCASANCgIBSAwLABGyYvtRNDXCgCAAF7Ml+1E0HHXIdcLH4AIBbg8OABmvHfaiaEAQ65DrhY/AABmtznaiaEAg65Drhf/AAtzQINdJwSCRW49jINcLHyCCEGV4dG69IYIQc2ludL2wkl8D4IIQZXh0brqOtIAg1yEB0HTXIfpAMPpE+Cj6RDBYvZFb4O1E0IEBQdch9AWDB/QOb6ExkTDhgEDXIXB/2zzgMSDXSYECgLmRMOBw4hIRAeaO8O2i7fshgwjXIgKDCNcjIIAg1yHTH9Mf0x/tRNDSANMfINMf0//XCgAK+QFAzPkQmiiUXwrbMeHywIffArNQB7Dy0IRRJbry4IVQNrry4Ib4I7vy0IgikvgA3gGkf8jKAMsfAc8Wye1UIJL4D95w2zzYEgP27aLt+wL0BCFukmwhjkwCIdc5MHCUIccAs44tAdcoIHYeQ2wg10nACPLgkyDXSsAC8uCTINcdBscSwgBSMLDy0InXTNc5MAGk6GwShAe78uCT10rAAPLgk+1V4tIAAcAAkVvg69csCBQgkXCWAdcsCBwS4lIQseMPINdKFRQTABCTW9sx4ddM0AByMNcsCCSOLSHy4JLSAO1E0NIAURO68tCPVFAwkTGcAYEBQNch1woA8uCO4sjKAFjPFsntVJPywI3iAJYB+kAB+kT4KPpEMFi68uCR7UTQgQFB1xj0BQSdf8jKAEAEgwf0U/Lgi44UA4MH9Fvy4Iwi1woAIW4Bs7Dy0JDiyFADzxYS9ADJ7VQ=").unwrap().as_slice()).unwrap();
        let state = nekoton_utils::deserialize_account_stuff(cell)?;

        if let AccountState::AccountActive { state_init } = state.storage.state() {
            let init_data = InitData::try_from(state_init.data().unwrap())?;
            assert_eq!(init_data.is_signature_allowed, true);
            assert_eq!(
                init_data.public_key.to_hex_string(),
                "9107a65271437e1a982bb98404bd9a82c434f31ee30c621b6596702bb59bf0a0"
            );
            assert_eq!(init_data.wallet_id, WALLET_ID);
            assert_eq!(init_data.extensions, None);

            let public_key = PublicKey::from_bytes(init_data.public_key.as_slice())?;
            let address = compute_contract_address(&public_key, 0);
            assert_eq!(
                address.to_string(),
                "0:6ca35273892588b4c5f4ae898dc1983eec9662dffebeacdbe82103a1d1dcac60"
            );
        }

        Ok(())
    }

    #[test]
    fn code_hash() -> anyhow::Result<()> {
        let code_cell = wallets::code::wallet_v5r1();

        let is_wallet_v5r1 = is_wallet_v5r1(&code_cell.repr_hash());
        assert!(is_wallet_v5r1);

        Ok(())
    }

    #[test]
    fn check_signature_test() -> anyhow::Result<()> {
        let public_key_bytes =
            hex::decode("6c2f9514c1c0f2ec54cffe1ac2ba0e85268e76442c14205581ebc808fe7ee52c")?;
        //let payload = base64::decode("te6ccgECCQEAAWMAASFzaW50f///EWjJNSIAAAABoAECCg7DyG0DBQIB80IAEiSxvuIkjLwTZ/69OCTi5io4ZpgjPKnD56XnecGH1Q0gcJ32yAAAAAAAAAAAAAAAAABz4iFDAAAAAAAAAAAAAAAAO5rKAIAfPq6ksCQX/kNfsY8xS5PTRd4WSjwjs5C/fod9ktFK+MAAAAAAAAAAAAAAAAD39JAwAwFDgBI2HlLkTtTC7ntWgsSS4jmXMUkhy2OTDHvAO1YAIIdyCAQBCAAAAAAIAgoOw8htAwgGAdNCABIksb7iJIy8E2f+vTgk4uYqOGaYIzypw+el53nBh9UNIC+vCAAAAAAAAAAAAAAAAAAARqnX7AAAAAAAAAAAAAAAAAIA9mKAH6YK7ZtGhTyJBnq9b54dnz07z830q8r/r5MBXJdSioIQBwFDgBhcpJ/VWhGKPK44GyznIrRqKDcoivK5/ZanRrMrFKCjiAgAAA==")?;
        let payload = base64::decode("te6ccgECCQEAAaMAAaFzaW50f///EWjJNSIAAAABr9SYdbfeTOkhxaWVTsB40YIzxnswT6p7oxjydvTUZ0afi8fq5F2NvuyGho+YxBUC2NPkhtL3+tuMa5CfUwJMg2ABAgoOw8htAwUCAfNCABIksb7iJIy8E2f+vTgk4uYqOGaYIzypw+el53nBh9UNIHCd9sgAAAAAAAAAAAAAAAAAc+IhQwAAAAAAAAAAAAAAADuaygCAHz6upLAkF/5DX7GPMUuT00XeFko8I7OQv36HfZLRSvjAAAAAAAAAAAAAAAAA9/SQMAMBQ4ASNh5S5E7Uwu57VoLEkuI5lzFJIctjkwx7wDtWACCHcggEAQgAAAAACAIKDsPIbQMIBgHTQgASJLG+4iSMvBNn/r04JOLmKjhmmCM8qcPnped5wYfVDSAvrwgAAAAAAAAAAAAAAAAAAEap1+wAAAAAAAAAAAAAAAACAPZigB+mCu2bRoU8iQZ6vW+eHZ89O8/N9KvK/6+TAVyXUoqCEAcBQ4AYXKSf1VoRijyuOBss5yK0aig3KIryuf2Wp0azKxSgo4gIAAA=")?;
        let in_msg_body = ton_types::deserialize_tree_of_cells(&mut payload.as_slice())?;
        let in_msg_body_slice = SliceData::load_cell(in_msg_body)?;

        let public_key = PublicKey::from_bytes(public_key_bytes.as_slice())?;

        let result = check_signature(in_msg_body_slice, public_key, Some(2000))?;
        assert!(result);
        Ok(())
    }

    fn check_signature(
        mut in_msg_body: SliceData,
        public_key: PublicKey,
        signature_id: Option<i32>,
    ) -> anyhow::Result<bool> {
        let signature_binding = in_msg_body
            .get_slice(in_msg_body.remaining_bits() - 512, 512)?
            .remaining_data();
        let sig = signature_binding.data();

        let payload = in_msg_body
            .shrink_data(in_msg_body.remaining_bits() - 512..)
            .into_cell();

        let hash = payload.repr_hash();

        let data = extend_with_signature_id(hash.as_ref(), signature_id);

        Ok(public_key
            .verify(&*data, &Signature::from_bytes(sig)?)
            .is_ok())
    }
}
