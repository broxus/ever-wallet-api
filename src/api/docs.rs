#![allow(clippy::needless_update)]

use opg::*;

use dexpa::currency::Currency;

use crate::api::{requests, responses};

pub fn swagger() -> String {
    let api = describe_api! {
        info: {
            title: "Tokens",
            version: "1.0.0",
            description: r##"This API allows you to get the information on TIP-3 tokens"##,
        },
        servers: {
            "https://token-indexer.broxus.com/v1",
            "https://token-indexer-test.broxus.com/v1"
        },
        tags: {
            balances,
            transactions,
        },
        paths: {
            ("transactions"): {
                POST: {
                    tags: { transactions },
                    summary: "Transactions data",
                    description: "Get Transactions data.",
                    body: requests::TransactionsRequest,
                    200: responses::TransactionsInfoAndCountResponse,
                }
            },
            ("address" / String / "balances"): {
                POST: {
                    tags: { balances },
                    summary: "Balances data",
                    description: "Get address Balances data.",
                    body: requests::AddressBalancesRequest,
                    200: responses::BalancesAndCountResponse,
                }
            },
            ("address" / String / "transactions"): {
                POST: {
                    tags: { transactions },
                    summary: "Address transactions data",
                    description: "Get address transactions.",
                    body: requests::AddressTransactionsRequest,
                    200: responses::AddressTransactionsInfoResponse,
                }
            },
            ("balances"): {
                POST: {
                    tags: { balances },
                    summary: "Balances data",
                    description: "Get address Balances data.",
                    body: requests::BalancesRequest,
                    200: responses::BalancesAndCountResponse,
                }
            },
            ("token_owner" / "address" / String): {
                GET: {
                    tags: { tokens },
                    summary: "Token owner data",
                    description: "Get token owner data.",
                    200: responses::TokenOwnerResponse,
                }
            },
            ("root_contract" / "root_address" / String): {
                GET: {
                    tags: { root_contracts },
                    summary: "Root contract data",
                    description: "Get root contract data.",
                    200: responses::RootContractInfoWithTotalSupplyResponse,
                }
            },
            ("root_contract"): {
                POST: {
                    tags: { root_contracts },
                    summary: "Root contract data",
                    description: "Get root contract data.",
                    body: requests::RootTokenContractsSearchRequest,
                    200: responses::RootTokenContractsSearchResponse,
                }
            },
            ("root_contract" / "symbol_substring" / String): {
                GET: {
                    tags: { root_contracts },
                    summary: "Root contracts data",
                    description: "Get root contracts data.",
                    200: Vec<responses::RootContractInfoWithTotalSupplyResponse>,
                }
            },
            ("tokens"): {
                GET: {
                    tags: { tokens },
                    summary: "Tokens data",
                    description: "Get all tokens data.",
                    200: Vec<responses::RootTokenContractResponse>,
                }
            },
        }
    };

    serde_yaml::to_string(&api).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_docs() {
        println!("{}", swagger());
    }
}
