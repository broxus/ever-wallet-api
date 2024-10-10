use std::str::FromStr;
use ton_types::UInt256;

pub fn get_network_id(root_hash: UInt256) -> i32 {
    let ever = UInt256::from_str("WP/KGheNr/cF3lQhblQzyb0ufYUAcNM004mXhHq56EU=").unwrap();
    let venom = UInt256::from_str("YLICYJzBkBm9C7RjszRzr7sUv/VsDkdaibI+baqLahA=").unwrap();
    let ton = UInt256::from_str("F6OpKZKqvqeFp6CQmFomXNMfMj2EnaUSOXN+Mh+wVWk=").unwrap();
    if root_hash == ever {
        42
    } else if root_hash == venom {
        1
    } else if root_hash == ton {
        -239
    } else {
        0
    }
}
