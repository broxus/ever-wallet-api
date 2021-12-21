#![allow(clippy::needless_update)]

use opg::*;

use crate::api::*;
use crate::models::*;

pub fn swagger() -> String {
    let api = describe_api! {
        info: {
            title: "TON api",
            version: "4.0.0",
            description: r##"This API allows you to use TON api"##,
        },
        servers: {
            "https://ton-api.broxus.com/ton/v3",
            "https://ton-api-test.broxus.com/ton/v3"
        },
        tags: {
            addresses,
            transactions,
            tokens,
            events,
            metrics,
        },
        paths: {
            ("address" / "check"): {
                POST: {
                    tags: { addresses },
                    summary: "Check address",
                    description: "Check correction of TON address.",
                    parameters: { (header "api-key") },
                    body: requests::PostAddressBalanceRequest,
                    200: responses::PostCheckedAddressResponse,
                }
            },
            ("address" / "create"): {
                POST: {
                    tags: { addresses },
                    summary: "Address creation",
                    description: "Create user address.",
                    parameters: { (header "api-key") },
                    body: requests::CreateAddressRequest,
                    200: responses::AccountAddressResponse,
                }
            },
             ("address" / String): {
                GET: {
                    tags: { addresses },
                    summary: "Address balance",
                    description: "Get address balance.",
                    parameters: { (header "api-key") },
                    200: responses::AddressBalanceResponse,
                }
            },
             ("address" / String / "info"): {
                GET: {
                    tags: { addresses },
                    summary: "Address info",
                    description: "Get address info.",
                    parameters: { (header "api-key") },
                    200: responses::AddressInfoResponse,
                }
            },
            ("transactions" / "create"): {
                POST: {
                    tags: { transactions },
                    summary: "Create transaction",
                    description: "Send transaction.",
                    parameters: { (header "api-key") },
                    body: requests::PostTonTransactionSendRequest,
                    200: responses::AccountTransactionResponse,
                    callbacks: {
                        transactionSent: {
                            ("callbackUrl"): {
                                POST: {
                                    description: "Event transaction sent. If address are not \
                                    deployed the event will be sent a twice since in this case \
                                    will be created two transactions.",
                                    parameters: {
                                        (header "timestamp"),
                                        (header "sign")
                                    },
                                    body: AccountTransactionEvent,
                                    200: None,
                                }
                            }
                        }
                    }
                }
            },
            ("transactions" / "confirm"): {
                POST: {
                    tags: { transactions },
                    summary: "Create confirm transaction",
                    description: "Confirm transaction.",
                    parameters: { (header "api-key") },
                    body: requests::PostTonTransactionConfirmRequest,
                    200: responses::AccountTransactionResponse,
                    callbacks: {
                        transactionSent: {
                            ("callbackUrl"): {
                                POST: {
                                    description: "Event transaction sent. If address are not \
                                    deployed the event will be sent a twice since in this case \
                                    will be created two transactions.",
                                    parameters: {
                                        (header "timestamp"),
                                        (header "sign")
                                    },
                                    body: AccountTransactionEvent,
                                    200: None,
                                }
                            }
                        }
                    }
                }
            },
            ("transactions" / "mh" / String): {
                GET: {
                    tags: { transactions },
                    summary: "Get transaction",
                    description: "Get transaction by message hash.",
                    parameters: { (header "api-key") },
                    200: responses::AccountTransactionResponse,
                }
            },
            ("transactions" / "h" / String): {
                GET: {
                    tags: { transactions },
                    summary: "Get transaction",
                    description: "Get transaction by transaction hash.",
                    parameters: { (header "api-key") },
                    200: responses::AccountTransactionResponse,
                }
            },
            ("transactions" / "id" / String): {
                GET: {
                    tags: { transactions },
                    summary: "Get transaction",
                    description: "Get transaction by id.",
                    parameters: { (header "api-key") },
                    200: responses::AccountTransactionResponse,
                }
            },
            ("transactions"): {
                POST: {
                    tags: { transactions },
                    summary: "Search transactions",
                    description: "Search transactions.",
                    parameters: { (header "api-key") },
                    body: requests::PostTonTransactionsRequest,
                    200: responses::TonTransactionsResponse,
                }
            },
            ("events" ): {
                POST: {
                    tags: { events },
                    summary: "Get events",
                    description: "Get events.",
                    parameters: { (header "api-key") },
                    body: requests::PostTonTransactionEventsRequest,
                    200: responses::TonEventsResponse,
                }
            },
            ("events" / "id" / String): {
                GET: {
                    tags: { events },
                    summary: "Get event",
                    description: "Get event by id.",
                    parameters: { (header "api-key") },
                    200: responses::AccountTransactionEventResponse,
                }
            },
            ("events" / "mark" ): {
                POST: {
                    tags: { events },
                    summary: "Mark event",
                    description: "Mark event by id.",
                    parameters: { (header "api-key") },
                    body: requests::PostTonMarkEventsRequest,
                    200: responses::MarkEventsResponse,
                }
            },
            ("events" / "mark" / "all" ): {
                POST: {
                    tags: { events },
                    summary: "Mark events",
                    description: "Mark events by status optional.",
                    parameters: { (header "api-key") },
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
                    parameters: { (header "api-key") },
                    body: requests::PostTonTokenTransactionSendRequest,
                    200: responses::AccountTokenTransactionResponse,
                    callbacks: {
                        tokenTransactionSent: {
                            ("callbackUrl"): {
                                POST: {
                                    description: "Event token transaction sent. If address are not \
                                    deployed the event will be sent a twice since in this case \
                                    will be created two transactions.",
                                    parameters: {
                                        (header "timestamp"),
                                        (header "sign")
                                    },
                                    body: AccountTransactionEvent,
                                    200: None,
                                }
                            }
                        }
                    }
                }
            },
            ("tokens" / "transactions" / "mh" / String): {
                GET: {
                    tags: { transactions, tokens  },
                    summary: "Get tokens transaction",
                    description: "Get tokens transaction by message hash.",
                    parameters: { (header "api-key") },
                    200: responses::AccountTokenTransactionResponse,
                }
            },
            ("tokens" / "transactions" / "id" / String): {
                GET: {
                    tags: { transactions, tokens },
                    summary: "Get tokens transaction",
                    description: "Get tokens transaction by id.",
                    parameters: { (header "api-key") },
                    200: responses::AccountTokenTransactionResponse,
                }
            },
            ("tokens" / "events" ): {
                POST: {
                    tags: { events, tokens },
                    summary: "Get token events",
                    description: "Get token events.",
                    parameters: { (header "api-key") },
                    body: requests::PostTonTokenTransactionEventsRequest,
                    200: responses::TonTokenEventsResponse,
                }
            },
            ("tokens" / "events" / "mark" ): {
                POST: {
                    tags: { events, tokens },
                    summary: "Mark tokens event",
                    description: "Mark tokens event by id.",
                    parameters: { (header "api-key") },
                    body: requests::PostTonTokenMarkEventsRequest,
                    200: responses::MarkTokenEventsResponse,
                }
            },
            ("metrics"): {
                GET: {
                    tags: { metrics  },
                    summary: "Get metrics",
                    description: "Get metrics of api health.",
                    parameters: { (header "api-key") },
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
