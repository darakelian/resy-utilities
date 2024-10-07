use std::{error::Error, path::PathBuf};

use anyhow::anyhow;
use chrono::{DateTime, Datelike};
use clap::{CommandFactory, Parser};
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
    #[arg(short = 'k', long, env, hide_env_values = true)]
    api_key: String,
    #[arg(short, long, env, hide_env_values = true)]
    auth_token: String,
    #[arg(long, env, action)]
    no_cache: bool,
    #[arg(short, long, env, action)]
    /// If enabled, search names must match exactly
    strict_match: bool,
    /// Size of party to find tables for.
    #[arg(short, long, env, default_value_t = 2)]
    party_size: u8,
    #[arg(short, long, env)]
    date: Option<String>,
}

/// Normalizes the date from YYYYMMDD to YYYY-MM-DD for Resy requests. Will use
/// today's date if the user did not provide one.
fn get_default_date(provided_date: Option<String>) -> String {
    match provided_date {
        Some(p) => {
            // Try to parse using our format of YYYYMMDD, if it fails, user entered
            // the wrong date format
            let dt = DateTime::parse_from_str(&p, "%Y%M%d")
                .expect("ERROR: Date must be in YYYYMMDD format!");
            // Convert to YYYY-MM-DD format
            format!("{}-{:0>2}-{:0>2}", dt.year(), dt.month(), dt.day())
        }
        None => {
            let current_date = chrono::Local::now();
            format!(
                "{}-{:0>2}-{:0>2}",
                current_date.year(),
                current_date.month(),
                current_date.day()
            )
        }
    }
}
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Since restaurant name can be multiple positional args, check that the user
    // actually provided them
    if cli.restaurant_names.len() == 0 {
        let _ = Cli::command().print_help();
        return Err(anyhow!("You must provide a restaurant name to search for!"));
    }

    let date = get_default_date(cli.date);

    let mut builder = ResyClientBuilder::new(cli.api_key, cli.auth_token);
    if cli.no_cache {
        builder = builder.no_cache();
    }
    if cli.strict_match {
        builder = builder.strict_match();
    }

    let mut resy_client = builder.build();

    resy_client.load_config().await?;

    let restaurant_name = cli.restaurant_names.join(" ");

    println!(
        "Looking for reservations at {} on {} for a party size of {}",
        restaurant_name, date, cli.party_size
    );

    // Try and find a matching restaurant config for the city/country/restaurant_name
    let city_config = resy_client
        .get_restaurant_city_config(&cli.city, &cli.country)
        .unwrap_or_else(|| panic!("No city {} was found in country {}", cli.city, cli.country));

    // After we have the city, lets try to find the restaurant
    let restaurant = resy_client
        .find_restaurant(&city_config, &restaurant_name)
        .await?;
    match restaurant {
        Some(r) => {
            let reservations = resy_client
                .get_reservations(&r.object_id, &date, cli.party_size)
                .await?;
            if reservations.len() > 0 {
                // Print the reservations
                for reservation in reservations.iter() {
                    println!("{:?}", reservation);
                }
            } else {
                println!(
                    "There are no reservations at {} on {} for a party size of {}",
                    r.name, date, cli.party_size
                );
            }
        }
        None => {
            return Err(anyhow!(format!(
                "Unable to find a restaurant {} in {}",
                restaurant_name, cli.city
            )))
        }
    }
    Ok(())
}
