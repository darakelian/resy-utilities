use anyhow::anyhow;
use chrono::{NaiveDate, NaiveTime};
use clap::{builder::PossibleValue, command, Parser, Subcommand, ValueEnum};
use libresy::{ResyClient, ResyClientBuilder};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Country you want to search for restaurants in.
    #[arg(long, env, default_value = "US")]
    country: String,
    /// City you want to search for restaurants in.
    #[arg(short, long, env)]
    city: String,
    /// Resy ID of the restaurant you are trying to reserve. See documentation for
    /// finding this value.
    #[arg(long = "id")]
    restaurant_id: String,
    #[command(flatten)]
    verbose: clap_verbosity_flag::Verbosity,
    #[arg(short = 'k', long, env, hide_env_values = true)]
    api_key: String,
    #[arg(short, long, env, hide_env_values = true)]
    auth_token: String,
    /// Size of party to find tables for.
    #[arg(short, long, env, default_value_t = 2)]
    party_size: u8,
    #[arg(short, long, env)]
    date: Option<String>,
    /// Time you want to try and reserve a reservation at.
    #[arg(short, long, env)]
    time: String,
    /// Optional type of table (Indoor, Outdoor, etc.) if you care about sitting
    /// at a specific spot. Check the restaurant itself for valid values. If no
    /// value is specified, time will be the deciding factor.
    #[arg(long, env)]
    table_type: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

/// Controls how to handle finding the best reservation match if there isn't one at
/// the provided time.
///
/// Exact: Reservation time must exactly match the provided time.
///
/// Earlier: Will consider reservations with earlier times (closer times will be preferred.)
///
/// Later: Will consider reservations with later times (closer times will be preferred.)
#[derive(Clone)]
enum ReservationTimeMode {
    Exact,
    Earlier,
    Later,
}

impl ValueEnum for ReservationTimeMode {
    fn value_variants<'a>() -> &'a [Self] {
        &[Self::Exact, Self::Earlier, Self::Later]
    }

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        match self {
            Self::Exact => Some(PossibleValue::new("exact")),
            Self::Earlier => Some(PossibleValue::new("earlier")),
            Self::Later => Some(PossibleValue::new("later")),
        }
    }
}

#[derive(Subcommand)]
enum Commands {
    /// Enables one-shot reservation sniping mode. User is responsible for handling
    /// any retries in this mode.
    OneShot,
    /// Enables automatic reservation sniping mode. The application will automatically
    /// handle waiting and retrying to find reservations in this mode.
    Automatic {
        /// Controls how many times reservations will be refreshed to check for new
        /// reservations.
        #[arg(long, env, default_value_t = 5)]
        retry_count: u8,
        /// Controls how long the program will wait between retry attempts in seconds.
        #[arg(long, env, default_value_t = 1)]
        retry_delay: u16,
        /// Determines how to handle matching reservations
        #[arg(long, env)]
        reservation_time_mode: ReservationTimeMode,
    },
}

/// Normalizes the date from YYYYMMDD to YYYY-MM-DD for Resy requests. Will use
/// today's date if the user did not provide one.
fn get_default_date(provided_date: Option<String>) -> NaiveDate {
    match provided_date {
        Some(p) => {
            // Try to parse using our format of YYYYMMDD, if it fails, user entered
            // the wrong date format
            NaiveDate::parse_from_str(&p, "%Y%m%d")
                .expect("ERROR: Date must be in YYYYMMDD format!")
        }
        None => chrono::Local::now().date_naive(),
    }
}

/// Checks if the reservation slot matches the user's requested table type. If the
/// user hasn't provided a preference, this will always return true.
fn table_type_matches(slot_type: &String, requested_table_type: &Option<String>) -> bool {
    match requested_table_type {
        Some(r) => r.eq_ignore_ascii_case(slot_type),
        None => true,
    }
}

async fn attempt_reservation(
    resy_client: &ResyClient,
    restaurant_id: &String,
    date: &NaiveDate,
    time: &NaiveTime,
    party_size: u8,
    table_type: &Option<String>,
) -> anyhow::Result<()> {
    // Using the resy_id, get the reservations available
    let reservations = resy_client
        .get_reservations(restaurant_id, date, party_size)
        .await?;
    if reservations.is_empty() {
        return Err(anyhow!(
            "No reservations exist at the restaurant for the given date and party size"
        ));
    }
    // Find a reservation that matches the time requested
    let matching_reservation = reservations
        .iter()
        .find(|&reservation_slot| {
            reservation_slot.date.to_datetime().time() == *time
                && table_type_matches(&reservation_slot.config.slot_type, table_type)
        })
        .expect("No reservations were found for the given time. Try a new time?");

    // Get the reservation details to book. For now, let's assume if we got a reservation slot
    // that this function won't fail.
    let reservation_details = resy_client
        .get_reservation_details(matching_reservation, date, party_size)
        .await?;
    // This naively also assumes that the reservation will book properly for now.
    let _ = resy_client
        .book_restaurant(
            &reservation_details.book_token,
            &reservation_details.get_payment_id().unwrap(),
        )
        .await?;
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let date = get_default_date(cli.date);

    let builder = ResyClientBuilder::new(cli.api_key, cli.auth_token);

    let mut resy_client = builder.build();

    resy_client.load_config().await?;

    match &cli.command {
        Commands::Automatic {
            retry_count,
            retry_delay,
            reservation_time_mode,
        } => {
            println!("User requested automatic mode");
        }
        Commands::OneShot => {
            println!("User requested one-shot mode");
            attempt_reservation(
                &resy_client,
                &cli.restaurant_id,
                &date,
                &NaiveTime::parse_from_str(&cli.time, "%H:%M").unwrap(),
                cli.party_size,
                &cli.table_type,
            )
            .await?;
        }
    }

    Ok(())
}
