pub(crate) use error::Error;
use fehler::throw;

mod commands;
mod config;
mod cushion;
mod download;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let mut args = std::env::args().skip(1);
    let cmd = args.next().ok_or(Error::MissingArgumentsError)?;
    let mut cfg = config::Config::load().await;
    match cfg {
        Ok(ref mut cfg) => match cmd.as_ref() {
            "add" => commands::add(cfg, args)?,
            "start" => commands::start(&cfg).await?,
            _ => throw!(Error::InvalidArgument),
        },
        Err(Error::IoError(_)) => match cmd.as_ref() {
            "init" => todo!("init command"),
            _ => throw!(Error::UninitError),
        },
        Err(e) => throw!(e),
    }

    let cfg = cfg.unwrap();
    cfg.save().await?;

    Ok(())
}

mod error {
    use std::io;

    #[derive(Debug)]
    pub enum Error {
        SerializeError(toml::ser::Error),
        DeserializeError(toml::de::Error),
        IoError(io::Error),
        RemoteError(reqwest::Error),
        RssError(rss::Error),
        MissingDescriptionError,
        MissingUserError,
        MissingArgumentsError,
        InvalidArgument,
        UninitError,
    }

    impl From<toml::ser::Error> for Error {
        fn from(e: toml::ser::Error) -> Self {
            Self::SerializeError(e)
        }
    }

    impl From<toml::de::Error> for Error {
        fn from(e: toml::de::Error) -> Self {
            Self::DeserializeError(e)
        }
    }

    impl From<io::Error> for Error {
        fn from(e: io::Error) -> Self {
            Self::IoError(e)
        }
    }

    impl From<reqwest::Error> for Error {
        fn from(e: reqwest::Error) -> Self {
            Self::RemoteError(e)
        }
    }

    impl From<rss::Error> for Error {
        fn from(e: rss::Error) -> Self {
            Self::RssError(e)
        }
    }
}
