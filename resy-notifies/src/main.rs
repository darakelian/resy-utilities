use anyhow::anyhow;
use clap::{command, Parser, Subcommand};
use libresy::{
    resy_data::{ResyNotification, ResyNotificationSpec},
    ResyClientBuilder,
};

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
    /// List active notifications.
    List {
        #[arg(long = "id")]
        restaurant_id: Option<u32>,
    },
    /// Delete operations on notifications.
    Delete {
        /// Delete all notifications.
        #[arg(long, action)]
        all: bool,
        /// Date the notification was for.
        #[arg(long, short)]
        date: Option<String>,
        /// ID of restaurant the notification was for.
        #[arg(long, short)]
        restaurant_id: Option<u32>,
        /// Party size the notification was for.
        #[arg(long, short)]
        num_seats: Option<u8>,
        /// Service type the notification was for.
        #[arg(long, short)]
        service_type_id: Option<u8>,
    },
    /// Create/update notifications.
    Create {
        /// Date the notification is for.
        #[arg(long, short)]
        date: String,
        /// ID of restaurant the notification is for.
        #[arg(long, short)]
        restaurant_id: u32,
        /// Party size the notification is for.
        #[arg(long, short)]
        num_seats: u8,
        /// Service type the notification is for.
        #[arg(long = "type", short = 't')]
        service_type_id: u8,
        /// Start time to set the notification for (HH:MM)
        #[arg(long, short)]
        start_time: String,
        /// End time to set the notification for (HH:MM)
        #[arg(long, short)]
        end_time: String,
    },
}

fn notifications_filter(notification: &ResyNotification, restaurant_id: &Option<u32>) -> bool {
    if restaurant_id.is_none() {
        return true;
    }
    notification.specs.venue_id == restaurant_id.unwrap()
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let builder = ResyClientBuilder::new(cli.api_key, cli.auth_token);

    let resy_client = builder.build();

    match &cli.command {
        Commands::List { restaurant_id } => {
            let notifications = resy_client.get_notifications().await?;
            let notifications_iter = notifications
                .iter()
                .filter(|p| notifications_filter(p, restaurant_id));
            for n in notifications_iter {
                println!("{:?}", n);
            }
        }
        Commands::Delete {
            all,
            date,
            restaurant_id,
            num_seats,
            service_type_id,
        } => {
            if *all {
                let notifications = resy_client.get_notifications().await?;
                for n in notifications {
                    resy_client.delete_notification(&n).await?
                }
            } else {
                if date.is_none()
                    || restaurant_id.is_none()
                    || num_seats.is_none()
                    || service_type_id.is_none()
                {
                    return Err(anyhow!("You must specify date, restaurant_id, num_seats, and service_type_id when trying to delete a single notification."));
                }
                let day = date.clone().unwrap();
                let notification_to_delete = ResyNotification {
                    specs: ResyNotificationSpec {
                        venue_id: restaurant_id.unwrap(),
                        party_size: num_seats.unwrap(),
                        day,
                        time_preferred_start: "".to_string(),
                        time_preferred_end: "".to_string(),
                        service_type_id: service_type_id.unwrap(),
                    },
                };
                resy_client
                    .delete_notification(&notification_to_delete)
                    .await
                    .expect("Unable to delete notification");
            }
        }
        Commands::Create {
            date,
            restaurant_id,
            num_seats,
            service_type_id,
            start_time,
            end_time,
        } => {
            let day = date.clone();
            let notification_to_create = ResyNotification {
                specs: ResyNotificationSpec {
                    venue_id: *restaurant_id,
                    party_size: *num_seats,
                    day,
                    time_preferred_start: start_time.clone(),
                    time_preferred_end: end_time.clone(),
                    service_type_id: *service_type_id,
                },
            };
            resy_client
                .create_notification(&notification_to_create)
                .await
                .expect("Unable to create notification");
        }
    }
    Ok(())
}
