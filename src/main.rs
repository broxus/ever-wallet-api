use std::env;

/*#[global_allocator]
static GLOBAL: ton_indexer::alloc::Allocator = ton_indexer::alloc::allocator();*/

#[tokio::main(worker_threads = 8)]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let args: Vec<String> = env::args().collect();

    match &*args[1] {
        "server" => ton_wallet_api_lib::start_server().await?,
        other => return Err(format!("Unknown arg - {}", other).into()),
    }

    Ok(())
}
