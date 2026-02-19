use std::{collections::LinkedList, convert::TryFrom};

use anyhow::Result;
use ed25519_dalek::VerifyingKey;
use tycho_types::{
    cell::{Cell, CellBuilder, HashBytes, Lazy, Load, Store},
    models::{
        Account, AccountState, CurrencyCollection, ExtInMsgInfo, IntAddr, OutAction, OwnedMessage,
        OwnedRelaxedMessage, RelaxedIntMsgInfo, RelaxedMsgInfo, SendMsgFlags, StateInit, StdAddr,
    },
};

use crate::utils::{
    ton_wallet::{Gift, TonWalletDetails},
    wallets::{self},
};

const SIGNED_EXTERNAL_PREFIX: u32 = 0x7369676E;
const SIGNED_INTERNAL_PREFIX: u32 = 0x73696E74;

pub fn prepare_deploy(
    public_key: &VerifyingKey,
    workchain: i8,
    expire_at: u32,
) -> Result<UnsignedWalletV5> {
    let init_data = make_init_data(public_key);
    let dst = compute_contract_address(public_key, workchain)?;
    let message = OwnedMessage {
        info: tycho_types::models::MsgInfo::ExtIn(ExtInMsgInfo {
            dst: IntAddr::Std(dst),
            ..Default::default()
        }),
        body: Default::default(),
        init: Some(init_data.make_state_init()?),
        layout: None,
    };
    let (hash, payload) = init_data.make_transfer_payload(None, expire_at, false)?;
    Ok(UnsignedWalletV5 {
        init_data,
        gifts: vec![],
        payload,
        hash,
        expire_at,
        message,
    })
}

pub fn prepare_state_init(public_key: &VerifyingKey) -> Result<StateInit> {
    let init_data = make_init_data(public_key);
    init_data.make_state_init()
}

pub fn make_init_data(public_key: &VerifyingKey) -> InitData {
    InitData::from_key(public_key)
        .with_wallet_id(WALLET_ID)
        .with_is_signature_allowed(true)
}

pub fn get_init_data(
    current_state: &Account,
    public_key: &VerifyingKey,
) -> Result<(InitData, bool)> {
    match current_state.state {
        AccountState::Active(ref state_init) => match &state_init.data {
            Some(data) => Ok((InitData::try_from(data)?, false)),
            None => Err(WalletV5Error::InvalidInitData.into()),
        },
        AccountState::Frozen { .. } => Err(WalletV5Error::AccountIsFrozen.into()),
        AccountState::Uninit => Ok((make_init_data(public_key), true)),
    }
}

pub fn get_init_data_from_state_init(init: &StateInit) -> Result<InitData> {
    match &init.data {
        Some(data) => Ok(InitData::try_from(data)?),
        None => Err(WalletV5Error::InvalidInitData.into()),
    }
}

pub fn prepare_transfer(
    public_key: &VerifyingKey,
    current_state: &Account,
    seqno_offset: u32,
    gifts: Vec<Gift>,
    expire_at: u32,
) -> Result<UnsignedWalletV5> {
    if gifts.len() > MAX_MESSAGES {
        return Err(WalletV5Error::TooManyGifts.into());
    }
    let (mut init_data, with_state_init) = get_init_data(current_state, public_key)?;

    init_data.seqno += seqno_offset;

    let (hash, payload) = init_data.make_transfer_payload(gifts.clone(), expire_at, false)?;

    let mut message = OwnedMessage {
        info: tycho_types::models::MsgInfo::ExtIn(ExtInMsgInfo {
            dst: IntAddr::Std(
                current_state
                    .address
                    .as_std()
                    .ok_or(WalletV5Error::InvalidAddress)?
                    .clone(),
            ),
            ..Default::default()
        }),
        body: Default::default(),
        init: None,
        layout: None,
    };

    if with_state_init {
        let state_init = init_data.make_state_init()?;
        message.init = Some(state_init);
    }

    Ok(UnsignedWalletV5 {
        init_data,
        gifts,
        payload,
        hash,
        expire_at,
        message,
    })
}

pub struct UnsignedWalletV5 {
    init_data: InitData,
    gifts: Vec<Gift>,
    payload: CellBuilder,
    hash: HashBytes,
    expire_at: u32,
    message: OwnedMessage,
}

impl UnsignedWalletV5 {
    pub fn expire_at(&self) -> u32 {
        self.expire_at
    }

    pub fn hash(&self) -> &[u8] {
        self.hash.as_slice()
    }

    pub fn gifts(&self) -> &[Gift] {
        &self.gifts
    }

    pub fn message(&self) -> &OwnedMessage {
        &self.message
    }

    pub fn init_data(&self) -> &InitData {
        &self.init_data
    }

    pub fn sign(&self, signature: &[u8; ed25519_dalek::SIGNATURE_LENGTH]) -> Result<OwnedMessage> {
        let mut payload = self.payload.clone();
        payload.store_raw(signature, (ed25519_dalek::SIGNATURE_LENGTH * 8) as u16)?;

        let mut message = self.message.clone();
        message.body = payload.build()?.into();

        Ok(message)
    }
}

pub static CODE_HASH: &[u8; 32] = &[
    0x20, 0x83, 0x4b, 0x7b, 0x72, 0xb1, 0x12, 0x14, 0x7e, 0x1b, 0x2f, 0xb4, 0x57, 0xb8, 0x4e, 0x74,
    0xd1, 0xa3, 0x0f, 0x04, 0xf7, 0x37, 0xd4, 0xf6, 0x2a, 0x66, 0x8e, 0x95, 0x52, 0xd2, 0xb7, 0x2f,
];

pub fn is_wallet_v5r1(code_hash: &HashBytes) -> bool {
    code_hash.as_slice() == CODE_HASH
}

pub fn compute_contract_address(public_key: &VerifyingKey, workchain_id: i8) -> Result<StdAddr> {
    make_init_data(public_key).compute_addr(workchain_id)
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
    pub public_key: HashBytes,
    pub extensions: Option<Cell>,
}

impl InitData {
    pub fn public_key(&self) -> &[u8; 32] {
        &self.public_key.0
    }

    pub fn from_key(key: &VerifyingKey) -> Self {
        Self {
            is_signature_allowed: false,
            seqno: 0,
            wallet_id: 0,
            public_key: HashBytes::from_slice(key.as_bytes()),
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

    pub fn compute_addr(&self, workchain_id: i8) -> Result<StdAddr> {
        let state_init = self.make_state_init()?;
        let cell_builder = CellBuilder::build_from(&state_init)?;
        let hash = cell_builder.repr_hash();
        Ok(StdAddr::new(workchain_id, *hash))
    }

    pub fn make_state_init(&self) -> Result<StateInit> {
        Ok(StateInit {
            code: Some(wallets::code::wallet_v5r1()),
            data: Some(self.serialize()?),
            ..Default::default()
        })
    }

    pub fn serialize(&self) -> Result<Cell> {
        let mut builder = CellBuilder::new();
        builder.store_bit(self.is_signature_allowed)?;
        builder.store_u32(self.seqno)?;
        builder.store_u32(self.wallet_id)?;
        builder.store_u256(&self.public_key)?;

        if let Some(extensions) = &self.extensions {
            builder.store_bit_one()?;
            builder.store_reference(extensions.clone())?;
        } else {
            builder.store_bit_zero()?;
        }

        let data = builder.build()?;
        Ok(data)
    }

    pub fn make_transfer_payload(
        &self,
        gifts: impl IntoIterator<Item = Gift>,
        expire_at: u32,
        is_internal_flow: bool,
    ) -> Result<(HashBytes, CellBuilder)> {
        // Check if signatures are allowed
        if !self.is_signature_allowed {
            return if self.extensions.is_none() {
                Err(WalletV5Error::WalletLocked.into())
            } else {
                Err(WalletV5Error::SignaturesDisabled.into())
            };
        }

        let mut builder = CellBuilder::new();

        // insert prefix
        if is_internal_flow {
            builder.store_u32(SIGNED_INTERNAL_PREFIX)?;
        } else {
            builder.store_u32(SIGNED_EXTERNAL_PREFIX)?;
        };

        builder.store_u32(self.wallet_id)?;
        builder.store_u32(expire_at)?;
        builder.store_u32(self.seqno)?;

        let mut actions_builder = OutActions(Default::default());

        for gift in gifts {
            let internal_message = Lazy::new(&OwnedRelaxedMessage {
                info: RelaxedMsgInfo::Int(RelaxedIntMsgInfo {
                    ihr_disabled: true,
                    bounce: gift.bounce,
                    dst: IntAddr::Std(gift.destination),
                    value: CurrencyCollection::new(gift.amount),
                    ..Default::default()
                }),
                init: gift.state_init,
                body: gift.body.unwrap_or(Default::default()).into(),
                layout: None,
            })?;

            let action = OutAction::SendMsg {
                mode: SendMsgFlags::from_bits_retain(gift.flags),
                out_msg: internal_message,
            };

            actions_builder.0.push_back(action);
        }

        builder.store_bit_one()?;
        builder.store_reference(CellBuilder::build_from(&actions_builder)?)?;

        // has_other_actions
        builder.store_bit_zero()?;

        let payload = builder.clone().build()?;
        let hash = payload.repr_hash();

        Ok((*hash, builder))
    }
}

impl TryFrom<&Cell> for InitData {
    type Error = anyhow::Error;

    fn try_from(data: &Cell) -> Result<Self, Self::Error> {
        let mut slice = data.as_slice()?;
        let is_signature_allowed = slice.load_bit()?;
        let seqno = slice.load_u32()?;
        let wallet_id = slice.load_u32()?;
        let mut buffer = [0u8; 32];
        slice.load_raw(&mut buffer, 256)?;
        let public_key = HashBytes::from_slice(&buffer);
        let extensions = Option::<Cell>::load_from(&mut slice)?;

        Ok(Self {
            is_signature_allowed,
            seqno,
            wallet_id,
            public_key,
            extensions,
        })
    }
}

pub struct OutActions(pub LinkedList<OutAction>);

impl Store for OutActions {
    fn store_into(
        &self,
        builder: &mut CellBuilder,
        context: &dyn tycho_types::prelude::CellContext,
    ) -> std::result::Result<(), tycho_types::error::Error> {
        let mut new_builder = CellBuilder::new();

        for action in self.0.iter() {
            let mut next_builder = CellBuilder::new();

            next_builder.store_reference(new_builder.build()?)?;
            action.store_into(&mut next_builder, context)?;

            new_builder = next_builder;
        }

        builder.store_builder(&new_builder)?;
        Ok(())
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
    #[error("Account address is not valid")]
    InvalidAddress,
}

#[cfg(test)]
mod tests {
    use ed25519_dalek::VerifyingKey;
    use tycho_types::{
        boc::Boc,
        cell::Load,
        models::{Account, AccountState},
    };

    use crate::utils::{
        ton_wallet::wallet_v5r1::{compute_contract_address, is_wallet_v5r1, InitData, WALLET_ID},
        wallets,
    };

    #[test]
    fn state_init() -> anyhow::Result<()> {
        let account_base64 = "te6ccgECFgEAAucAAm6ADZRqTnEksRaYvpXRMbgzB92SzFv/19WbfQQgdDo7lYwEWQnKBnPzD1AAAXPmjwdAEj9i9OgmAgEAUYAAAAG///+IyIPTKTihvw1MFdzCAl7NQWIaeY9xhjENsss4FdrN+FAgART/APSkE/S88sgLAwIBIAYEAQLyBQEeINcLH4IQc2lnbrry4Ip/EQIBSBAHAgEgCQgAGb5fD2omhAgKDrkPoCwCASANCgIBSAwLABGyYvtRNDXCgCAAF7Ml+1E0HHXIdcLH4AIBbg8OABmvHfaiaEAQ65DrhY/AABmtznaiaEAg65Drhf/AAtzQINdJwSCRW49jINcLHyCCEGV4dG69IYIQc2ludL2wkl8D4IIQZXh0brqOtIAg1yEB0HTXIfpAMPpE+Cj6RDBYvZFb4O1E0IEBQdch9AWDB/QOb6ExkTDhgEDXIXB/2zzgMSDXSYECgLmRMOBw4hIRAeaO8O2i7fshgwjXIgKDCNcjIIAg1yHTH9Mf0x/tRNDSANMfINMf0//XCgAK+QFAzPkQmiiUXwrbMeHywIffArNQB7Dy0IRRJbry4IVQNrry4Ib4I7vy0IgikvgA3gGkf8jKAMsfAc8Wye1UIJL4D95w2zzYEgP27aLt+wL0BCFukmwhjkwCIdc5MHCUIccAs44tAdcoIHYeQ2wg10nACPLgkyDXSsAC8uCTINcdBscSwgBSMLDy0InXTNc5MAGk6GwShAe78uCT10rAAPLgk+1V4tIAAcAAkVvg69csCBQgkXCWAdcsCBwS4lIQseMPINdKFRQTABCTW9sx4ddM0AByMNcsCCSOLSHy4JLSAO1E0NIAURO68tCPVFAwkTGcAYEBQNch1woA8uCO4sjKAFjPFsntVJPywI3iAJYB+kAB+kT4KPpEMFi68uCR7UTQgQFB1xj0BQSdf8jKAEAEgwf0U/Lgi44UA4MH9Fvy4Iwi1woAIW4Bs7Dy0JDiyFADzxYS9ADJ7VQ=";
        let account = Boc::decode_base64(account_base64)?;

        let state = Account::load_from(&mut account.as_slice()?)?;

        if let AccountState::Active(state_init) = state.state {
            let init_data = InitData::try_from(&state_init.data.unwrap())?;
            assert_eq!(init_data.is_signature_allowed, true);
            assert_eq!(
                init_data.public_key.to_string(),
                "9107a65271437e1a982bb98404bd9a82c434f31ee30c621b6596702bb59bf0a0"
            );
            assert_eq!(init_data.wallet_id, WALLET_ID);
            assert_eq!(init_data.extensions, None);

            let public_key =
                VerifyingKey::from_bytes(init_data.public_key.as_slice().try_into().unwrap())?;
            let address = compute_contract_address(&public_key, 0)?;
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

    //    #[test]
    //    fn check_signature_test() -> anyhow::Result<()> {
    //        let public_key_bytes =
    //            hex::decode("6c2f9514c1c0f2ec54cffe1ac2ba0e85268e76442c14205581ebc808fe7ee52c")?;
    //        //let payload = base64::decode("te6ccgECCQEAAWMAASFzaW50f///EWjJNSIAAAABoAECCg7DyG0DBQIB80IAEiSxvuIkjLwTZ/69OCTi5io4ZpgjPKnD56XnecGH1Q0gcJ32yAAAAAAAAAAAAAAAAABz4iFDAAAAAAAAAAAAAAAAO5rKAIAfPq6ksCQX/kNfsY8xS5PTRd4WSjwjs5C/fod9ktFK+MAAAAAAAAAAAAAAAAD39JAwAwFDgBI2HlLkTtTC7ntWgsSS4jmXMUkhy2OTDHvAO1YAIIdyCAQBCAAAAAAIAgoOw8htAwgGAdNCABIksb7iJIy8E2f+vTgk4uYqOGaYIzypw+el53nBh9UNIC+vCAAAAAAAAAAAAAAAAAAARqnX7AAAAAAAAAAAAAAAAAIA9mKAH6YK7ZtGhTyJBnq9b54dnz07z830q8r/r5MBXJdSioIQBwFDgBhcpJ/VWhGKPK44GyznIrRqKDcoivK5/ZanRrMrFKCjiAgAAA==")?;
    //        let payload = base64::decode("te6ccgECCQEAAaMAAaFzaW50f///EWjJNSIAAAABr9SYdbfeTOkhxaWVTsB40YIzxnswT6p7oxjydvTUZ0afi8fq5F2NvuyGho+YxBUC2NPkhtL3+tuMa5CfUwJMg2ABAgoOw8htAwUCAfNCABIksb7iJIy8E2f+vTgk4uYqOGaYIzypw+el53nBh9UNIHCd9sgAAAAAAAAAAAAAAAAAc+IhQwAAAAAAAAAAAAAAADuaygCAHz6upLAkF/5DX7GPMUuT00XeFko8I7OQv36HfZLRSvjAAAAAAAAAAAAAAAAA9/SQMAMBQ4ASNh5S5E7Uwu57VoLEkuI5lzFJIctjkwx7wDtWACCHcggEAQgAAAAACAIKDsPIbQMIBgHTQgASJLG+4iSMvBNn/r04JOLmKjhmmCM8qcPnped5wYfVDSAvrwgAAAAAAAAAAAAAAAAAAEap1+wAAAAAAAAAAAAAAAACAPZigB+mCu2bRoU8iQZ6vW+eHZ89O8/N9KvK/6+TAVyXUoqCEAcBQ4AYXKSf1VoRijyuOBss5yK0aig3KIryuf2Wp0azKxSgo4gIAAA=")?;
    //        let in_msg_body = ton_types::deserialize_tree_of_cells(&mut payload.as_slice())?;
    //        let in_msg_body_slice = SliceData::load_cell(in_msg_body)?;
    //
    //        let public_key = VerifyingKey::from_bytes(public_key_bytes.as_slice())?;
    //
    //        let result = check_signature(in_msg_body_slice, public_key, Some(2000))?;
    //        assert!(result);
    //        Ok(())
    //    }
    //
    //    fn check_signature(
    //        mut in_msg_body: SliceData,
    //        public_key: VerifyingKey,
    //        signature_id: Option<i32>,
    //    ) -> anyhow::Result<bool> {
    //        let signature_binding = in_msg_body
    //            .get_slice(in_msg_body.remaining_bits() - 512, 512)?
    //            .remaining_data();
    //        let sig = signature_binding.data();
    //
    //        let payload = in_msg_body
    //            .shrink_data(in_msg_body.remaining_bits() - 512..)
    //            .into_cell();
    //
    //        let hash = payload.repr_hash();
    //
    //        let data = extend_with_signature_id(hash.as_ref(), signature_id);
    //
    //        Ok(public_key
    //            .verify(&*data, &Signature::from_bytes(sig)?)
    //            .is_ok())
    //    }
}
