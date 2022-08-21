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
            addresses,
            tokens,
            metrics,
        },
        paths: {
            ("address" / "check"): {
                POST: {
                    tags: { addresses },
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
                    tags: { addresses },
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
                    tags: { addresses },
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
                    tags: { addresses },
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
            ("tokens" / "address" / { address: String }): {
                GET: {
                    tags: { addresses, tokens },
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
