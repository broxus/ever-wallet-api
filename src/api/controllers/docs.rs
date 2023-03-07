#![allow(clippy::needless_update)]

use nekoton_utils::TrustMe;
use opg::*;

use crate::api::requests;
use crate::api::responses;

use crate::models::*;

pub fn swagger(prod_url: &str) -> String {
    let api = describe_api! {
        info: {
            title: "Everscale API",
            version: "4.0.0",
            description: r##"This API allows you to use Everscale Wallet API"##,
        },
        servers: {
            prod_url
        },
        tags: {
            address,
            events,
            tokens,
            misc,
            metrics,
        },
        paths: {
            ("address" / "check"): {
                POST: {
                    tags: { address },
                    summary: "Check address",
                    description: "Check correction of EVER address.",
                    parameters: {
                        (header "api-key"): {
                            description: "API Key",
                        },
                        (header "sign"): {
                            description: "Signature",
                        },
                        (header "timestamp"): {
                            description: "Timestamp in ms",
                        },
                        (header "x-real-ip"): {
                            required: false
                        },
                    },
                    body: requests::AddressCheckRequest,
                    200: responses::CheckedAddressResponse,
                }
            },
            ("address" / "create"): {
                POST: {
                    tags: { address },
                    summary: "Address creation",
                    description: "Create user address.",
                    parameters: {
                        (header "api-key"): {
                            description: "API Key",
                        },
                        (header "sign"): {
                            description: "Signature",
                        },
                        (header "timestamp"): {
                            description: "Timestamp in ms",
                        },
                        (header "x-real-ip"): {
                            required: false
                        },
                    },
                    body: requests::CreateAddressRequest,
                    200: responses::AddressResponse,
                }
            },
             ("address" / { address: String }): {
                GET: {
                    tags: { address },
                    summary: "Address balance",
                    description: "Get address balance.",
                    parameters: {
                        (header "api-key"): {
                            description: "API Key",
                        },
                        (header "sign"): {
                            description: "Signature",
                        },
                        (header "timestamp"): {
                            description: "Timestamp in ms",
                        },
                        (header "x-real-ip"): {
                            required: false
                        },
                    },
                    200: responses::AddressBalanceResponse,
                }
            },
             ("address" / { address: String } / "info"): {
                GET: {
                    tags: { address },
                    summary: "Address info",
                    description: "Get address info.",
                    parameters: {
                        (header "api-key"): {
                            description: "API Key",
                        },
                        (header "sign"): {
                            description: "Signature",
                        },
                        (header "timestamp"): {
                            description: "Timestamp in ms",
                        },
                        (header "x-real-ip"): {
                            required: false
                        },
                    },
                    200: responses::AddressInfoResponse,
                }
            },
            ("transactions"): {
                POST: {
                    tags: { transactions },
                    summary: "Search transactions",
                    description: "Search transactions.",
                    parameters: {
                        (header "api-key"): {
                            description: "API Key",
                        },
                        (header "sign"): {
                            description: "Signature",
                        },
                        (header "timestamp"): {
                            description: "Timestamp in ms",
                        },
                        (header "x-real-ip"): {
                            required: false
                        },
                    },
                    body: requests::TonTransactionsRequest,
                    200: responses::TonTransactionsResponse,
                }
            },
            ("transactions" / "create"): {
                POST: {
                    tags: { transactions },
                    summary: "Create transaction",
                    description: "Send transaction.",
                    parameters: {
                        (header "api-key"): {
                            description: "API Key",
                        },
                        (header "sign"): {
                            description: "Signature",
                        },
                        (header "timestamp"): {
                            description: "Timestamp in ms",
                        },
                        (header "x-real-ip"): {
                            required: false
                        },
                    },
                    body: requests::TonTransactionSendRequest,
                    200: responses::TransactionResponse,
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
                    parameters: {
                        (header "api-key"): {
                            description: "API Key",
                        },
                        (header "sign"): {
                            description: "Signature",
                        },
                        (header "timestamp"): {
                            description: "Timestamp in ms",
                        },
                        (header "x-real-ip"): {
                            required: false
                        },
                    },
                    body: requests::TonTransactionConfirmRequest,
                    200: responses::TransactionResponse,
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
            ("transactions" / "id" / { id: String }): {
                GET: {
                    tags: { transactions },
                    summary: "Get transaction",
                    description: "Get transaction by id.",
                    parameters: {
                        (header "api-key"): {
                            description: "API Key",
                        },
                        (header "sign"): {
                            description: "Signature",
                        },
                        (header "timestamp"): {
                            description: "Timestamp in ms",
                        },
                        (header "x-real-ip"): {
                            required: false
                        },
                    },
                    200: responses::TransactionResponse,
                }
            },
            ("transactions" / "h" / { transaction_hash: String }): {
                GET: {
                    tags: { transactions },
                    summary: "Get transaction",
                    description: "Get transaction by transaction hash.",
                    parameters: {
                        (header "api-key"): {
                            description: "API Key",
                        },
                        (header "sign"): {
                            description: "Signature",
                        },
                        (header "timestamp"): {
                            description: "Timestamp in ms",
                        },
                        (header "x-real-ip"): {
                            required: false
                        },
                    },
                    200: responses::TransactionResponse,
                }
            },
            ("transactions" / "mh" / { message_hash: String }): {
                GET: {
                    tags: { transactions },
                    summary: "Get transaction",
                    description: "Get transaction by message hash.",
                    parameters: {
                        (header "api-key"): {
                            description: "API Key",
                        },
                        (header "sign"): {
                            description: "Signature",
                        },
                        (header "timestamp"): {
                            description: "Timestamp in ms",
                        },
                        (header "x-real-ip"): {
                            required: false
                        },
                    },
                    200: responses::TransactionResponse,
                }
            },
            ("events" / "id" / { id: String }): {
                GET: {
                    tags: { events },
                    summary: "Get event",
                    description: "Get event by id.",
                    parameters: {
                        (header "api-key"): {
                            description: "API Key",
                        },
                        (header "sign"): {
                            description: "Signature",
                        },
                        (header "timestamp"): {
                            description: "Timestamp in ms",
                        },
                        (header "x-real-ip"): {
                            required: false
                        },
                    },
                    200: responses::TransactionEventResponse,
                }
            },
            ("events" ): {
                POST: {
                    tags: { events },
                    summary: "Get events",
                    description: "Get events.",
                    parameters: {
                        (header "api-key"): {
                            description: "API Key",
                        },
                        (header "sign"): {
                            description: "Signature",
                        },
                        (header "timestamp"): {
                            description: "Timestamp in ms",
                        },
                        (header "x-real-ip"): {
                            required: false
                        },
                    },
                    body: requests::TonTransactionEventsRequest,
                    200: responses::TonEventsResponse,
                }
            },
            ("events" / "mark" ): {
                POST: {
                    tags: { events },
                    summary: "Mark event",
                    description: "Mark event by id.",
                    parameters: {
                        (header "api-key"): {
                            description: "API Key",
                        },
                        (header "sign"): {
                            description: "Signature",
                        },
                        (header "timestamp"): {
                            description: "Timestamp in ms",
                        },
                        (header "x-real-ip"): {
                            required: false
                        },
                    },
                    body: requests::TonMarkEventsRequest,
                    200: responses::MarkEventsResponse,
                }
            },
            ("events" / "mark" / "all" ): {
                POST: {
                    tags: { events },
                    summary: "Mark events",
                    description: "Mark events by status optional.",
                    parameters: {
                        (header "api-key"): {
                            description: "API Key",
                        },
                        (header "sign"): {
                            description: "Signature",
                        },
                        (header "timestamp"): {
                            description: "Timestamp in ms",
                        },
                        (header "x-real-ip"): {
                            required: false
                        },
                    },
                    body: requests::MarkAllTransactionEventRequest,
                    200: responses::MarkEventsResponse,
                }
            },
            ("tokens" / "address" / { address: String }): {
                GET: {
                    tags: { address, tokens },
                    summary: "Address tokens balances",
                    description: "Get address tokens balances.",
                    parameters: {
                        (header "api-key"): {
                            description: "API Key",
                        },
                        (header "sign"): {
                            description: "Signature",
                        },
                        (header "timestamp"): {
                            description: "Timestamp in ms",
                        },
                        (header "x-real-ip"): {
                            required: false
                        },
                    },
                    200: responses::TokenBalanceResponse,
                }
            },
            ("tokens" / "events" ): {
                POST: {
                    tags: { events, tokens },
                    summary: "Get token events",
                    description: "Get token events.",
                    parameters: {
                        (header "api-key"): {
                            description: "API Key",
                        },
                        (header "sign"): {
                            description: "Signature",
                        },
                        (header "timestamp"): {
                            description: "Timestamp in ms",
                        },
                        (header "x-real-ip"): {
                            required: false
                        },
                    },
                    body: requests::TonTokenTransactionEventsRequest,
                    200: responses::TonTokenEventsResponse,
                }
            },
            ("tokens" / "events" / "mark" ): {
                POST: {
                    tags: { events, tokens },
                    summary: "Mark tokens event",
                    description: "Mark tokens event by id.",
                    parameters: {
                        (header "api-key"): {
                            description: "API Key",
                        },
                        (header "sign"): {
                            description: "Signature",
                        },
                        (header "timestamp"): {
                            description: "Timestamp in ms",
                        },
                        (header "x-real-ip"): {
                            required: false
                        },
                    },
                    body: requests::TonTokenMarkEventsRequest,
                    200: responses::MarkTokenEventsResponse,
                }
            },
            ("tokens" / "transactions" / "mh" / { message_hash: String }): {
                GET: {
                    tags: { transactions, tokens  },
                    summary: "Get tokens transaction",
                    description: "Get tokens transaction by message hash.",
                    parameters: {
                        (header "api-key"): {
                            description: "API Key",
                        },
                        (header "sign"): {
                            description: "Signature",
                        },
                        (header "timestamp"): {
                            description: "Timestamp in ms",
                        },
                        (header "x-real-ip"): {
                            required: false
                        },
                    },
                    200: responses::TokenTransactionResponse,
                }
            },
            ("tokens" / "transactions" / "id" / { id: String }): {
                GET: {
                    tags: { transactions, tokens },
                    summary: "Get tokens transaction",
                    description: "Get tokens transaction by id.",
                    parameters: {
                        (header "api-key"): {
                            description: "API Key",
                        },
                        (header "sign"): {
                            description: "Signature",
                        },
                        (header "timestamp"): {
                            description: "Timestamp in ms",
                        },
                        (header "x-real-ip"): {
                            required: false
                        },
                    },
                    200: responses::TokenTransactionResponse,
                }
            },
            ("tokens" / "transactions" / "create"): {
                POST: {
                    tags: { transactions, tokens },
                    summary: "Create token transaction",
                    description: "Send token transaction.",
                    parameters: {
                        (header "api-key"): {
                            description: "API Key",
                        },
                        (header "sign"): {
                            description: "Signature",
                        },
                        (header "timestamp"): {
                            description: "Timestamp in ms",
                        },
                        (header "x-real-ip"): {
                            required: false
                        },
                    },
                    body: requests::TonTokenTransactionSendRequest,
                    200: responses::TokenTransactionResponse,
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
            ("tokens" / "transactions" / "burn"): {
                POST: {
                    tags: { transactions, tokens },
                    summary: "Burn token transaction",
                    description: "Burn token transaction.",
                    parameters: {
                        (header "api-key"): {
                            description: "API Key",
                        },
                        (header "sign"): {
                            description: "Signature",
                        },
                        (header "timestamp"): {
                            description: "Timestamp in ms",
                        },
                        (header "x-real-ip"): {
                            required: false
                        },
                    },
                    body: requests::TonTokenTransactionBurnRequest,
                    200: responses::TokenTransactionResponse,
                    callbacks: {
                        tokenTransactionSent: {
                            ("callbackUrl"): {
                                POST: {
                                    description: "Event token transaction burn.",
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
            ("tokens" / "transactions" / "mint"): {
                POST: {
                    tags: { transactions, tokens },
                    summary: "Mint token transaction",
                    description: "Mint token transaction.",
                    parameters: {
                        (header "api-key"): {
                            description: "API Key",
                        },
                        (header "sign"): {
                            description: "Signature",
                        },
                        (header "timestamp"): {
                            description: "Timestamp in ms",
                        },
                        (header "x-real-ip"): {
                            required: false
                        },
                    },
                    body: requests::TonTokenTransactionMintRequest,
                    200: responses::TokenTransactionResponse,
                    callbacks: {
                        tokenTransactionSent: {
                            ("callbackUrl"): {
                                POST: {
                                    description: "Event token transaction mint.",
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
            ("read-contract"): {
                POST: {
                    tags: { misc  },
                    summary: "Read contract data",
                    description: "Execute function of generic contract",
                    parameters: {
                        (header "api-key"): {
                            description: "API Key",
                        },
                        (header "sign"): {
                            description: "Signature",
                        },
                        (header "timestamp"): {
                            description: "Timestamp in ms",
                        },
                        (header "x-real-ip"): {
                            required: false
                        },
                    },
                    body: requests::ExecuteContractRequest,
                    200: responses::ReadContractResponse,
                }
            },
            ("encode-into-cell"): {
                POST: {
                    tags: { misc  },
                    summary: "Encode tvm cell",
                    description: "Create cell from custom tokens and get base64 cell representation",
                    parameters: {
                        (header "api-key"): {
                            description: "API Key",
                        },
                        (header "sign"): {
                            description: "Signature",
                        },
                        (header "timestamp"): {
                            description: "Timestamp in ms",
                        },
                        (header "x-real-ip"): {
                            required: false
                        },
                    },
                    body: requests::EncodeParamRequest,
                    200: responses::EncodedCellResponse,
                }
            },
            ("prepare-message"): {
                POST: {
                    tags: { misc  },
                    summary: "Prepare message from params",
                    description: "Prepare unsigned message from specified tokens",
                    parameters: {
                        (header "api-key"): {
                            description: "API Key",
                        },
                        (header "sign"): {
                            description: "Signature",
                        },
                        (header "timestamp"): {
                            description: "Timestamp in ms",
                        },
                        (header "x-real-ip"): {
                            required: false
                        },
                    },
                    body: requests::PrepareMessageRequest,
                    200: responses::UnsignedMessageHashResponse,
                }
            },
            ("send-signed-message"): {
                POST: {
                    tags: { misc  },
                    summary: "Send signed message",
                    description: "Seng signed message",
                    parameters: {
                        (header "api-key"): {
                            description: "API Key",
                        },
                        (header "sign"): {
                            description: "Signature",
                        },
                        (header "timestamp"): {
                            description: "Timestamp in ms",
                        },
                        (header "x-real-ip"): {
                            required: false
                        },
                    },
                    body: requests::SignedMessageRequest,
                    200: responses::TransactionResponse,
                }
            },
            ("send-message"): {
                POST: {
                    tags: { misc  },
                    summary: "Prepare and send message from params",
                    description: "Prepare unsigned message from specified tokens, sign and send it",
                    parameters: {
                        (header "api-key"): {
                            description: "API Key",
                        },
                        (header "sign"): {
                            description: "Signature",
                        },
                        (header "timestamp"): {
                            description: "Timestamp in ms",
                        },
                        (header "x-real-ip"): {
                            required: false
                        },
                    },
                    body: requests::SendMessageRequest,
                    200: responses::SignedMessageHashResponse,
                }
            },
            ("metrics"): {
                GET: {
                    tags: { metrics  },
                    summary: "Get metrics",
                    description: "Get metrics of api health.",
                    parameters: {
                        (header "api-key"): {
                            description: "API Key",
                        },
                        (header "sign"): {
                            description: "Signature",
                        },
                        (header "timestamp"): {
                            description: "Timestamp in ms",
                        },
                        (header "x-real-ip"): {
                            required: false
                        },
                    },
                    200: responses::MetricsResponse,
                }
            },
        }
    };

    serde_yaml::to_string(&api).trust_me()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_docs() {
        println!("{}", swagger("prod_url"));
    }
}
