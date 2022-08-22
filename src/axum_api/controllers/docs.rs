#![allow(clippy::needless_update)]

use nekoton_utils::TrustMe;
use opg::*;

use crate::axum_api::requests;
use crate::axum_api::responses;

pub fn swagger(prod_url: &str) -> String {
    let api = describe_api! {
        info: {
            title: "Everscale API",
            version: "4.0.0",
            description: r##"This API allows you to use Everscale API"##,
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
