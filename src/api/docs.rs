#![allow(clippy::needless_update)]

use opg::*;

use crate::api::{requests, responses};

pub fn swagger() -> String {
    let api = describe_api! {
        info: {
            title: "TON api",
            version: "1.0.0",
            description: r##"This API allows you to use TON api"##,
        },
        servers: {
            "https://ton-api.broxus.com/ton/v4",
            "https://ton-api-test.broxus.com/ton/v4"
        },
        tags: {
            addresses,
            transactions,
            events,
        },
        paths: {
            ("address" / "create"): {
                POST: {
                    tags: { addresses },
                    summary: "Address creation",
                    description: "Create user address.",
                    body: requests::CreateAddressRequest,
                    200: responses::AccountAddressResponse,
                }
            },
            ("address" / "check"): {
                POST: {
                    tags: { addresses },
                    summary: "Check address",
                    description: "Check correction of TON address.",
                    body: requests::PostAddressBalanceRequest,
                    200: responses::PostCheckedAddressResponse,
                }
            },
             ("address" / String): {
                GET: {
                    tags: { addresses },
                    summary: "Address balance",
                    description: "Get address balance.",
                    200: responses::AccountAddressResponse,
                }
            },
            ("transactions" / "create"): {
                POST: {
                    tags: { transactions },
                    summary: "Create transaction",
                    description: "Send transaction.",
                    body: requests::PostTonTransactionSendRequest,
                    200: responses::AccountTransactionResponse,
                }
            },
            ("transactions" / "mh"): {
                GET: {
                    tags: { transactions },
                    summary: "Get transaction",
                    description: "Get transaction by message hash.",
                    200: responses::AccountTransactionResponse,
                }
            },
            ("transactions" / "h"): {
                GET: {
                    tags: { transactions },
                    summary: "Get transaction",
                    description: "Get transaction by transaction hash.",
                    200: responses::AccountTransactionResponse,
                }
            },
            ("events" / "mark" ): {
                POST: {
                    tags: { events },
                    summary: "Mark event",
                    description: "Mark event by id.",
                    body: requests::PostTonMarkEventsRequest,
                    200: responses::MarkEventsResponse,
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
