use {
    super::*,
    fehler::throws,
    std::{
        io::{stdout, Write},
        time::Duration,
    },
};

/// Adds a new board. Expects the name of the board and either the URL
/// of the board's RSS feed or the last path segment of the board's URL.
///
/// If `default_user` is not specified in `cfg`, it also expects a user
/// to be passed to the `--user` option before any other arguments.
#[throws]
pub fn add(cfg: &mut config::Config, args: impl Iterator<Item = String>) {
    parse::add(cfg, args, None, None, None)?;
}

#[throws]
pub async fn start(cfg: &config::Config) {
    let (tx, rx) = tokio::sync::watch::channel(true);

    let mut handles = Vec::new();

    for (user, boards) in cfg.boards() {
        for board in boards {
            let mut cushion = cushion::Cushion::load(&cfg, user, board).await?;
            let mut rx = rx.clone();
            let handle = tokio::spawn(async move {
                while let Some(true) = rx.recv().await {
                    if let Err(e) = cushion.update().await {
                        eprintln!("something went wrong, re-trying next time: {:?}", e);
                    }
                }
                println!("cancelling task");
                return Ok(()) as Result<_, Error>;
            });
            handles.push(handle);
        }
    }

    let (cancel_tx, mut cancel_rx) = tokio::sync::oneshot::channel();

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        loop {
            print!("listening...\r");
            stdout().flush().expect("couldn't flush to stdout.");
            tokio::select! {
                _ = interval.tick() => {
                    let _ = tx.broadcast(true);
                }
                Ok(_) = &mut cancel_rx => {
                    tx.broadcast(false).expect("failed to cancel");
                    break;
                }
            }
        }
    });
    drop(rx);

    let mut input = String::with_capacity(5);
    loop {
        std::io::stdin().read_line(&mut input)?;
        if input.trim() == "stop" {
            cancel_tx.send(()).expect("cancellation failed");
            break;
        } else {
            eprintln!("To stop listening to your feeds, type \"stop\".");
            input.clear();
        }
    }

    for handle in handles {
        handle.await.expect("failed to join task")?;
    }
}

mod parse {
    use super::*;

    #[throws]
    pub(super) fn add(
        cfg: &mut config::Config,
        mut args: impl Iterator<Item = String>,
        mut user: Option<String>,
        mut board: Option<String>,
        mut url: Option<String>,
    ) {
        if let (Some(user), Some(board), Some(url)) = (&user, &board, &url) {
            cfg.add_board(user, board, url)?;
            return;
        }
        let arg = args.next().ok_or(Error::MissingArgumentsError)?;
        if user.is_none() {
            if arg == "--user" {
                user = args.next();
            } else {
                user.replace(
                    cfg.default_user()
                        .map(ToString::to_string)
                        .ok_or(Error::MissingUserError)?,
                );
                board.replace(arg);
            }
        } else if board.is_none() {
            board.replace(arg);
        } else if url.is_none() {
            url.replace(arg);
        }
        add(cfg, args, user, board, url)?;
    }

    #[throws]
    pub fn _init(
        mut _args: impl Iterator<Item = String>,
        _pin_dir: Option<String>,
        _default_user: Option<String>,
    ) {
        // todo!()
    }
}
