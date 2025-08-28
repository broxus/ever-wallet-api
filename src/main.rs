use std::process::ExitCode;

use anyhow::Result;
use clap::Args;
use clap::{Parser, Subcommand};
use tycho_wallet_api::commands::*;

mod cmd {
    pub mod run;
}

#[global_allocator]
static ALLOC: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

#[allow(clippy::print_stderr)]
fn main() -> ExitCode {
    if std::env::var("RUST_BACKTRACE").is_err() {
        // Enable backtraces on panics by default.
        std::env::set_var("RUST_BACKTRACE", "1");
    }
    if std::env::var("RUST_LIB_BACKTRACE").is_err() {
        // Disable backtraces in libraries by default
        std::env::set_var("RUST_LIB_BACKTRACE", "0");
    }

    match App::parse().run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("Error: {err:?}");
            ExitCode::FAILURE
        }
    }
}

#[derive(Parser)]
#[clap(version = tycho_wallet_api::version_string())]
#[clap(subcommand_required = true)]
pub struct App {
    #[clap(subcommand)]
    cmd: SubCmd,
}

impl App {
    pub fn run(self) -> Result<()> {
        match self.cmd {
            SubCmd::Server(cmd) => cmd.run(),
            SubCmd::RootToken(cmd) => cmd.run(),
            SubCmd::ApiService(cmd) => cmd.run(),
            SubCmd::Salt(cmd) => cmd.run(),
        }
    }
}

#[derive(Subcommand)]
enum SubCmd {
    Server(cmd::run::Cmd),
    RootToken(CmdRootToken),
    ApiService(CmdApiService),
    Salt(CmdSalt),
}

#[derive(Args, Clone)]
struct CmdRootToken {
    /// root token name
    #[clap(short, long)]
    pub name: String,
    /// root token address
    #[clap(short, long)]
    pub address: String,
    /// root token version
    #[clap(short, long)]
    pub version: String,
}

impl CmdRootToken {
    fn run(self) -> Result<()> {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .worker_threads(2)
            .build()?
            .block_on(add_root_token(self.name, self.address, self.version))
    }
}

#[derive(Args, Clone)]
struct CmdApiService {
    /// service id
    #[clap(short = 'i')]
    id: Option<String>,
    /// service name
    #[clap(short = 'n')]
    name: String,
    /// service key
    #[clap(short = 'k')]
    key: String,
    /// service secret
    #[clap(short = 's')]
    secret: String,
}

impl CmdApiService {
    fn run(self) -> Result<()> {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .worker_threads(2)
            .build()?
            .block_on(create_api_service(
                self.id,
                self.name,
                self.key,
                self.secret,
            ))
    }
}

#[derive(Args, Clone)]
struct CmdSalt {}

impl CmdSalt {
    fn run(self) -> Result<()> {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .worker_threads(2)
            .build()?
            .block_on(generate_salt())
    }
}
