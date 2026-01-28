use std::borrow::Cow;
use std::convert::TryFrom;

use anyhow::Result;
use ed25519_dalek::PublicKey;
use tycho_types::{
    abi::{AbiValue, FromAbi, Function, IntoAbi, NamedAbiValue, UnsignedExternalMessage},
    cell::{Cell, CellBuilder, CellDataBuilder, CellFamily, HashBytes, Load},
    dict::RawDict,
    models::{Account, StateInit, StdAddr},
};
use tycho_util::time::Clock;

use crate::utils::ton_wallet::{MessageFlags, MultisigPendingTransaction, MultisigPendingUpdate};

use super::{Gift, TonWalletDetails};

#[derive(Copy, Clone, Debug)]
pub struct DeployParams<'a> {
    pub owners: &'a [PublicKey],
    pub req_confirms: u8,
    pub expiration_time: Option<u32>,
}

impl<'a> DeployParams<'a> {
    pub fn single_custodian(pubkey: &'a PublicKey) -> Self {
        Self {
            owners: std::slice::from_ref(pubkey),
            req_confirms: 1,
            expiration_time: None,
        }
    }
}

pub fn prepare_deploy(
    public_key: &PublicKey,
    multisig_type: MultisigType,
    workchain: i8,
    expire_at: u32,
    params: DeployParams<'_>,
) -> Result<UnsignedExternalMessage> {
    let state_init = prepare_state_init(public_key, multisig_type)?;
    let cell_builder = CellBuilder::build_from(&state_init)?;
    let hash = cell_builder.repr_hash();

    let dst = StdAddr::new(workchain, *hash);

    let owners = params
        .owners
        .iter()
        .map(|public_key| HashBytes(public_key.to_bytes()))
        .collect::<Vec<HashBytes>>();

    let is_new_multisig = multisig_type.is_multisig2();
    let function = if is_new_multisig {
        crate::utils::wallets::multisig2::constructor()
    } else if params.expiration_time.is_none() {
        crate::utils::wallets::multisig::constructor()
    } else {
        return Err(MultisigError::CustomExpirationTimeNotSupported.into());
    };

    let mut abi_values = vec![
        owners.as_abi().named("owners"),
        params.req_confirms.as_abi().named("reqConfirms"),
    ];
    if is_new_multisig {
        abi_values.push(
            params
                .expiration_time
                .unwrap_or(DEFAULT_LIFETIME)
                .as_abi()
                .named("lifetime"),
        );
    }

    let external_input = function.encode_external(&abi_values);

    let unsigned_body = external_input.with_expire_at(expire_at).build_input()?;
    let mut unsigned_message = unsigned_body.with_dst(dst);
    unsigned_message.set_state_init(Some(state_init));
    Ok(unsigned_message)
}

pub fn prepare_confirm_transaction(
    multisig_type: MultisigType,
    public_key: &PublicKey,
    address: StdAddr,
    transaction_id: u64,
    expire_at: u32,
) -> Result<UnsignedExternalMessage> {
    let function = if multisig_type.is_multisig2() {
        crate::utils::wallets::multisig2::confirm_transaction()
    } else {
        crate::utils::wallets::multisig::confirm_transaction()
    };

    make_ext_message(
        public_key,
        address,
        expire_at,
        function,
        vec![transaction_id.as_abi().named("transactionId")],
    )
}

pub fn prepare_transfer(
    multisig_type: MultisigType,
    public_key: &PublicKey,
    has_multiple_owners: bool,
    address: StdAddr,
    gift: Gift,
    expire_at: u32,
) -> Result<UnsignedExternalMessage> {
    let is_new_multisig = multisig_type.is_multisig2();

    let (function, input) = if has_multiple_owners || is_new_multisig && gift.state_init.is_some() {
        let all_balance = match MessageFlags::try_from(gift.flags) {
            Ok(MessageFlags::Normal) => false,
            Ok(MessageFlags::AllBalance) => true,
            _ => return Err(MultisigError::UnsupportedFlagsSet.into()),
        };

        let function = if is_new_multisig {
            crate::utils::wallets::multisig2::submit_transaction()
        } else {
            crate::utils::wallets::multisig::submit_transaction()
        };

        let mut named_abi_values = vec![
            AbiValue::address(gift.destination).named("destination"),
            AbiValue::uint(128, gift.amount).named("amount"),
            AbiValue::Bool(gift.bounce).named("bounce"),
            AbiValue::uint(8, all_balance).named("flags"),
            AbiValue::Cell(gift.body.unwrap_or_default()).named("body"),
        ];

        if is_new_multisig {
            named_abi_values.push(
                gift.state_init
                    .map(|state_init| CellBuilder::build_from(&state_init))
                    .transpose()?
                    .as_abi()
                    .named("stateInit"),
            );
        }
        (function, named_abi_values)
    } else {
        let function = if is_new_multisig {
            crate::utils::wallets::multisig2::send_transaction()
        } else {
            crate::utils::wallets::multisig::send_transaction()
        };
        let named_abi_values = vec![
            AbiValue::address(gift.destination).named("destination"),
            AbiValue::uint(128, gift.amount).named("amount"),
            AbiValue::Bool(gift.bounce).named("bounce"),
            AbiValue::uint(8, gift.flags).named("flags"),
            AbiValue::Cell(gift.body.unwrap_or_default()).named("body"),
        ];
        (function, named_abi_values)
    };

    make_ext_message(public_key, address, expire_at, function, input)
}

pub fn prepare_code_update(
    multisig_type: MultisigType,
    public_key: &PublicKey,
    address: StdAddr,
    new_code_hash: &[u8; 32],
    expire_at: u32,
) -> Result<UnsignedExternalMessage> {
    use crate::utils::wallets::multisig2;

    if !multisig_type.is_multisig2() {
        return Err(MultisigError::UnsupportedUpdate.into());
    }

    make_ext_message(
        public_key,
        address,
        expire_at,
        multisig2::submit_update(),
        multisig2::SubmitUpdateParams {
            code_hash: Some(HashBytes::from(*new_code_hash)),
            owners: None,
            req_confirms: None,
            lifetime: None,
        }
        .abi_values(),
    )
}

pub fn prepare_confirm_update(
    multisig_type: MultisigType,
    public_key: &PublicKey,
    address: StdAddr,
    update_id: u64,
    expire_at: u32,
) -> Result<UnsignedExternalMessage> {
    use crate::utils::wallets::multisig2;

    if !multisig_type.is_multisig2() {
        return Err(MultisigError::UnsupportedUpdate.into());
    }

    make_ext_message(
        public_key,
        address,
        expire_at,
        multisig2::confirm_update(),
        multisig2::ConfirmUpdateParams { update_id }.abi_values(),
    )
}

pub fn prepare_execute_update(
    multisig_type: MultisigType,
    public_key: &PublicKey,
    update_id: u64,
    code: Option<Cell>,
    address: StdAddr,
    expire_at: u32,
) -> Result<UnsignedExternalMessage> {
    use crate::utils::wallets::multisig2;

    if !multisig_type.is_multisig2() {
        return Err(MultisigError::UnsupportedUpdate.into());
    }

    make_ext_message(
        public_key,
        address,
        expire_at,
        multisig2::execute_update(),
        multisig2::ExecuteUpdateParams { update_id, code }.abi_values(),
    )
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum MultisigType {
    SafeMultisigWallet,
    SafeMultisigWallet24h,
    SetcodeMultisigWallet,
    SetcodeMultisigWallet24h,
    BridgeMultisigWallet,
    SurfWallet,
    Multisig2,
    Multisig2_1,
}

impl MultisigType {
    #[inline(always)]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::SafeMultisigWallet => "SafeMultisigWallet",
            Self::SafeMultisigWallet24h => "SafeMultisigWallet24h",
            Self::SetcodeMultisigWallet => "SetcodeMultisigWallet",
            Self::SetcodeMultisigWallet24h => "SetcodeMultisigWallet24h",
            Self::BridgeMultisigWallet => "BridgeMultisigWallet",
            Self::SurfWallet => "SurfWallet",
            Self::Multisig2 => "Multisig2",
            Self::Multisig2_1 => "Multisig2_1",
        }
    }
}

impl std::str::FromStr for MultisigType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "SafeMultisigWallet" => Self::SafeMultisigWallet,
            "SafeMultisigWallet24h" => Self::SafeMultisigWallet24h,
            "SetcodeMultisigWallet" => Self::SetcodeMultisigWallet,
            "SetcodeMultisigWallet24h" => Self::SetcodeMultisigWallet24h,
            "BridgeMultisigWallet" => Self::BridgeMultisigWallet,
            "SurfWallet" => Self::SurfWallet,
            "Multisig2" => Self::Multisig2,
            "Multisig2_1" => Self::Multisig2_1,
            _ => return Err(anyhow::anyhow!("Invalid multisig type")),
        })
    }
}

impl std::fmt::Display for MultisigType {
    fn fmt(&self, f: &'_ mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl MultisigType {
    pub fn is_multisig2(self) -> bool {
        matches!(self, Self::Multisig2 | Self::Multisig2_1)
    }

    pub fn is_updatable(&self) -> bool {
        matches!(
            self,
            Self::SetcodeMultisigWallet
                | Self::SetcodeMultisigWallet24h
                | Self::SurfWallet
                | Self::Multisig2
                | Self::Multisig2_1
        )
    }

    pub fn state_init(&self) -> Result<StateInit> {
        use crate::utils::wallets;

        let state_init = match self {
            MultisigType::SafeMultisigWallet => wallets::code::safe_multisig_wallet(),
            MultisigType::SafeMultisigWallet24h => wallets::code::safe_multisig_wallet_24h(),
            MultisigType::SetcodeMultisigWallet => wallets::code::setcode_multisig_wallet(),
            MultisigType::SetcodeMultisigWallet24h => wallets::code::setcode_multisig_wallet_24h(),
            MultisigType::BridgeMultisigWallet => wallets::code::bridge_multisig_wallet(),
            MultisigType::SurfWallet => wallets::code::surf_wallet(),
            MultisigType::Multisig2 => wallets::code::multisig2(),
            MultisigType::Multisig2_1 => wallets::code::multisig2_1(),
        };
        StateInit::load_from(&mut state_init.as_slice()?).map_err(Into::into)
    }

    pub fn code_hash(&self) -> &[u8; 32] {
        match self {
            Self::SafeMultisigWallet => SAFE_MULTISIG_WALLET_HASH,
            Self::SafeMultisigWallet24h => SAFE_MULTISIG_WALLET_24H_HASH,
            Self::SetcodeMultisigWallet => SETCODE_MULTISIG_WALLET_HASH,
            Self::SetcodeMultisigWallet24h => SETCODE_MULTISIG_WALLET_24H_HASH,
            Self::BridgeMultisigWallet => BRIDGE_MULTISIG_WALLET_HASH,
            Self::SurfWallet => SURF_WALLET_HASH,
            Self::Multisig2 => MULTISIG2_HASH,
            Self::Multisig2_1 => MULTISIG2_1_HASH,
        }
    }

    pub fn code(&self) -> Result<Cell> {
        self.state_init()?
            .code
            .ok_or(UnpackerError::InvalidAbi.into())
    }
}

static SAFE_MULTISIG_WALLET_HASH: &[u8; 32] = &[
    0x80, 0xd6, 0xc4, 0x7c, 0x4a, 0x25, 0x54, 0x3c, 0x9b, 0x39, 0x7b, 0x71, 0x71, 0x6f, 0x3f, 0xae,
    0x1e, 0x2c, 0x5d, 0x24, 0x71, 0x74, 0xc5, 0x2e, 0x2c, 0x19, 0xbd, 0x89, 0x64, 0x42, 0xb1, 0x05,
];
static SAFE_MULTISIG_WALLET_24H_HASH: &[u8; 32] = &[
    0x7d, 0x09, 0x96, 0x94, 0x34, 0x06, 0xf7, 0xd6, 0x2a, 0x4f, 0xf2, 0x91, 0xb1, 0x22, 0x8b, 0xf0,
    0x6e, 0xbd, 0x3e, 0x04, 0x8b, 0x58, 0x43, 0x6c, 0x5b, 0x70, 0xfb, 0x77, 0xff, 0x8b, 0x4b, 0xf2,
];
static SETCODE_MULTISIG_WALLET_HASH: &[u8; 32] = &[
    0xe2, 0xb6, 0x0b, 0x6b, 0x60, 0x2c, 0x10, 0xce, 0xd7, 0xea, 0x8e, 0xde, 0x4b, 0xdf, 0x96, 0x34,
    0x2c, 0x97, 0x57, 0x0a, 0x37, 0x98, 0x06, 0x6f, 0x3f, 0xb5, 0x0a, 0x4b, 0x2b, 0x27, 0xa2, 0x08,
];
static SETCODE_MULTISIG_WALLET_24H_HASH: &[u8; 32] = &[
    0xa4, 0x91, 0x80, 0x4c, 0xa5, 0x5d, 0xd5, 0xb2, 0x8c, 0xff, 0xdf, 0xf4, 0x8c, 0xb3, 0x41, 0x42,
    0x93, 0x09, 0x99, 0x62, 0x1a, 0x54, 0xac, 0xee, 0x6b, 0xe8, 0x3c, 0x34, 0x20, 0x51, 0xd8, 0x84,
];
static BRIDGE_MULTISIG_WALLET_HASH: &[u8; 32] = &[
    0xf3, 0xa0, 0x7a, 0xe8, 0x4f, 0xc3, 0x43, 0x25, 0x9d, 0x7f, 0xa4, 0x84, 0x7b, 0x86, 0x33, 0x5b,
    0x3f, 0xdc, 0xfc, 0x8b, 0x31, 0xf1, 0xba, 0x4b, 0x7a, 0x94, 0x99, 0xd5, 0x53, 0x0f, 0x0b, 0x18,
];
static SURF_WALLET_HASH: &[u8; 32] = &[
    0x20, 0x7d, 0xc5, 0x60, 0xc5, 0x95, 0x6d, 0xe1, 0xa2, 0xc1, 0x47, 0x93, 0x56, 0xf8, 0xf3, 0xee,
    0x70, 0xa5, 0x97, 0x67, 0xdb, 0x2b, 0xf4, 0x78, 0x8b, 0x1d, 0x61, 0xad, 0x42, 0xcd, 0xad, 0x82,
];
static MULTISIG2_HASH: &[u8; 32] = &[
    0x29, 0xb2, 0x47, 0x76, 0xb3, 0xdf, 0x6a, 0x05, 0xc5, 0xa1, 0xb8, 0xd8, 0xfd, 0x75, 0xcb, 0x72,
    0xa1, 0xd3, 0x3c, 0x0a, 0x44, 0x38, 0x53, 0x32, 0xa8, 0xbf, 0xc2, 0x72, 0x7f, 0xb6, 0x65, 0x90,
];
static MULTISIG2_1_HASH: &[u8; 32] = &[
    0xd6, 0x6d, 0x19, 0x87, 0x66, 0xab, 0xdb, 0xe1, 0x25, 0x3f, 0x34, 0x15, 0x82, 0x6c, 0x94, 0x6c,
    0x37, 0x1f, 0x51, 0x12, 0x55, 0x24, 0x08, 0x62, 0x5a, 0xeb, 0x0b, 0x31, 0xe0, 0xef, 0x2d, 0xf3,
];

pub fn guess_multisig_type(code_hash: &HashBytes) -> Option<MultisigType> {
    match code_hash.as_slice() {
        s if s == SAFE_MULTISIG_WALLET_HASH => Some(MultisigType::SafeMultisigWallet),
        s if s == SAFE_MULTISIG_WALLET_24H_HASH => Some(MultisigType::SafeMultisigWallet24h),
        s if s == SETCODE_MULTISIG_WALLET_HASH => Some(MultisigType::SetcodeMultisigWallet),
        s if s == BRIDGE_MULTISIG_WALLET_HASH => Some(MultisigType::BridgeMultisigWallet),
        s if s == SETCODE_MULTISIG_WALLET_24H_HASH => Some(MultisigType::SetcodeMultisigWallet24h),
        s if s == SURF_WALLET_HASH => Some(MultisigType::SurfWallet),
        s if s == MULTISIG2_HASH => Some(MultisigType::Multisig2),
        s if s == MULTISIG2_1_HASH => Some(MultisigType::Multisig2_1),
        _ => None,
    }
}

pub fn compute_contract_address(
    public_key: &PublicKey,
    multisig_type: MultisigType,
    workchain_id: i8,
) -> Result<StdAddr> {
    let state_init = prepare_state_init(public_key, multisig_type)?;
    let cell_builder = CellBuilder::build_from(&state_init)?;
    let hash = cell_builder.repr_hash();
    Ok(StdAddr::new(workchain_id, *hash))
}

pub fn ton_wallet_details(multisig_type: MultisigType) -> TonWalletDetails {
    TonWalletDetails {
        requires_separate_deploy: true,
        min_amount: if multisig_type.is_multisig2() {
            0
        } else {
            1000000 // 0.001 EVER
        },
        max_messages: 1,
        supports_payload: true,
        supports_state_init: multisig_type.is_multisig2(),
        supports_multiple_owners: true,
        supports_code_update: multisig_type.is_updatable(),
        expiration_time: match multisig_type {
            MultisigType::SafeMultisigWallet
            | MultisigType::SetcodeMultisigWallet
            | MultisigType::Multisig2
            | MultisigType::Multisig2_1 => 3600,
            MultisigType::SurfWallet => 3601,
            MultisigType::SafeMultisigWallet24h
            | MultisigType::SetcodeMultisigWallet24h
            | MultisigType::BridgeMultisigWallet => 86400,
        },
        required_confirmations: None,
    }
}

pub fn prepare_state_init(
    public_key: &PublicKey,
    multisig_type: MultisigType,
) -> Result<StateInit> {
    let mut state_init = multisig_type.state_init()?;

    let mut result = if state_init.data.is_none() {
        RawDict::new()
    } else {
        RawDict::<64>::from(state_init.data)
    };

    let context = Cell::empty_context();
    let mut key_builder = CellDataBuilder::new();

    key_builder.store_u64(0)?;
    result.set_ext(
        key_builder.as_data_slice(),
        &CellBuilder::from_raw_data(public_key.as_bytes(), 256)?.as_data_slice(),
        context,
    )?;

    // Encode init data as mapping
    let cell = CellBuilder::build_from_ext(result, context)?;
    state_init.data = Some(cell);

    Ok(state_init)
}

fn run_local(
    clock: &dyn Clock,
    function: &Function,
    account_stuff: Account,
) -> Result<Vec<NamedAbiValue>> {
    unimplemented!()
    //let ExecutionOutput {
    //    tokens,
    //    result_code,
    //} = function.run_local(clock, account_stuff, &[], &[])?;
    //tokens.ok_or_else(|| MultisigError::NonZeroResultCode(result_code).into())
}

#[derive(Copy, Clone)]
pub struct MultisigParamsPrefix {
    pub max_queued_transactions: u8,
    pub max_custodian_count: u8,
    pub expiration_time: u64,
    pub min_value: u128,
    pub required_confirms: u8,
}

impl TryFrom<Vec<NamedAbiValue>> for MultisigParamsPrefix {
    fn try_from(params: Vec<NamedAbiValue>) -> std::result::Result<Self, Self::Error> {
        let mut params_iter = params.into_iter();

        let Some(AbiValue::Uint(8, max_queued_transactions)) = params_iter.next().map(|v| v.value)
        else {
            return Err(MultisigError::InvalidParams.into());
        };

        let Some(AbiValue::Uint(8, max_custodian_count)) = params_iter.next().map(|v| v.value)
        else {
            return Err(MultisigError::InvalidParams.into());
        };

        let Some(AbiValue::Uint(64, expiration_time)) = params_iter.next().map(|v| v.value) else {
            return Err(MultisigError::InvalidParams.into());
        };

        let Some(AbiValue::Uint(128, min_value)) = params_iter.next().map(|v| v.value) else {
            return Err(MultisigError::InvalidParams.into());
        };

        let Some(AbiValue::Uint(8, required_confirms)) = params_iter.next().map(|v| v.value) else {
            return Err(MultisigError::InvalidParams.into());
        };

        Ok(MultisigParamsPrefix {
            max_queued_transactions: u8::try_from(max_queued_transactions)?,
            max_custodian_count: u8::try_from(max_custodian_count)?,
            expiration_time: u64::try_from(expiration_time)?,
            min_value: u128::try_from(min_value)?,
            required_confirms: u8::try_from(required_confirms)?,
        })
    }

    type Error = anyhow::Error;
}

pub fn get_params(
    clock: &dyn Clock,
    multisig_type: MultisigType,
    account: Cow<'_, Account>,
) -> Result<MultisigParamsPrefix> {
    let function = match multisig_type {
        MultisigType::Multisig2 | MultisigType::Multisig2_1 => {
            crate::utils::wallets::multisig2::get_parameters()
        }
        MultisigType::SafeMultisigWallet
        | MultisigType::SafeMultisigWallet24h
        | MultisigType::BridgeMultisigWallet => {
            crate::utils::wallets::multisig::safe_multisig::get_parameters()
        }
        MultisigType::SetcodeMultisigWallet
        | MultisigType::SetcodeMultisigWallet24h
        | MultisigType::SurfWallet => {
            crate::utils::wallets::multisig::set_code_multisig::get_parameters()
        }
    };

    let output = run_local(clock, function, account.into_owned())?;
    MultisigParamsPrefix::try_from(output)
}

pub fn get_custodians(
    clock: &dyn Clock,
    multisig_type: MultisigType,
    account: Cow<'_, Account>,
) -> Result<Vec<HashBytes>> {
    let function = if multisig_type.is_multisig2() {
        crate::utils::wallets::multisig2::get_custodians()
    } else {
        crate::utils::wallets::multisig::get_custodians()
    };
    run_local(clock, function, account.into_owned()).and_then(parse_multisig_contract_custodians)
}

fn parse_multisig_contract_custodians(tokens: Vec<NamedAbiValue>) -> Result<Vec<HashBytes>> {
    let array = match tokens.into_iter().next().map(|v| v.value) {
        Some(AbiValue::Array(_, tokens)) => tokens,
        _ => return Err(UnpackerError::InvalidAbi.into()),
    };

    let mut custodians = array
        .into_iter()
        .map(crate::utils::wallets::multisig::MultisigCustodian::from_abi)
        .collect::<Result<Vec<crate::utils::wallets::multisig::MultisigCustodian>, _>>()?;

    custodians.sort_by(|a, b| a.index.cmp(&b.index));

    Ok(custodians.into_iter().map(|item| item.pubkey).collect())
}

pub fn find_pending_transaction(
    clock: &dyn Clock,
    multisig_type: MultisigType,
    account: Cow<'_, Account>,
    pending_transaction_id: u64,
) -> Result<bool> {
    #[derive(Copy, Clone)]
    pub struct MultisigTransactionId {
        pub id: u64,
    }

    impl TryFrom<AbiValue> for MultisigTransactionId {
        fn try_from(params: AbiValue) -> std::result::Result<Self, Self::Error> {
            let AbiValue::Tuple(params) = params else {
                return Err(anyhow::anyhow!("Invalid params"));
            };

            let mut params_iter = params.into_iter();

            let Some(AbiValue::Uint(64, index)) = params_iter.next().map(|v| v.value) else {
                return Err(anyhow::anyhow!("Invalid params"));
            };

            Ok(MultisigTransactionId {
                id: u64::try_from(index)?,
            })
        }

        type Error = anyhow::Error;
    }

    let function = if multisig_type.is_multisig2() {
        crate::utils::wallets::multisig2::get_transactions()
    } else {
        crate::utils::wallets::multisig::get_transactions()
    };

    let tokens = run_local(clock, function, account.into_owned())?;

    let array = match tokens.into_iter().next().map(|v| v.value) {
        Some(AbiValue::Array(_, tokens)) => tokens,
        _ => return Err(UnpackerError::InvalidAbi.into()),
    };

    for item in array {
        let m = MultisigTransactionId::try_from(item)?;
        if pending_transaction_id == m.id {
            return Ok(true);
        }
    }
    Ok(false)
}

pub fn find_pending_update(
    clock: &dyn Clock,
    multisig_type: MultisigType,
    account: Cow<'_, Account>,
    update_id: u64,
) -> Result<Option<UpdatedParams>> {
    use crate::utils::wallets::multisig2;

    let function = match multisig_type {
        MultisigType::Multisig2 => multisig2::v2_0::get_update_requests(),
        MultisigType::Multisig2_1 => multisig2::v2_1::get_update_requests(),
        _ => return Ok(None),
    };

    let tokens = run_local(clock, function, account.into_owned())?;

    let array = match tokens.into_iter().next().map(|v| v.value) {
        Some(AbiValue::Array(_, tokens)) => tokens,
        _ => return Err(UnpackerError::InvalidAbi.into()),
    };

    for item in array {
        let update = multisig2::UpdateTransaction::from_abi(item)?;
        if update_id == update.id {
            return Ok(Some(UpdatedParams {
                new_code_hash: update.new_code_hash,
                new_custodians: update.new_custodians,
                new_req_confirms: update.new_req_confirms,
                new_lifetime: update.new_lifetime,
            }));
        }
    }

    Ok(None)
}

#[derive(Debug, Clone)]
pub struct UpdatedParams {
    pub new_code_hash: Option<HashBytes>,
    pub new_custodians: Option<Vec<HashBytes>>,
    pub new_req_confirms: Option<u8>,
    pub new_lifetime: Option<u32>,
}


pub fn get_pending_transactions(
    clock: &dyn Clock,
    multisig_type: MultisigType,
    account: Cow<'_, Account>,
    custodians: &[HashBytes],
) -> Result<Vec<MultisigPendingTransaction>> {
    let function = if multisig_type.is_multisig2() {
        crate::utils::wallets::multisig2::get_transactions()
    } else {
        crate::utils::wallets::multisig::get_transactions()
    };
    run_local(clock, function, account.into_owned()).and_then(|tokens| {
        let array = match tokens.into_iter().next().map(|v| v.value) {
            Some(AbiValue::Array(_, tokens)) => tokens,
            _ => return Err(UnpackerError::InvalidAbi.into()),
        };

        let transactions = array
            .into_iter()
            .map(|item| {
                Ok(extend_pending_transaction(
                    crate::utils::wallets::multisig::MultisigTransaction::from_abi(item)?,
                    custodians,
                ))
            })
            .collect::<Result<Vec<MultisigPendingTransaction>>>()?;

        Ok(transactions)
    })
}

pub fn get_pending_updates(
    clock: &dyn Clock,
    multisig_type: MultisigType,
    account: Cow<'_, Account>,
    custodians: &[HashBytes],
) -> Result<Vec<MultisigPendingUpdate>> {
    use crate::utils::wallets::multisig2;

    let function = match multisig_type {
        MultisigType::Multisig2 => multisig2::v2_0::get_update_requests(),
        MultisigType::Multisig2_1 => multisig2::v2_1::get_update_requests(),
        _ => return Ok(Vec::new()),
    };

    run_local(clock, function, account.into_owned()).and_then(|tokens| {
        let array = match tokens.into_iter().next().map(|v| v.value) {
            Some(AbiValue::Array(_, tokens)) => tokens,
            _ => return Err(UnpackerError::InvalidAbi.into()),
        };

        let updates = array
            .into_iter()
            .map(|item| {
                Ok(extend_pending_update(
                    crate::utils::wallets::multisig2::UpdateTransaction::from_abi(item)?,
                    custodians,
                ))
            })
            .collect::<Result<Vec<MultisigPendingUpdate>>>()?;

        Ok(updates)
    })
}

fn extend_pending_transaction(
    tx: crate::utils::wallets::multisig::MultisigTransaction,
    custodians: &[HashBytes],
) -> MultisigPendingTransaction {
    let confirmations = custodians
        .iter()
        .enumerate()
        .filter(|(i, _)| (0b1 << i) & tx.confirmation_mask != 0)
        .map(|(_, item)| *item)
        .collect::<Vec<HashBytes>>();

    MultisigPendingTransaction {
        id: tx.id,
        confirmations,
        signs_required: tx.signs_required,
        signs_received: tx.signs_received,
        creator: tx.creator,
        index: tx.index,
        dest: tx.dest,
        value: tx.value.into(),
        send_flags: tx.send_flags,
        payload: tx.payload,
        bounce: tx.bounce,
    }
}

fn extend_pending_update(
    tx: crate::utils::wallets::multisig2::UpdateTransaction,
    custodians: &[HashBytes],
) -> MultisigPendingUpdate {
    let confirmations = custodians
        .iter()
        .enumerate()
        .filter(|(i, _)| (0b1 << i) & tx.confirmations_mask != 0)
        .map(|(_, item)| *item)
        .collect::<Vec<HashBytes>>();

    MultisigPendingUpdate {
        id: tx.id,
        confirmations,
        signs_received: tx.signs,
        creator: tx.creator,
        index: tx.index,
        new_code_hash: tx.new_code_hash,
        new_custodians: tx.new_custodians,
        new_req_confirms: tx.new_req_confirms,
        new_lifetime: tx.new_lifetime,
    }
}

fn make_ext_message(
    public_key: &PublicKey,
    address: StdAddr,
    expire_at: u32,
    function: &'static Function,
    input: Vec<NamedAbiValue>,
) -> Result<UnsignedExternalMessage> {
    let external_input = function.encode_external(&input);
    let unsigned_body = external_input.with_expire_at(expire_at).build_input()?;
    let unsigned_message = unsigned_body.with_dst(address);

    Ok(unsigned_message)
}

const DEFAULT_LIFETIME: u32 = 3600;

#[derive(thiserror::Error, Debug)]
enum MultisigError {
    #[error("Non-zero execution result code: {}", .0)]
    NonZeroResultCode(i32),
    #[error("Unsupported message flags set")]
    UnsupportedFlagsSet,
    #[error("Custom lifetime is not supported for this contract type")]
    CustomExpirationTimeNotSupported,
    #[error("Update is not supported or not implemented for this contract type")]
    UnsupportedUpdate,
    #[error("Invalid params")]
    InvalidParams,
}

pub type UnpackerResult<T> = Result<T, UnpackerError>;

#[derive(thiserror::Error, Debug, Clone, Copy)]
pub enum UnpackerError {
    #[error("Invalid ABI")]
    InvalidAbi,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn correct_address() {
        let key = PublicKey::from_bytes(
            &hex::decode("5ace46d93d8f3932499df9f2bc7ef787385e16965e7797258948febd186de7f6")
                .unwrap(),
        )
        .unwrap();

        assert_eq!(
            compute_contract_address(&key, MultisigType::SetcodeMultisigWallet24h, 0)
                .unwrap()
                .to_string(),
            "0:3de70f9212154344a3158768b3fed731fc865ca15948b0d6d0d34daf4c6a7a0a"
        );
    }
}
