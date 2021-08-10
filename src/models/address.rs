#[derive(
    Clone,
    Debug,
    Default,
    derive_more::Display,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    derive_more::From,
    derive_more::FromStr,
    derive_more::Into,
    serde::Serialize,
    serde::Deserialize,
    opg::OpgModel,
)]
#[opg(inline, string)]
pub struct Address(pub String);
