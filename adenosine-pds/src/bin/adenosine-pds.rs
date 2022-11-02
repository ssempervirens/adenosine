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
    Serve {
        #[structopt(long, default_value = "3030")]
        port: u16,
    },

    /// Helper to import an IPLD CARv1 file in to sqlite data store
    Import {
        /// CARv1 file path to import from
        car_path: std::path::PathBuf,

        /// name of pointer to root of CAR DAG tree. Usually a DID
        alias: String,
    },

    /// Helper to print MST keys/docs from a sqlite repo
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
        Command::Serve { port } => {
            // TODO: log some config stuff?
            run_server(port, &opt.blockstore_db_path, &opt.atp_db_path)
        }
        // TODO: handle alias
        Command::Import { car_path, alias } => {
            load_car_to_sqlite(&opt.blockstore_db_path, &car_path)
        }
        Command::Inspect {} => mst::dump_mst_keys(&opt.blockstore_db_path),
    }
}
