use adenosine_pds::*;
use anyhow::Result;

use log::{self, debug};
use structopt::StructOpt;


#[derive(StructOpt)]
#[structopt(
    rename_all = "kebab-case",
    about = "personal digital server (PDS) implementation for AT protocol (atproto.com)"
)]
struct Opt {
    // TODO: different path type for structopt?

    /// File path of sqlite database for storing IPLD blocks (aka, repository content)
    #[structopt(
        parse(from_os_str),
        global = true,
        long = "--block-db",
        env = "ATP_BLOCK_DB",
        default_value = "adenosine_pds_blockstore.sqlite"
    )]
    blockstore_db_path: std::path::PathBuf,

    /// File path of sqlite database for ATP service (user accounts, indices, etc)
    #[structopt(
        parse(from_os_str),
        global = true,
        long = "--atp-db",
        env = "ATP_ATP_DB",
        default_value = "adenosine_pds_atp.sqlite"
    )]
    atp_db_path: std::path::PathBuf,

    /// Log more messages. Pass multiple times for ever more verbosity
    #[structopt(global = true, long, short = "v", parse(from_occurrences))]
    verbose: i8,

    #[structopt(subcommand)]
    cmd: Command,
}

#[derive(StructOpt)]
enum Command {
    /// Start ATP server as a foreground process
    Serve,

    /// Import a CAR file (TODO)
    Import,

    /// Dump info from databases (TODO)
    Inspect,
}

fn main() -> Result<()> {
    let opt = Opt::from_args();

    let log_level = match opt.verbose {
        std::i8::MIN..=-1 => "none",
        0 => "warn",
        1 => "info",
        2 => "debug",
        3..=std::i8::MAX => "trace",
    };
    // hyper logging is very verbose, so crank that down even if everything else is more verbose
    let cli_filter = format!("{},hyper=error", log_level);
    // defer to env var config, fallback to CLI settings
    let log_filter = std::env::var("RUST_LOG").unwrap_or(cli_filter);
    pretty_env_logger::formatted_timed_builder()
        .parse_filters(&log_filter)
        .init();

    debug!("config parsed, starting up");

    match opt.cmd {
        Command::Serve {} => {
            // TODO: log some config stuff?
            run_server()
        },
        Command::Import {} => {
            unimplemented!()
        },
        Command::Inspect {} => {
            unimplemented!()
        },
    }
}

