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
        },
        paths: {
            ("address" / "check"): {
                POST: {
                    tags: { addresses },
                    summary: "Check address",
                    description: "Check correction of EVER address.",
                    parameters: { (header "api-key") },
                    body: requests::AddressCheckRequest,
                    200: responses::CheckedAddressResponse,
                }
            },
            ("address" / "create"): {
                POST: {
                    tags: { addresses },
                    summary: "Address creation",
                    description: "Create user address.",
                    parameters: { (header "api-key") },
                    body: requests::CreateAddressRequest,
                    200: responses::AddressResponse,
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
