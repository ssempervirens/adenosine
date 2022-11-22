use adenosine_cli::identifiers::*;
use adenosine_cli::*;
use anyhow::anyhow;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::str::FromStr;

use colored_json::to_colored_json_auto;
use log::{self, debug};
use std::io::Write;
use structopt::StructOpt;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

#[derive(StructOpt)]
#[structopt(
    rename_all = "kebab-case",
    about = "command-line client for AT protocol (atproto.com)"
)]
struct Opt {
    /// HTTP(S) URL of Personal Data Server to connect to
    #[structopt(
        global = true,
        long = "--host",
        env = "ATP_HOST",
        default_value = "http://localhost:2583"
    )]
    atp_host: String,

    /// Authentication session token (JWT), for operations that need it
    #[structopt(
        global = true,
        long = "--auth-token",
        env = "ATP_AUTH_TOKEN",
        hide_env_values = true
    )]
    auth_token: Option<String>,

    /// Log more messages. Pass multiple times for ever more verbosity
    ///
    /// By default, it'll only report errors. Passing `-v` one time also prints
    /// warnings, `-vv` enables info logging, `-vvv` debug, and `-vvvv` trace.
    #[structopt(global = true, long, short = "v", parse(from_occurrences))]
    verbose: i8,

    #[structopt(long = "--shell-completions", hidden = true)]
    shell_completions: Option<structopt::clap::Shell>,

    #[structopt(subcommand)]
    cmd: Command,
}

#[derive(StructOpt)]
enum AccountCommand {
    /// Register a new account
    ///
    /// Does not (yet) support invite codes or email verification.
    ///
    /// This will return a JWT token that you should assign to the `ATP_AUTH_TOKEN` environment
    /// variable.
    Register {
        #[structopt(long, short)]
        email: String,

        #[structopt(long = "--username", short = "-u")]
        handle: String,

        #[structopt(long, short)]
        password: String,

        #[structopt(long, short)]
        recovery_key: Option<String>,

        #[structopt(long, short)]
        invite_code: Option<String>,
    },
    /// Delete the currently logged-in account (danger!)
    Delete,
    /// Create a new authenticated session
    ///
    /// This will return a JWT token that you should assign to the `ATP_AUTH_TOKEN` environment
    /// variable
    Login {
        #[structopt(long = "--username", short = "-u")]
        handle: String,

        #[structopt(long, short)]
        password: String,
    },
    /// Deletes the current login session
    Logout,
    /// Fetches account metadata for the current session
    Info,
    // TODO: CreateRevocationKey or CreateDid
}

#[derive(StructOpt)]
enum RepoCommand {
    /// Get the current 'root' commit for a DID
    ///
    Root {
        /// Repository DID, or uses the current session account
        did: Option<DidOrHost>,
    },
    /// Dump raw binary repository as CAR format to stdout
    Export {
        /// Repository DID, or uses the current session account
        did: Option<DidOrHost>,
        /// CID of a prior commit; only newer updates are included
        #[structopt(long)]
        from: Option<String>,
    },
    /// Read raw binary repository as CAR format from stdin, and import to PDS
    Import {
        // TODO: could accept either path or stdin?
        /// Repository DID, or uses the current session account
        #[structopt(long)]
        did: Option<DidOrHost>,
    },
}

#[derive(StructOpt)]
enum BskyCommand {
    /// Fetch the account feed for a specific user (or self, by default)
    Feed { name: Option<DidOrHost> },
    /// Fetch timeline for currently logged-in account
    Timeline,
    /// Fetch notification feed
    Notifications,
    /// Create a new 'post' record
    Post { text: String },
    /// Create a 'repost' record for the target by AT URI
    Repost { uri: AtUri },
    /// Create a 'like' record for the target by AT URI
    Like { uri: AtUri },
    /// Create a 'follow' record for the target by AT URI
    Follow { uri: DidOrHost },
    // TODO: Unlike { uri: String, },
    // TODO: Unfollow { uri: String, },
    /* TODO:
    Follows {
        name: String,
    },
    Followers {
        name: String,
    },
    */
    /// Display a profile record (or self if not provided)
    Profile { name: Option<DidOrHost> },
    /// Query by partial handle
    SearchUsers { query: String },
}

#[derive(StructOpt)]
enum Command {
    /// Summarize connection and authentication with API
    Status,

    /// List all collections for a user, or all records for a collection
    Ls { uri: AtUri },
    /// Fetch and display a generic record by full AT URI
    Get {
        uri: AtUri,

        /// Specific version of record to fetch
        #[structopt(long)]
        cid: Option<String>,
    },
    /// Generic record creation
    Create {
        collection: Nsid,

        /// Set of object fields (keys) and values to construct the object from
        fields: Vec<ArgField>,
    },
    /// Generic mutation of an existing record
    Update {
        uri: AtUri,

        /// Set of object fields (keys) and values to update in the record
        fields: Vec<ArgField>,
    },
    /// Generic record deletion
    Delete { uri: AtUri },

    /// Print user/repository-level description (including DID document)
    Describe { name: Option<DidOrHost> },

    /// Have PDS resolve the DID for a handle
    Resolve { name: DidOrHost },

    /// Generic HTTP XRPC helper, printing any result
    Xrpc {
        /// 'get' or 'post'
        method: XrpcMethod,
        /// Name of method to call
        nsid: Nsid,
        /// Set of query parameters and body fields for the request
        fields: Vec<ArgField>,
    },

    /// Manage user account and sessions
    Account {
        #[structopt(subcommand)]
        cmd: AccountCommand,
    },

    /// Direct access to binary repository content
    Repo {
        #[structopt(subcommand)]
        cmd: RepoCommand,
    },

    /// Helper commands for bsky.app Lexicon
    Bsky {
        #[structopt(subcommand)]
        cmd: BskyCommand,
    },
}

fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    let opt = Opt::from_args();

    let log_level = match opt.verbose {
        std::i8::MIN..=-1 => "none",
        0 => "error",
        1 => "warn",
        2 => "info",
        3 => "debug",
        4..=std::i8::MAX => "trace",
    };
    // hyper logging is very verbose, so crank that down even if everything else is more verbose
    let log_filter = format!("{},hyper=error", log_level);
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(log_filter))
        .format_timestamp(None)
        .init();

    debug!("Args parsed, starting up");

    #[cfg(windows)]
    colored_json::enable_ansi_support();

    if let Some(shell) = opt.shell_completions {
        Opt::clap().gen_completions_to("adenosine", shell, &mut std::io::stdout());
        std::process::exit(0);
    }

    if let Err(err) = run(opt) {
        // Be graceful about some errors
        if let Some(io_err) = err.root_cause().downcast_ref::<std::io::Error>() {
            if let std::io::ErrorKind::BrokenPipe = io_err.kind() {
                // presumably due to something like writing to stdout and piped to `head -n10` and
                // stdout was closed
                debug!("got BrokenPipe error, assuming stdout closed as expected and exiting with success");
                std::process::exit(0);
            }
        }
        let mut color_stderr = StandardStream::stderr(if atty::is(atty::Stream::Stderr) {
            ColorChoice::Auto
        } else {
            ColorChoice::Never
        });
        color_stderr.set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true))?;
        eprintln!("Error: {:?}", err);
        color_stderr.set_color(&ColorSpec::new())?;
        std::process::exit(1);
    }
    Ok(())
}

fn print_result_json(result: Option<Value>) -> Result<()> {
    if let Some(val) = result {
        writeln!(&mut std::io::stdout(), "{}", to_colored_json_auto(&val)?)?
    };
    Ok(())
}

fn run(opt: Opt) -> Result<()> {
    let xrpc_client = XrpcClient::new(opt.atp_host.clone(), opt.auth_token.clone())?;
    let mut params: HashMap<String, String> = HashMap::new();
    let jwt_did: Option<String> = if let Some(ref token) = opt.auth_token {
        Some(parse_did_from_jwt(token)?)
    } else {
        None
    };

    let result = match opt.cmd {
        Command::Status => {
            println!("Configuration");
            println!("  ATP_HOST: {}", opt.atp_host);
            if opt.auth_token.is_some() {
                println!("  ATP_AUTH_TOKEN: <configured>");
            } else {
                println!("  ATP_AUTH_TOKEN:");
            }
            // TODO: parse JWT?
            // TODO: connection, auth check
            // TODO: account username, did, etc
            None
        }
        Command::Describe { name } => {
            let name = name
                .map(|v| v.to_string())
                .or(jwt_did)
                .ok_or(anyhow!("expected a name, or self via auth token"))?;
            params.insert("user".to_string(), name);
            xrpc_client.get(&Nsid::from_str("com.atproto.repo.describe")?, Some(params))?
        }
        Command::Resolve { name } => {
            params.insert("name".to_string(), name.to_string());
            xrpc_client.get(&Nsid::from_str("com.atproto.handle.resolve")?, Some(params))?
        }
        Command::Get { uri, cid } => {
            params.insert("user".to_string(), uri.repository.to_string());
            params.insert(
                "collection".to_string(),
                uri.collection.ok_or(anyhow!("collection required"))?,
            );
            params.insert(
                "rkey".to_string(),
                uri.record.ok_or(anyhow!("record key required"))?,
            );
            if let Some(c) = cid {
                params.insert("cid".to_string(), c);
            }
            xrpc_client.get(&Nsid::from_str("com.atproto.repo.getRecord")?, Some(params))?
        }
        Command::Ls { uri } => {
            // TODO: option to print fully-qualified path?
            params.insert("user".to_string(), uri.repository.to_string());
            if uri.collection.is_none() {
                // if a repository, but no collection, list the collections
                let describe = xrpc_client
                    .get(&Nsid::from_str("com.atproto.repo.describe")?, Some(params))?
                    .ok_or(anyhow!("expected a repo.describe response"))?;
                for c in describe["collections"]
                    .as_array()
                    .ok_or(anyhow!("expected collection list"))?
                {
                    println!(
                        "at://{}/{}",
                        uri.repository,
                        c.as_str()
                            .ok_or(anyhow!("expected collection as a JSON string"))?
                    );
                }
            } else if uri.collection.is_some() && uri.record.is_none() {
                // if a collection, but no record, list the records (with extracted timestamps)
                params.insert("collection".to_string(), uri.collection.unwrap());
                let records = xrpc_client
                    .get(
                        &Nsid::from_str("com.atproto.repo.listRecords")?,
                        Some(params),
                    )?
                    .ok_or(anyhow!("expected a repoListRecords response"))?;
                for r in records["records"].as_array().unwrap_or(&vec![]).iter() {
                    println!("{}", r["uri"].as_str().unwrap());
                }
            } else {
                return Err(anyhow!("got too much of a URI to 'ls'"));
            }
            None
        }
        Command::Create { collection, fields } => {
            params.insert(
                "did".to_string(),
                jwt_did.ok_or(anyhow!("need auth token"))?,
            );
            params.insert("collection".to_string(), collection.to_string());
            update_params_from_fields(&fields, &mut params);
            let val = value_from_fields(fields);
            xrpc_client.post(
                &Nsid::from_str("com.atproto.repo.createRecord")?,
                Some(params),
                Some(val),
            )?
        }
        Command::Update { uri, fields } => {
            params.insert("did".to_string(), uri.repository.to_string());
            params.insert("user".to_string(), uri.repository.to_string());
            params.insert(
                "collection".to_string(),
                uri.collection.ok_or(anyhow!("collection required"))?,
            );
            params.insert(
                "rkey".to_string(),
                uri.record.ok_or(anyhow!("record key required"))?,
            );
            // fetch existing, extend map with fields, put the updated value
            let mut record = xrpc_client
                .get(
                    &Nsid::from_str("com.atproto.repo.getRecord")?,
                    Some(params.clone()),
                )?
                .unwrap_or(json!({}));
            update_params_from_fields(&fields, &mut params);
            update_value_from_fields(fields, &mut record);
            xrpc_client.post(
                &Nsid::from_str("com.atproto.repo.putRecord")?,
                Some(params),
                Some(record),
            )?
        }
        Command::Delete { uri } => {
            params.insert("did".to_string(), uri.repository.to_string());
            params.insert(
                "collection".to_string(),
                uri.collection.ok_or(anyhow!("collection required"))?,
            );
            params.insert(
                "rkey".to_string(),
                uri.record.ok_or(anyhow!("record key required"))?,
            );
            xrpc_client.post(
                &Nsid::from_str("com.atproto.repo.deleteRecord")?,
                Some(params),
                None,
            )?
        }
        Command::Xrpc {
            method,
            nsid,
            fields,
        } => {
            update_params_from_fields(&fields, &mut params);
            let body = value_from_fields(fields);
            match method {
                XrpcMethod::Get => xrpc_client.get(&nsid, Some(params))?,
                XrpcMethod::Post => xrpc_client.post(&nsid, Some(params), Some(body))?,
            }
        }
        Command::Account {
            cmd:
                AccountCommand::Register {
                    email,
                    handle,
                    password,
                    recovery_key,
                    invite_code,
                },
        } => {
            let mut body = json!({
                "email": email,
                "handle": handle,
                "password": password,
            });
            if let Some(key) = recovery_key {
                body["recoveryKey"] = json!(key);
            }
            if let Some(code) = invite_code {
                body["inviteCode"] = json!(code);
            }
            xrpc_client.post(
                &Nsid::from_str("com.atproto.account.create")?,
                None,
                Some(body),
            )?
        }
        Command::Account {
            cmd: AccountCommand::Login { handle, password },
        } => xrpc_client.post(
            &Nsid::from_str("com.atproto.session.create")?,
            None,
            Some(json!({
                "handle": handle,
                "password": password,
            })),
        )?,
        Command::Account {
            cmd: AccountCommand::Logout,
        } => xrpc_client.post(&Nsid::from_str("com.atproto.session.delete")?, None, None)?,
        Command::Account {
            cmd: AccountCommand::Delete,
        } => xrpc_client.post(&Nsid::from_str("com.atproto.account.delete")?, None, None)?,
        Command::Account {
            cmd: AccountCommand::Info,
        } => xrpc_client.get(&Nsid::from_str("com.atproto.account.get")?, None)?,
        Command::Repo {
            cmd: RepoCommand::Root { did },
        } => {
            let did = match did {
                Some(DidOrHost::Host(_)) => return Err(anyhow!("expected a DID, not a hostname")),
                Some(v) => v.to_string(),
                None => jwt_did.ok_or(anyhow!("expected a DID"))?,
            };
            params.insert("did".to_string(), did);
            xrpc_client.get(&Nsid::from_str("com.atproto.sync.getRoot")?, Some(params))?
        }
        Command::Repo {
            cmd: RepoCommand::Export { did, from },
        } => {
            let did = match did {
                Some(DidOrHost::Host(_)) => return Err(anyhow!("expected a DID, not a hostname")),
                Some(v) => v.to_string(),
                None => jwt_did.ok_or(anyhow!("expected a DID"))?,
            };
            params.insert("did".to_string(), did);
            if let Some(from) = from {
                params.insert("from".to_string(), from);
            };
            xrpc_client.get_to_writer(
                &Nsid::from_str("com.atproto.sync.getRepo")?,
                Some(params),
                &mut std::io::stdout(),
            )?;
            None
        }
        Command::Repo {
            cmd: RepoCommand::Import { did },
        } => {
            let did = match did {
                Some(DidOrHost::Host(_)) => return Err(anyhow!("expected a DID, not a hostname")),
                Some(v) => v.to_string(),
                None => jwt_did.ok_or(anyhow!("expected a DID"))?,
            };
            params.insert("did".to_string(), did);
            xrpc_client.post_cbor_from_reader(
                &Nsid::from_str("com.atproto.sync.updateRepo")?,
                Some(params),
                &mut std::io::stdin(),
            )?
        }
        Command::Bsky {
            cmd: BskyCommand::Feed { name },
        } => {
            // TODO: not expect here
            let name = name
                .map(|v| v.to_string())
                .unwrap_or(jwt_did.expect("feed name or logged in"));
            params.insert("author".to_string(), name);
            xrpc_client.get(
                &Nsid::from_str("app.bsky.feed.getAuthorFeed")?,
                Some(params),
            )?
        }
        Command::Bsky {
            cmd: BskyCommand::Timeline,
        } => xrpc_client.get(&Nsid::from_str("app.bsky.feed.getTimeline")?, None)?,
        Command::Bsky {
            cmd: BskyCommand::Notifications,
        } => xrpc_client.get(&Nsid::from_str("app.bsky.notifications.get")?, None)?,
        Command::Bsky {
            cmd: BskyCommand::Post { text },
        } => {
            params.insert(
                "did".to_string(),
                jwt_did.ok_or(anyhow!("need auth token"))?,
            );
            params.insert("collection".to_string(), "app.bsky.feed.post".to_string());
            xrpc_client.post(
                &Nsid::from_str("com.atproto.repo.createRecord")?,
                Some(params),
                Some(json!({
                    "text": text,
                })),
            )?
        }
        Command::Bsky {
            cmd: BskyCommand::Repost { uri },
        } => {
            params.insert(
                "did".to_string(),
                jwt_did.ok_or(anyhow!("need auth token"))?,
            );
            params.insert("collection".to_string(), "app.bsky.feed.repost".to_string());
            xrpc_client.post(
                &Nsid::from_str("com.atproto.repo.createRecord")?,
                Some(params),
                Some(json!({
                    "subject": uri.to_string(),
                    "createdAt": created_at_now(),
                })),
            )?
        }
        Command::Bsky {
            cmd: BskyCommand::Like { uri },
        } => {
            params.insert(
                "did".to_string(),
                jwt_did.ok_or(anyhow!("need auth token"))?,
            );
            params.insert("collection".to_string(), "app.bsky.feed.like".to_string());
            xrpc_client.post(
                &Nsid::from_str("com.atproto.repo.createRecord")?,
                Some(params),
                Some(json!({
                    "subject": { "uri": uri.to_string(), "cid": "TODO" },
                    "createdAt": created_at_now(),
                })),
            )?
        }
        Command::Bsky {
            cmd: BskyCommand::Follow { uri },
        } => {
            params.insert(
                "did".to_string(),
                jwt_did.ok_or(anyhow!("need auth token"))?,
            );
            params.insert(
                "collection".to_string(),
                "app.bsky.graph.follow".to_string(),
            );
            xrpc_client.post(
                &Nsid::from_str("com.atproto.repo.createRecord")?,
                Some(params),
                Some(json!({
                    "subject": { "did": uri.to_string() },
                    "createdAt": created_at_now(),
                })),
            )?
        }
        Command::Bsky {
            cmd: BskyCommand::Profile { name },
        } => {
            let name = name
                .map(|v| v.to_string())
                .or(jwt_did)
                .ok_or(anyhow!("expected a name, or self via auth token"))?;
            params.insert("user".to_string(), name);
            xrpc_client.get(&Nsid::from_str("app.bsky.actor.getProfile")?, Some(params))?
        }
        Command::Bsky {
            cmd: BskyCommand::SearchUsers { query },
        } => {
            params.insert("term".to_string(), query);
            xrpc_client.get(&Nsid::from_str("app.bsky.actor.search")?, Some(params))?
        }
    };
    print_result_json(result)?;
    Ok(())
}
