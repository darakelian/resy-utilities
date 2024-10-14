use clap::{command, Parser, Subcommand};
use libresy::ResyClientBuilder;

#[derive(Parser)]
#[command(author, version, about)]
struct Cli {
    /// Disables use of the restaurant configuration cache, resulting in a network call.
    #[arg(long, env, action)]
    no_cache: bool,
    #[command(flatten)]
    verbose: clap_verbosity_flag::Verbosity,
    #[arg(short = 'k', long, env, hide_env_values = true)]
    api_key: String,
    #[arg(short, long, env, hide_env_values = true)]
    auth_token: String,
    /// Size of party to get notified for.
    #[arg(short, long, env, default_value_t = 2)]
    party_size: u8,
    /// Flag enabling json output instead of human-readable.
    #[arg(long, action)]
    json: bool,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Displays all notifications currently active.
    List {
        #[arg(long = "id")]
        restaurant_id: Option<u32>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let builder = ResyClientBuilder::new(cli.api_key, cli.auth_token);

    let resy_client = builder.build();

    match &cli.command {
        Commands::List { restaurant_id } => {
            let notifications = resy_client.get_notifications().await?;
            for n in notifications {
                println!("{:?}", n);
            }
        }
    }
    Ok(())
}
