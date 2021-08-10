use std::env;

use dexpa::errors::*;

#[tokio::main(worker_threads = 8)]
async fn main() -> StdResult<()> {
    let args: Vec<String> = env::args().collect();

    match &*args[1] {
        "server" => ton_wallet_api_lib::start_server().await?,
        other => return Err(format!("Unknown arg - {}", other).into()),
    }

    Ok(())
}
