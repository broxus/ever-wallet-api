use std::convert::TryFrom;

use anyhow::Result;
use ed25519_dalek::PublicKey;
use ton_block::{MsgAddrStd, MsgAddressInt, Serializable};
use ton_types::{BuilderData, Cell, HashmapE, HashmapType, IBitstring, SliceData, UInt256};

use nekoton_utils::*;

use super::{Gift, TonWalletDetails, TransferAction};
use crate::core::models::{Expiration, ExpireAt};
use crate::crypto::{SignedMessage, UnsignedMessage};

pub fn prepare_deploy(
    clock: &dyn Clock,
    public_key: &PublicKey,
    workchain: i8,
    expiration: Expiration,
) -> Result<Box<dyn UnsignedMessage>> {
    let init_data = InitData::from_key(public_key).with_wallet_id(WALLET_ID);
    let dst = compute_contract_address(public_key, workchain);
    let mut message =
        ton_block::Message::with_ext_in_header(ton_block::ExternalInboundMessageHeader {
            dst,
            ..Default::default()
        });

    message.set_state_init(init_data.make_state_init()?);

    let expire_at = ExpireAt::new(clock, expiration);
    let (hash, payload) = init_data.make_deploy_payload(expire_at.timestamp)?;

    Ok(Box::new(UnsignedHighloadWalletV2Message {
        init_data,
        gifts: Vec::new(),
        payload,
        message,
        expire_at,
        hash,
    }))
}

pub fn prepare_state_init(public_key: &PublicKey) -> Result<ton_block::StateInit> {
    let init_data = InitData::from_key(public_key).with_wallet_id(WALLET_ID);
    init_data.make_state_init()
}

pub fn prepare_transfer(
    clock: &dyn Clock,
    public_key: &PublicKey,
    current_state: &ton_block::AccountStuff,
    gifts: Vec<Gift>,
    expiration: Expiration,
) -> Result<TransferAction> {
    if gifts.len() > DETAILS.max_messages {
        return Err(HighloadWalletV2Error::TooManyGifts.into());
    }

    let (init_data, with_state_init) = match &current_state.storage.state {
        ton_block::AccountState::AccountActive { state_init, .. } => match &state_init.data {
            Some(data) => (InitData::try_from(data)?, false),
            None => return Err(HighloadWalletV2Error::InvalidInitData.into()),
        },
        ton_block::AccountState::AccountFrozen { .. } => {
            return Err(HighloadWalletV2Error::AccountIsFrozen.into())
        }
        ton_block::AccountState::AccountUninit => (
            InitData::from_key(public_key).with_wallet_id(WALLET_ID),
            true,
        ),
    };

    if init_data.data.len()? >= 500_usize {
        return Err(HighloadWalletV2Error::InitDataTooLarge.into());
    }

    let mut message =
        ton_block::Message::with_ext_in_header(ton_block::ExternalInboundMessageHeader {
            dst: current_state.addr.clone(),
            ..Default::default()
        });

    if with_state_init {
        message.set_state_init(init_data.make_state_init()?);
    }

    let expire_at = ExpireAt::new(clock, expiration);
    let (hash, payload) = init_data.make_transfer_payload(gifts.clone(), expire_at.timestamp)?;

    Ok(TransferAction::Sign(Box::new(
        UnsignedHighloadWalletV2Message {
            init_data,
            gifts,
            payload,
            hash,
            expire_at,
            message,
        },
    )))
}

#[derive(Clone)]
struct UnsignedHighloadWalletV2Message {
    init_data: InitData,
    gifts: Vec<Gift>,
    payload: BuilderData,
    hash: UInt256,
    expire_at: ExpireAt,
    message: ton_block::Message,
}

impl UnsignedMessage for UnsignedHighloadWalletV2Message {
    fn refresh_timeout(&mut self, clock: &dyn Clock) {
        if !self.expire_at.refresh(clock) {
            return;
        }

        let expire_at = self.expire_at();

        let (hash, payload) = if self.gifts.is_empty() {
            self.init_data.make_deploy_payload(expire_at)
        } else {
            self.init_data
                .make_transfer_payload(self.gifts.clone(), expire_at)
        }
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
        payload.prepend_raw(signature, signature.len() * 8)?;

        let mut message = self.message.clone();
        message.set_body(ton_types::SliceData::load_builder(payload)?);

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
        payload.prepend_raw(signature, signature.len() * 8)?;
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
    0x0b, 0x3a, 0x88, 0x7a, 0xea, 0xcd, 0x2a, 0x7d, 0x40, 0xbb, 0x55, 0x50, 0xbc, 0x92, 0x53, 0x15,
    0x6a, 0x02, 0x90, 0x65, 0xae, 0xfb, 0x6d, 0x6b, 0x58, 0x37, 0x35, 0xd5, 0x8d, 0xa9, 0xd5, 0xbe,
];

pub fn is_highload_wallet_v2(code_hash: &UInt256) -> bool {
    code_hash.as_slice() == CODE_HASH
}

pub fn compute_contract_address(public_key: &PublicKey, workchain_id: i8) -> MsgAddressInt {
    InitData::from_key(public_key)
        .with_wallet_id(WALLET_ID)
        .compute_addr(workchain_id)
        .trust_me()
}

pub static DETAILS: TonWalletDetails = TonWalletDetails {
    requires_separate_deploy: false,
    min_amount: 1, // 0.000000001 TON
    max_messages: 250,
    supports_payload: true,
    supports_state_init: true,
    supports_multiple_owners: false,
    supports_code_update: false,
    expiration_time: 0,
    required_confirmations: None,
};

/// `HighloadWalletV2` init data
#[derive(Clone)]
pub struct InitData {
    pub wallet_id: u32,
    pub last_cleaned: u64,
    pub public_key: UInt256,
    pub data: HashmapE,
}

impl InitData {
    pub fn public_key(&self) -> &[u8; 32] {
        self.public_key.as_slice()
    }

    pub fn from_key(key: &PublicKey) -> Self {
        Self {
            wallet_id: 0,
            last_cleaned: 0,
            public_key: key.as_bytes().into(),
            data: HashmapE::default(),
        }
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
            code: Some(nekoton_contracts::wallets::code::highload_wallet_v2()),
            data: Some(self.serialize()?),
            ..Default::default()
        })
    }

    pub fn serialize(&self) -> Result<Cell> {
        let mut data = BuilderData::new();
        data.append_u32(self.wallet_id)?
            .append_u64(self.last_cleaned)?
            .append_raw(self.public_key.as_slice(), 256)?;
        self.data.write_hashmap_data(&mut data)?;
        data.into_cell()
    }

    pub fn make_deploy_payload(&self, expire_at: u32) -> Result<(UInt256, BuilderData)> {
        let mut payload = BuilderData::new();
        payload
            .append_u32(self.wallet_id)?
            .append_u32(expire_at)?
            .append_u32(u32::MAX)?
            .append_bit_zero()?;

        let hash = payload.clone().into_cell()?.repr_hash();

        Ok((hash, payload))
    }

    pub fn make_transfer_payload(
        &self,
        gifts: impl IntoIterator<Item = Gift>,
        expire_at: u32,
    ) -> Result<(UInt256, BuilderData)> {
        // Prepare messages array
        let mut messages = ton_types::HashmapE::with_bit_len(16);
        for (i, gift) in gifts.into_iter().enumerate() {
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

            let mut item = BuilderData::new();
            item.append_u8(gift.flags)?
                .checked_append_reference(internal_message.serialize()?)?;

            let key = (i as u16)
                .serialize()
                .and_then(SliceData::load_cell)
                .trust_me();

            messages.set_builder(key, &item)?;
        }

        let messages = messages.serialize()?;
        let messages_hash = messages.repr_hash();

        // Build payload
        let mut payload = BuilderData::new();
        payload
            .append_u32(self.wallet_id)?
            .append_u32(expire_at)?
            .append_raw(&messages_hash.as_slice()[28..32], 32)?
            .append_builder(&messages.into())?;

        let hash = payload.clone().into_cell()?.repr_hash();

        Ok((hash, payload))
    }
}

impl TryFrom<&Cell> for InitData {
    type Error = anyhow::Error;

    fn try_from(data: &Cell) -> Result<Self, Self::Error> {
        let mut cs = SliceData::load_cell_ref(data)?;

        Ok(Self {
            wallet_id: cs.get_next_u32()?,
            last_cleaned: cs.get_next_u64()?,
            public_key: UInt256::from_be_bytes(&cs.get_next_bytes(32)?),
            data: {
                let mut map = HashmapE::with_bit_len(64);
                map.read_hashmap_data(&mut cs)?;
                map
            },
        })
    }
}

const WALLET_ID: u32 = 0x00000000;

#[derive(thiserror::Error, Debug)]
enum HighloadWalletV2Error {
    #[error("Invalid init data")]
    InvalidInitData,
    #[error("Account is frozen")]
    AccountIsFrozen,
    #[error("Too many outgoing messages")]
    TooManyGifts,
    #[error("Account init data is too large")]
    InitDataTooLarge,
}

#[cfg(test)]
pub mod tests {
    use crate::core::ton_wallet::highload_wallet_v2::InitData;
    use anyhow::Result;
    use ton_block::Deserializable;
    use ton_types::HashmapType;

    #[tokio::test]
    async fn check_state() -> Result<()> {
        let data = "te6ccgICCBAAAQAAOOsAAAIBmggHAAEBWQAAAABij1ipvO9y33lafOQTY2Zjcpu/tM7FomMbSFDp4+8Ei8aUcpwDouTBwAACAgiLsUesAl4AAwIBYgA1AAQCAnAAFAAFAgFIABEABgIBIAAKAAcCAW4ACQAIAAm3P2/0YAAJty45OOACASAADAALAAm7gCmMCAIBIAAQAA0CASAADwAOAAm3SNaR4AAJt2IEcmAACbn59J8wAgEgABMAEgAJvH0s0sQACb0fFP0kAgEgACYAFQIBIAAfABYCASAAGgAXAgEgABkAGAAJulOvR9gACbr90p9IAgFYABwAGwAJuaPhPtACASAAHgAdAAm29FO6oAAJt4e9WeACASAAJQAgAgEgACQAIQIBIAAjACIACbg4HOPQAAm5N8GT0AAJumQHj3gACbxuQUbMAgEgACwAJwIBIAApACgACbxoeVqsAgEgACsAKgAJu3kh5AgACbuT7z2oAgEgADIALQIBIAAxAC4CAUgAMAAvAAm3D/yF4AAJttuTf6AACboR+FvYAgEgADQAMwAJu/xtoCgACbswljEoAgEgAVEANgIBIADGADcCASAAfQA4AgEgAFwAOQIBIABLADoCASAASgA7AgEgAD8APAIBIAA+AD0ACbv67XUoAAm728BYuAIBIABDAEACASAAQgBBAAm4XHbOEAAJuEX0C9ACASAASQBEAgEgAEgARQIBSABHAEYACLJFqpcACLIZDQgACbb/vM5gAAm5tBZx8AAJv6qZvYYCASAAVQBMAgEgAFIATQIBWABRAE4CAWYAUABPAAizva6wAAizJMbvAAm4yrfY0AIBIABUAFMACbv97cQIAAm7rFAKCAIBIABXAFYACbwdUD3MAgEgAFkAWAAJuqMTi4gCA5B3AFsAWgAHqXo6sAAHqTSr0AIBIABsAF0CASAAYwBeAgEgAGIAXwICcQBhAGAACbU816HAAAm0goAhwAAJvMcETxwCASAAawBkAgEgAGoAZQIBIABnAGYACbjQvmEQAgJxAGkAaAAHsBsUkQAHsGq37wAJu6lVp4gACb01zX/cAgEgAHYAbQIBIABzAG4CASAAcABvAAm6QVAyuAIBSAByAHEACbd6cIWgAAm2kdMmYAIBIAB1AHQACbotzlT4AAm66wR8CAIBIAB8AHcCASAAewB4AgEgAHoA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trax3CACASADAgMBAAm7WoeimAIBIAMEAwMACbm8yrWwAAm5HOVjEAIBSAMHAwYACbpktfloAAm7LsDquAIBIAMUAwkCAVgDCwMKAAm6hR6B6AIBIAMPAwwCASADDgMNAAm2f4hroAAJtllCn+ACASADEQMQAAm2Yx6y4AIBIAMTAxIACbSflolAAAm0QerswAIBIAMYAxUCASADFwMWAAm7tkoeSAAJuknHOugACb2TEThEAgEgAzkDGgIBIAMoAxsCASADIQMcAgEgAyADHQIBIAMfAx4ACbuaE9GIAAm76TwyuAAJvTF5SlwCASADJQMiAgFmAyQDIwAJt+MXWCAACbfdIXagAgEgAycDJgAJu+jFS4gACbqhfTLYAgEgAzIDKQIBIAMvAyoCAVgDLgMrAgFYAy0DLAAJtOPh2kAACbTQbPDAAAm5hunfsAIBIAMxAzAACbqF6XDoAAm7UeL9mAIBIAM2AzMCASADNQM0AAm7xZq5KAAJuxrvMUgCASADOAM3AAm7OY2quAAJumXrW7gCASADSwM6AgEgA0YDOwIBIANBAzwCASADPgM9AAm7wUzueAIBIANAAz8ACbmxiDdwAAm5KDyKEAIBIANDA0IACbupeOFIAgEgA0UDRAAJuNLLTRAACbir+oZQAgEgA0oDRwIBIANJA0gACbonQw4YAAm6iJr9KAAJvAwRrswCASADVQNMAgEgA1QDTQIBIANPA04ACbo2gb2IAgFqA1EDUAAJtAxQcEACAVgDUwNSAAex5r87AAex7i7vAAm8a0P6FAIBIANXA1YACb0WUg0sAgEgA1kDWAAJupoiEwgACbsZNqNoAgEgA94DWwIBIAObA1wCASADeANdAgEgA3EDXgIBIANmA18CASADYwNgAgEgA2IDYQAJu3UoScgACbtyxfUoAgEgA2UDZAAJu7/lGDgACbugeIXoAgEgA24DZwIBIANrA2gCASADagNpAAm5vLA10AAJubPahdACASADbQNsAAm5tK+QUAAJuEioGPACAUgDcANvAAm5CURc8AAJuCyslfACAUgDcwNyAAm8IYMwTAIBSAN3A3QCASADdgN1AAm3DbyIoAAJti1cfKAACbgyOlvwAgEgA4oDeQIBIAOBA3oCASADgAN7AgEgA30DfAAJu0UfkFgCASADfwN+AAm4pvk5sAAJua8OZHAA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";

        let bytes = base64::decode(data).expect("cant decode data");
        let cell =
            ton_types::deserialize_tree_of_cells(&mut bytes.as_slice()).expect("deser failed");
        let account = ton_block::AccountState::construct_from_cell(cell)?;

        let data = match account {
            ton_block::AccountState::AccountActive { state_init } => state_init.data.unwrap(),
            _ => anyhow::bail!("ACCOUNT NOT ACTIVE"),
        };

        let init_data = InitData::try_from(&data).expect("init data failed");

        println!("{:?}", init_data.data.len().unwrap());

        Ok(())
    }
}
