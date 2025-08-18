use clap::Parser;

#[derive(Parser, Debug)]
#[clap(author, version, about = "Expenses web application", long_about = None)]
pub struct Args {
    #[arg(long, default_value_t = String::from(""), help = "The log directory e.g. '/var/logs'. If this is not provided, only logs out to stdout.")]
    pub base_log_dir: String,

    #[arg(
        long,
        help = "Base URL of the application e.g. \"https://example.com\""
    )]
    pub base_url: String,

    #[arg(long, env = "CLIENT_ID", help = "Monzo Client ID")]
    pub client_id: String,

    #[arg(long, env = "CLIENT_SECRET", help = "Monzo Client Secret")]
    pub client_secret: String,

    #[arg(
        long,
        env = "DATABASE_URL",
        help = "PostgreSQL database URL that is compliant with sqlx PgPool e.g. 'postgresql://user:password@db-host:5432/dbname'"
    )]
    pub database_url: String,

    #[arg(long)]
    pub port: u32,

    #[arg(
        long,
        default_value_t = 300u64,
        help = "Interval in seconds for checking which tokens to refresh"
    )]
    pub token_refresh_interval: u64,

    #[arg(
        long,
        default_value_t = 3600u64,
        help = "Time remaining before expiry when a refresh will be triggered"
    )]
    pub token_refresh_threshold: u64,

    #[arg(
        long,
        default_value_t = 3600u64,
        help = "Interval in seconds for polling accounts"
    )]
    pub account_poll_interval: u64,
}

pub fn parse_args() -> Args {
    return Args::parse();
}
