use std::str::FromStr;
use ton_types::UInt256; // Replace with the actual crate providing `UInt256`

pub fn get_network_id(root_hash: UInt256) -> i32 {
    let EVER_ID = UInt256::from_str("WP/KGheNr/cF3lQhblQzyb0ufYUAcNM004mXhHq56EU=").unwrap();
    let VENOM_ID = UInt256::from_str("YLICYJzBkBm9C7RjszRzr7sUv/VsDkdaibI+baqLahA=").unwrap();
    let TON_ID = UInt256::from_str("F6OpKZKqvqeFp6CQmFomXNMfMj2EnaUSOXN+Mh+wVWk=").unwrap();
    if root_hash == EVER_ID {
        42
    } else if root_hash == VENOM_ID {
        1
    } else if root_hash == TON_ID {
        -239
    } else {
        0
    }
}
