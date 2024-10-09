use std::fmt::Display;
use std::time::Duration;

use anyhow::anyhow;
use chrono::{Days, NaiveDate, NaiveTime};
use clap::{builder::PossibleValue, command, Parser, Subcommand, ValueEnum};
use libresy::resy_data::ReservationSlot;
use libresy::{ResyClient, ResyClientBuilder};

#[derive(Parser)]
#[command(author, version, about)]
struct Cli {
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
    /// Determines how to handle matching reservations
    #[arg(long, env, default_value_t = ReservationTimeMode::Exact)]
    reservation_time_mode: ReservationTimeMode,

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
#[derive(Debug, Clone)]
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

impl Display for ReservationTimeMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::Exact => write!(f, "exact"),
            Self::Earlier => write!(f, "earlier"),
            Self::Later => write!(f, "later"),
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
        /// How many days should be added to the reservation date to determine the real
        /// reservation date. Useful if running the tool with the default date.
        #[arg(long, env)]
        offset: Option<u8>,
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
fn table_type_matches(slot_type: &str, requested_table_type: &Option<String>) -> bool {
    match requested_table_type {
        Some(r) => r.eq_ignore_ascii_case(slot_type),
        None => true,
    }
}

/// Finds a reservation that best matches the time and time_mode provided.
fn get_matching_reservation(
    reservations: &[ReservationSlot],
    time: &NaiveTime,
    table_type: &Option<String>,
    time_mode: &ReservationTimeMode,
) -> Option<ReservationSlot> {
    match time_mode {
        ReservationTimeMode::Exact => {
            return reservations
                .iter()
                .find(|&reservation_slot| {
                    reservation_slot.date.to_datetime().time() == *time
                        && table_type_matches(&reservation_slot.config.slot_type, table_type)
                })
                .cloned();
        }
        ReservationTimeMode::Earlier => {
            return reservations
                .iter()
                .filter(|&r| {
                    r.date.to_datetime().time() <= *time
                        && table_type_matches(&r.config.slot_type, table_type)
                })
                .last()
                .cloned();
        }
        ReservationTimeMode::Later => {
            return reservations
                .iter()
                .find(|&r| {
                    r.date.to_datetime().time() >= *time
                        && table_type_matches(&r.config.slot_type, table_type)
                })
                .cloned();
        }
    }
}

async fn attempt_reservation(
    resy_client: &ResyClient,
    restaurant_id: &String,
    date: &NaiveDate,
    time: &NaiveTime,
    party_size: u8,
    table_type: &Option<String>,
    time_mode: &ReservationTimeMode,
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
    let matching_reservation = get_matching_reservation(&reservations, time, table_type, time_mode);
    match matching_reservation {
        Some(r) => {
            // Get the reservation details to book. For now, let's assume if we got a reservation slot
            // that this function won't fail.
            let reservation_details = resy_client
                .get_reservation_details(&r, date, party_size)
                .await?;
            // This naively also assumes that the reservation will book properly for now.
            let booking_res = resy_client
                .book_restaurant(
                    &reservation_details.book_token,
                    &reservation_details.get_payment_id().unwrap(),
                )
                .await;
            match booking_res {
                Ok(b) => Ok(b),
                Err(e) => Err(e),
            }
        }
        None => Err(anyhow!(
            "No reservation was found for the given time and time_mode"
        )),
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let mut date = get_default_date(cli.date);

    let builder = ResyClientBuilder::new(cli.api_key, cli.auth_token);

    let mut resy_client = builder.build();

    resy_client.load_config().await?;

    let requested_time = NaiveTime::parse_from_str(&cli.time, "%H:%M").unwrap();
    println!("Checking for reservations on {:?}", date);

    match &cli.command {
        Commands::Automatic {
            retry_count,
            retry_delay,
            offset,
        } => {
            println!("User requested automatic mode");
            if let Some(offset) = offset {
                date = date.checked_add_days(Days::new(*offset as u64)).unwrap();
            }
            for i in 0..*retry_count {
                println!(
                    "On try {} out ouf {} to book a reservation.",
                    i + 1,
                    retry_count
                );
                let reservation_attempt = attempt_reservation(
                    &resy_client,
                    &cli.restaurant_id,
                    &date,
                    &requested_time,
                    cli.party_size,
                    &cli.table_type,
                    &cli.reservation_time_mode,
                )
                .await;
                match reservation_attempt {
                    Ok(_) => {
                        println!("Booked your reservation, you should be receiving a confirmation email from Resy!");
                        return Ok(());
                    }
                    Err(e) => {
                        println!(
                            "Encountered error on this attempt: {}, retrying in {} seconds",
                            e, retry_delay
                        );
                        if i < *retry_count - 1 {
                            async_std::task::sleep(Duration::from_secs(*retry_delay as u64)).await;
                        }
                    }
                }
            }
        }
        Commands::OneShot => {
            println!("User requested one-shot mode");
            attempt_reservation(
                &resy_client,
                &cli.restaurant_id,
                &date,
                &requested_time,
                cli.party_size,
                &cli.table_type,
                &cli.reservation_time_mode,
            )
            .await?;
        }
    }

    Ok(())
}
