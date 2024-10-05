use std::{error::Error, path::PathBuf};

use anyhow::anyhow;
use clap::Parser;
use libresy::ResyClientBuilder;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg(long, env, default_value = "US")]
    country: String,
    #[arg(short, long, env)]
    city: String,
    #[arg(trailing_var_arg = true)]
    restaurant_names: Vec<String>,
    #[command(flatten)]
    verbose: clap_verbosity_flag::Verbosity,
    #[arg(short = 'k', long, env)]
    api_key: String,
    #[arg(short, long, env)]
    auth_token: String,
    #[arg(long, env, action)]
    no_cache: bool,
    #[arg(short, long, env, action)]
    /// If enabled, search names must match exactly
    strict_match: bool
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let mut resy_client = ResyClientBuilder::new()
        .api_key(cli.api_key)
        .auth_key(cli.auth_token)
        .no_cache(cli.no_cache)
        .build();

    resy_client.load_config().await?;

    let restaurant_name = cli.restaurant_names.join(" ");
    
    // Try and find a matching restaurant config for the city/country/restaurant_name
    let city_config = resy_client.get_restaurant_city_config(&cli.city, &cli.country).unwrap_or_else(|| panic!("No city {} was found in country {}", cli.city, cli.country));

    // After we have the city, lets try to find the restaurant
    let restaurant = resy_client.find_restaurant(&city_config, &restaurant_name).await?;
    match restaurant {
        Some(r) =>  println!("Found \"{}\": use ID {} for other Resy requests", restaurant_name, r.object_id),
        None => return Err(anyhow!(format!("Unable to find a restaurant {} in {}", restaurant_name, cli.city)))
    }
    Ok(())
}
