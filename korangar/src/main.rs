use clap::Parser;
use korangar::Client;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Arguments {
    /// Synchronize the asset cache and exit.
    #[arg(long)]
    sync_cache: bool,

    /// Increase log verbosity: -v info, -vv debug, -vvv trace.
    ///
    /// Logs are written to stderr; the launcher scripts redirect them to
    /// logs/korangar-client-*.log. The environment variable KORANGAR_LOG_LEVEL
    /// (0-3) can be used instead when arguments cannot be passed.
    #[arg(short = 'v', long = "verbose", action = clap::ArgAction::Count, help = "Increase log verbosity (-v info, -vv debug, -vvv trace)")]
    verbose: u8,
}

fn main() {
    let arguments = Arguments::parse();
    let verbosity = korangar::logging::resolve_verbosity(arguments.verbose);

    korangar::logging::init(verbosity);
    korangar::crash::install();

    korangar::logging::log_info!(
        "korangar {} starting (log verbosity {}, {} command line -v flags)",
        env!("CARGO_PKG_VERSION"),
        verbosity,
        arguments.verbose,
    );

    if let Some(mut client) = Client::init(arguments.sync_cache) {
        client.run();
    }

    korangar::logging::log_info!("korangar exited normally");
}
