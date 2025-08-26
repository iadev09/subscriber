use std::time::Duration;

use clap::Parser;

#[derive(Debug, Parser)]
#[command(name = "subscriber", author, version, about = "high performance event subscriber")]
pub struct Options {
    #[arg(long = "redis", env = "REDIS_URL", help = "redis url")]
    pub redis_url: String,

    #[arg(long, env = "BROADCAST_CHANNEL", help = "redis channel")]
    pub channel: String,

    #[arg(long, short = 'w', help = "max concurrent workers count  none unlimited")]
    pub workers: Option<usize>,

    #[arg(short='t', long= "idle" ,value_parser = parse_duration, help = "idle timeout duration for operations",)]
    pub idle_timeout: Option<Duration>,

    #[arg(short = 'g', long = "grace", value_parser = parse_duration, help = "idle timeout duration for operations",)]
    pub grace_timeout: Option<Duration>
}

fn parse_duration(s: &str) -> Result<Duration, humantime::DurationError> {
    humantime::parse_duration(s)
}
