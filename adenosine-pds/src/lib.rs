
use anyhow::Result;
use log::{info, error};
use rouille::{Request, Response, router};

pub fn run_server() -> Result<()> {

    // TODO: log access requests
    // TODO: some static files? https://github.com/tomaka/rouille/blob/master/examples/static-files.rs

    let log_ok = |req: &Request, resp: &Response, elap: std::time::Duration| {
        info!("{} {} ({:?})", req.method(), req.raw_url(), elap);
    };
    let log_err = |req: &Request, elap: std::time::Duration| {
        error!("HTTP handler panicked: {} {} ({:?})", req.method(), req.raw_url(), elap);
    };
    rouille::start_server("localhost:3030", move |request| {
        rouille::log_custom(request, log_ok, log_err, || {
            router!(request,
                (GET) ["/"] => {
                    Response::text("Not much to see here yet!")
                },
                (GET) ["/xrpc/some.method"] => {
                    Response::text("didn't get a thing")
                    // TODO: reply with query params as a JSON body
                },
                (POST) ["/xrpc/other.method"] => {
                    Response::text("didn't get other thing")
                    // TODO: parse and echo back JSON body
                },
                _ => rouille::Response::empty_404()
            )
        })
    });
}
