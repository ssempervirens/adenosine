use adenosine_cli::*;
use anyhow::anyhow;
use serde_json::{json, Value};
use std::collections::HashMap;

use colored_json::to_colored_json_auto;
use log::{self, debug};
use std::io::Write;
use structopt::StructOpt;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case", about = "CLI interface for AT Protocol")]
struct Opt {
    #[structopt(
        global = true,
        long = "--host",
        env = "ATP_HOST",
        default_value = "https://localhost:8080"
    )]
    atp_host: String,

    // API auth tokens can be generated from the account page in the fatcat.wiki web interface
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
    Register {
        #[structopt(long, short)]
        email: String,

        #[structopt(long, short)]
        username: String,

        #[structopt(long, short)]
        password: String,
    },
    Delete,
    Login {
        #[structopt(long, short)]
        username: String,

        #[structopt(long, short)]
        password: String,
    },
    Logout,
    Info,
    // TODO: CreateRevocationKey or CreateDid
}

#[derive(StructOpt)]
enum RepoCommand {
    Root {
        did: Option<DidOrHost>,
    },
    Export {
        did: Option<DidOrHost>,
        #[structopt(long)]
        from: Option<String>,
    },
    Import {
        // TODO: could accept either path or stdin?
        #[structopt(long)]
        did: Option<DidOrHost>,
    },
}

#[derive(StructOpt)]
enum BskyCommand {
    Feed { name: Option<DidOrHost> },
    Notifications,
    Post { text: String },
    Repost { uri: AtUri },
    Like { uri: AtUri },
    // TODO: Repost { uri: String, },
    Follow { uri: DidOrHost },
    // TODO: Unfollow { uri: String, },
    /* TODO:
    Follows {
        name: String,
    },
    Followers {
        name: String,
    },
    */
    Profile { name: DidOrHost },
    SearchUsers { query: String },
}

#[derive(StructOpt)]
enum Command {
    Get {
        uri: AtUri,

        #[structopt(long)]
        cid: Option<String>,
    },

    Ls {
        uri: AtUri,
    },

    Create {
        collection: String,
        fields: Vec<ArgField>,
    },
    Update {
        uri: AtUri,
        fields: Vec<ArgField>,
    },
    Delete {
        uri: AtUri,
    },

    Describe {
        name: Option<DidOrHost>,
    },

    Resolve {
        name: DidOrHost,
    },

    Xrpc {
        method: XrpcMethod,
        nsid: String,
        fields: Vec<ArgField>,
    },

    /// Sub-commands for managing account
    Account {
        #[structopt(subcommand)]
        cmd: AccountCommand,
    },

    Repo {
        #[structopt(subcommand)]
        cmd: RepoCommand,
    },

    Bsky {
        #[structopt(subcommand)]
        cmd: BskyCommand,
    },

    /// Summarize connection and authentication with API
    Status,
}

fn main() -> Result<()> {
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
            // XXX
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
                .ok_or(anyhow!("expected a name or auth token"))?;
            params.insert("user".to_string(), name.to_string());
            xrpc_client.get("com.atproto.repoDescribe", Some(params))?
        }
        Command::Resolve { name } => {
            let mut params: HashMap<String, String> = HashMap::new();
            params.insert("name".to_string(), name.to_string());
            xrpc_client.get("com.atproto.resolveName", Some(params))?
        }
        Command::Get { uri, cid } => {
            params.insert("did".to_string(), uri.repository.to_string());
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
            xrpc_client.get("com.atproto.repoGetRecord", Some(params))?
        }
        Command::Ls { uri } => {
            // TODO: option to print fully-qualified path?
            if !uri.collection.is_some() {
                // if a repository, but no collection, list the collections
                params.insert("user".to_string(), uri.repository.to_string());
                let describe = xrpc_client
                    .get("com.atproto.repoDescribe", Some(params))?
                    .ok_or(anyhow!("expected a repoDescribe response"))?;
                for c in describe["collections"]
                    .as_array()
                    .ok_or(anyhow!("expected collection list"))?
                {
                    println!(
                        "{}",
                        c.as_str()
                            .ok_or(anyhow!("expected collection as a JSON string"))?
                    );
                }
            } else if uri.collection.is_some() && !uri.record.is_some() {
                // if a collection, but no record, list the records (with extracted timestamps)
            } else {
                return Err(anyhow!("got too much of a URI to 'ls'"));
            }
            None
        }
        Command::Create { collection, fields } => {
            params.insert("collection".to_string(), collection);
            update_params_from_fields(&fields, &mut params);
            let val = value_from_fields(fields);
            xrpc_client.post("com.atproto.repoCreateRecord", Some(params), val)?
        }
        Command::Update { uri, fields } => {
            params.insert("did".to_string(), uri.repository.to_string());
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
                .get("com.atproto.repoGetRecord", Some(params.clone()))?
                .unwrap_or(json!({}));
            update_params_from_fields(&fields, &mut params);
            update_value_from_fields(fields, &mut record);
            xrpc_client.post("com.atproto.repoPutRecord", Some(params), record)?
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
            xrpc_client.post("com.atproto.repoDeleteRecord", Some(params), json!({}))?
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
                XrpcMethod::Post => xrpc_client.post(&nsid, Some(params), body)?,
            }
        }
        Command::Account {
            cmd:
                AccountCommand::Register {
                    email,
                    username,
                    password,
                },
        } => xrpc_client.post(
            "com.atproto.createAccount",
            None,
            json!({
                "email": email,
                "username": username,
                "password": password,
            }),
        )?,
        Command::Account {
            cmd: AccountCommand::Login { username, password },
        } => xrpc_client.post(
            "com.atproto.createSession",
            None,
            json!({
                "username": username,
                "password": password,
            }),
        )?,
        Command::Account {
            cmd: AccountCommand::Logout,
        } => xrpc_client.post("com.atproto.deleteSession", None, json!({}))?,
        Command::Account {
            cmd: AccountCommand::Delete,
        } => xrpc_client.post("com.atproto.deleteAccount", None, json!({}))?,
        Command::Account {
            cmd: AccountCommand::Info,
        } => xrpc_client.get("com.atproto.getAccount", None)?,
        Command::Repo {
            cmd: RepoCommand::Root { did },
        } => {
            let did = match did {
                Some(DidOrHost::Host(_)) => return Err(anyhow!("expected a DID, not a hostname")),
                Some(v) => v.to_string(),
                None => jwt_did.ok_or(anyhow!("expected a DID"))?,
            };
            params.insert("did".to_string(), did);
            xrpc_client.get("com.atproto.syncGetRoot", Some(params))?
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
                "com.atproto.syncGetRepo",
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
                "com.atproto.syncUpdateRepo",
                Some(params),
                &mut std::io::stdin(),
            )?
        }
        Command::Bsky {
            cmd: BskyCommand::Feed { name },
        } => {
            if let Some(name) = name {
                params.insert("author".to_string(), name.to_string());
                xrpc_client.get("app.bsky.getAuthorFeed", Some(params))?
            } else {
                xrpc_client.get("app.bsky.getHomeFeed", None)?
            }
        }
        Command::Bsky {
            cmd: BskyCommand::Notifications,
        } => xrpc_client.get("app.bsky.getNotifications", None)?,
        Command::Bsky {
            cmd: BskyCommand::Post { text },
        } => {
            params.insert(
                "did".to_string(),
                jwt_did.ok_or(anyhow!("need auth token"))?,
            );
            params.insert("collection".to_string(), "app.bsky.post".to_string());
            xrpc_client.post(
                "com.atproto.repoCreateRecord",
                Some(params),
                json!({
                    "text": text,
                }),
            )?
        }
        Command::Bsky {
            cmd: BskyCommand::Repost { uri },
        } => {
            params.insert(
                "did".to_string(),
                jwt_did.ok_or(anyhow!("need auth token"))?,
            );
            params.insert("collection".to_string(), "app.bsky.repost".to_string());
            xrpc_client.post(
                "com.atproto.repoCreateRecord",
                Some(params),
                json!({
                    "subject": uri.to_string(),
                    // TODO: "createdAt": now_timestamp(),
                }),
            )?
        }
        Command::Bsky {
            cmd: BskyCommand::Like { uri },
        } => {
            params.insert(
                "did".to_string(),
                jwt_did.ok_or(anyhow!("need auth token"))?,
            );
            params.insert("collection".to_string(), "app.bsky.like".to_string());
            xrpc_client.post(
                "com.atproto.repoCreateRecord",
                Some(params),
                json!({
                    "subject": uri.to_string(),
                    // TODO: "createdAt": now_timestamp(),
                }),
            )?
        }
        Command::Bsky {
            cmd: BskyCommand::Follow { uri },
        } => {
            params.insert(
                "did".to_string(),
                jwt_did.ok_or(anyhow!("need auth token"))?,
            );
            params.insert("collection".to_string(), "app.bsky.follow".to_string());
            xrpc_client.post(
                "com.atproto.repoCreateRecord",
                Some(params),
                json!({
                    "subject": uri.to_string(),
                    // TODO: "createdAt": now_timestamp(),
                }),
            )?
        }
        Command::Bsky {
            cmd: BskyCommand::Profile { name },
        } => {
            params.insert("name".to_string(), name.to_string());
            xrpc_client.get("app.bsky.getProfile", Some(params))?
        }
        Command::Bsky {
            cmd: BskyCommand::SearchUsers { query },
        } => {
            params.insert("term".to_string(), query);
            xrpc_client.get("app.bsky.getUsersSearch", Some(params))?
        }
    };
    print_result_json(result)?;
    Ok(())
}
