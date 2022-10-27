use adenosine_cli::*;
use anyhow::anyhow;
use serde_json::Value;
use std::collections::HashMap;

use colored_json::to_colored_json_auto;
use log::{self, debug, info};
use std::io::Write;
use std::path::PathBuf;
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
    Login,
    Logout,
    Info,
    CreateRevocationKey,
}

#[derive(StructOpt)]
enum Command {
    Get {
        uri: String,
    },

    Xrpc {
        method: XrpcMethod,
        nsid: String,
        params: Option<String>,
    },

    /// Sub-commands for managing account
    Account {
        #[structopt(subcommand)]
        cmd: AccountCommand,
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

fn run(opt: Opt) -> Result<()> {
    let xrpc_client = XrpcClient::new(opt.atp_host, opt.auth_token)?;

    match opt.cmd {
        Command::Xrpc {
            method,
            nsid,
            params,
        } => {
            let body: Value = ().into();
            let res = match method {
                // XXX: parse params
                XrpcMethod::Get => xrpc_client.get(nsid, None)?,
                XrpcMethod::Post => xrpc_client.post(nsid, None, body)?,
            };
            if let Some(val) = res {
                writeln!(&mut std::io::stdout(), "{}", to_colored_json_auto(&val)?)?
            };
        }
        Command::Get { uri } => {
            println!("GET: {}", uri);
            /*
            let result = specifier.get_from_api(&mut api_client, expand, hide)?;
            if toml {
                writeln!(&mut std::io::stdout(), "{}", result.to_toml_string()?)?
            } else {
                // "if json"
                writeln!(
                    &mut std::io::stdout(),
                    "{}",
                    to_colored_json_auto(&result.to_json_value()?)?
                )?
            }
            */
        }
        Command::Account {
            cmd:
                AccountCommand::Register {
                    email,
                    username,
                    password,
                },
        } => {
            println!(
                "REGISTER: email={} username={} password={}",
                email, username, password
            );
        }
        _ => {
            unimplemented!("some command");
        }
    }
    Ok(())
}
