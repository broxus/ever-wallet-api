#![allow(clippy::needless_update)]

use opg::*;

use crate::api::*;

pub fn swagger() -> String {
    let api = describe_api! {
        info: {
            title: "TON api",
            version: "4.0.0",
            description: r##"This API allows you to use TON api"##,
        },
        servers: {
            "https://ton-api.broxus.com/ton/v4",
            "https://ton-api-test.broxus.com/ton/v4"
        },
        tags: {
            addresses,
            transactions,
            tokens,
            events,
            metrics,
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
                    200: responses::AddressBalanceResponse,
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
            ("transactions" / "mh" / String): {
                GET: {
                    tags: { transactions },
                    summary: "Get transaction",
                    description: "Get transaction by message hash.",
                    200: responses::AccountTransactionResponse,
                }
            },
            ("transactions" / "h" / String): {
                GET: {
                    tags: { transactions },
                    summary: "Get transaction",
                    description: "Get transaction by transaction hash.",
                    200: responses::AccountTransactionResponse,
                }
            },
            ("transactions" / "id" / String): {
                GET: {
                    tags: { transactions },
                    summary: "Get transaction",
                    description: "Get transaction by id.",
                    200: responses::AccountTransactionResponse,
                }
            },
            ("events" ): {
                POST: {
                    tags: { events },
                    summary: "Get events",
                    description: "Get events.",
                    body: requests::PostTonTransactionEventsRequest,
                    200: responses::TonEventsResponse,
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
            ("events" / "mark" / "all" ): {
                POST: {
                    tags: { events },
                    summary: "Mark events",
                    description: "Mark events by status optional.",
                    body: requests::MarkAllTransactionEventRequest,
                    200: responses::MarkEventsResponse,
                }
            },
             ("tokens" / "address" / String): {
                GET: {
                    tags: { addresses, tokens },
                    summary: "Address tokens balances",
                    description: "Get  address tokens balances.",
                    200: responses::AccountTokenBalanceResponse,
                }
            },
            ("tokens" / "transactions" / "create"): {
                POST: {
                    tags: { transactions, tokens },
                    summary: "Create token transaction",
                    description: "Send token transaction.",
                    body: requests::PostTonTokenTransactionSendRequest,
                    200: responses::AccountTokenTransactionResponse,
                }
            },
            ("tokens" / "transactions" / "mh" / String): {
                GET: {
                    tags: { transactions, tokens  },
                    summary: "Get tokens transaction",
                    description: "Get tokens transaction by message hash.",
                    200: responses::AccountTokenTransactionResponse,
                }
            },
            ("tokens" / "transactions" / "id" / String): {
                GET: {
                    tags: { transactions, tokens },
                    summary: "Get tokens transaction",
                    description: "Get tokens transaction by id.",
                    200: responses::AccountTokenTransactionResponse,
                }
            },
            ("tokens" / "events" ): {
                POST: {
                    tags: { events, tokens },
                    summary: "Get token events",
                    description: "Get token events.",
                    body: requests::PostTonTokenTransactionEventsRequest,
                    200: responses::TonTokenEventsResponse,
                }
            },
            ("tokens" / "events" / "mark" ): {
                POST: {
                    tags: { events, tokens },
                    summary: "Mark tokens event",
                    description: "Mark tokens event by id.",
                    body: requests::PostTonTokenMarkEventsRequest,
                    200: responses::MarkTokenEventsResponse,
                }
            },
            ("metrics"): {
                GET: {
                    tags: { metrics  },
                    summary: "Get metrics",
                    description: "Get metrics of api health.",
                    200: responses::MetricsResponse,
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
