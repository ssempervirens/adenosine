use adenosine_pds::models::AccountRequest;
use adenosine_pds::*;
use anyhow::Result;
use serde_json::json;

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

    #[structopt(long = "--shell-completions", hidden = true)]
    shell_completions: Option<structopt::clap::Shell>,

    #[structopt(subcommand)]
    cmd: Command,
}

#[derive(StructOpt)]
enum Command {
    /// Start ATP server as a foreground process
    Serve {
        /// Secret key, encoded in hex. Use 'generate-secret' to create a new one
        #[structopt(
            long = "--pds-secret-key",
            env = "ATP_PDS_SECRET_KEY",
            hide_env_values = true
        )]
        pds_secret_key: String,

        /// Localhost port to listen on
        #[structopt(long, default_value = "3030", env = "ATP_PDS_PORT")]
        port: u16,

        /// A "public URL" for the PDS gets embedded in DID documents. If one is not provided, a
        /// localhost value will be used, which will not actually work for inter-PDS communication.
        #[structopt(long = "--public-url", env = "ATP_PDS_PUBLIC_URL")]
        public_url: Option<String>,

        /// If provided, allow registration for the given base domain name.
        #[structopt(long = "--registration-domain", env = "ATP_PDS_REGISTRATION_DOMAIN")]
        registration_domain: Option<String>,

        /// Optionally, require an invite code to sign up. This is just a single secret value.
        #[structopt(long = "--invite-code", env = "ATP_PDS_INVITE_CODE")]
        invite_code: Option<String>,

        /// Optionally, override domain name check and force the homepage to display the account
        /// page for this handle
        #[structopt(long = "--homepage-handle", env = "ATP_PDS_HOMEPAGE_HANDLE")]
        homepage_handle: Option<String>,
    },

    /// Helper to import an IPLD CARv1 file in to sqlite data store
    Import {
        /// CARv1 file path to import from
        car_path: std::path::PathBuf,

        /// name of pointer to root of CAR DAG tree. Usually a DID
        #[structopt(long, default_value = "last-import")]
        alias: String,
    },

    /// Helper to print MST keys/docs from a sqlite repo
    Inspect,

    /// Generate a PDS secret key and print to stdout (as hex)
    GenerateSecret,

    /// Create a new account with a did:plc. Bypasses most checks that the API would require for
    /// account registration.
    Register {
        /// Secret key, encoded in hex. Use 'generate-secret' to create a new one
        #[structopt(
            long = "--pds-secret-key",
            env = "ATP_PDS_SECRET_KEY",
            hide_env_values = true
        )]
        pds_secret_key: String,

        #[structopt(long = "--public-url", env = "ATP_PDS_PUBLIC_URL")]
        public_url: Option<String>,

        #[structopt(long, short)]
        handle: String,

        #[structopt(long, short)]
        password: String,

        #[structopt(long, short)]
        email: String,

        #[structopt(long, short)]
        recovery_key: Option<String>,

        /// Should we generate a did:plc, instead of using the handle as a did:web?
        #[structopt(long, short)]
        did_plc: bool,
    },
}

fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    let opt = Opt::from_args();

    if let Some(shell) = opt.shell_completions {
        Opt::clap().gen_completions_to("adenosine", shell, &mut std::io::stdout());
        std::process::exit(0);
    }

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
        Command::Serve {
            port,
            pds_secret_key,
            registration_domain,
            public_url,
            invite_code,
            homepage_handle,
        } => {
            let keypair = KeyPair::from_hex(&pds_secret_key)?;
            // clean up config a bit
            let registration_domain = match registration_domain {
                None => None,
                Some(v) if v.is_empty() => None,
                Some(v) => Some(v),
            };
            let public_url = match public_url {
                None => format!("http://localhost:{}", port),
                Some(v) if v.is_empty() => format!("http://localhost:{}", port),
                Some(v) => v,
            };
            let config = AtpServiceConfig {
                listen_host_port: format!("localhost:{}", port),
                public_url,
                registration_domain,
                invite_code,
                homepage_handle,
            };
            log::info!("PDS config: {:?}", config);
            let srv = AtpService::new(&opt.blockstore_db_path, &opt.atp_db_path, keypair, config)?;
            srv.run_server()
        }
        // TODO: handle alias
        Command::Import { car_path, alias } => {
            let mut repo = RepoStore::open(&opt.blockstore_db_path)?;
            repo.import_car_path(&car_path, Some(alias))?;
            Ok(())
        }
        Command::Inspect {} => mst::dump_mst_keys(&opt.blockstore_db_path),
        Command::GenerateSecret {} => {
            let keypair = KeyPair::new_random();
            println!("{}", keypair.to_hex());
            Ok(())
        }
        Command::Register {
            handle,
            password,
            email,
            recovery_key,
            pds_secret_key,
            public_url,
            did_plc,
        } => {
            let req = AccountRequest {
                email: email,
                handle: handle.clone(),
                password: password,
                inviteCode: None,
                recoveryKey: recovery_key,
            };
            let mut config = AtpServiceConfig::default();
            config.public_url = public_url.unwrap_or(format!("https://{}", handle));
            let keypair = KeyPair::from_hex(&pds_secret_key)?;
            let mut srv =
                AtpService::new(&opt.blockstore_db_path, &opt.atp_db_path, keypair, config)?;
            let sess = create_account(&mut srv, &req, did_plc)?;
            println!("{}", json!(sess));
            Ok(())
        }
    }
}
