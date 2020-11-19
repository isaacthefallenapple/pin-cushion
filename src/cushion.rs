use {
    super::{config::Config, download, Error},
    fehler::throws,
    reqwest::Client,
    serde::{Deserialize, Serialize},
    std::path::{Path, PathBuf},
    tokio::{fs, io::AsyncWriteExt, stream::StreamExt},
};

const CUSHION_PATH: &str = ".cushion.toml";

/// A `Cushion` manages the state of a Pinterest board.
/// It stores the owner, the name, and the url of the board,
/// as well as the latest Pin that has been downloaded.
///
/// `Cushion` implements `Drop` and is saved to disk everytime
/// it gets dropped.
#[derive(Serialize, Deserialize)]
pub struct Cushion {
    user: String,
    board: String,
    url: String,
    path: PathBuf,
    latest_download: String,
    #[serde(skip)]
    client: Client,
}

impl Cushion {
    /// Constructs a new `Cushion` for a board found at `url`.
    ///
    /// `user` is the owner of the board and `board` is its name.
    #[throws]
    pub fn new(cfg: &Config, user: String, board: String, url: String) -> Self {
        let path = Self::construct_path(cfg, &user, &board);
        std::fs::create_dir_all(&path)?;

        let url: String = if url.starts_with("https://www.pinterest") && url.ends_with(".rss") {
            url
        } else {
            format!("https://www.pinterest.com/{}/{}.rss", &user, &url)
        };

        let cushion = Self {
            user,
            board,
            url,
            path,
            latest_download: Default::default(),
            client: Client::new(),
        };

        cushion
    }

    /// Updates the cushion, fetching the rss feed and downloading any newly added Pins.
    ///
    /// Returns how many Pins were downloaded.
    #[throws]
    pub async fn update(&mut self) -> u32 {
        let resp = self.client.get(&self.url).send().await?.bytes().await?;
        let channel = rss::Channel::read_from(&resp[..])?;
        let latest = self.latest_download.clone();
        let mut downloaded = 0;
        let items = channel
            .items()
            .iter()
            .take_while(|item| item.guid().map(rss::Guid::value).unwrap_or("\u{fffd}") != latest);

        let mut item_stream = tokio::stream::iter(items);

        if let Some(item) = item_stream.next().await {
            if self.download(item).await? {
                downloaded += 1;
            }

            self.latest_download = item
                .guid()
                .map(|id| id.value().to_string())
                .unwrap_or_default();
        }

        let mut item_stream = item_stream.map(|item| self.download(item));
        while let Some(b) = item_stream.next().await {
            if let Ok(true) = b.await {
                downloaded += 1;
            }
        }

        if downloaded >= 1 {
            self.save().await?;
        }

        downloaded
    }

    /// Download a Pin.
    #[throws]
    async fn download(&self, item: &rss::Item) -> bool {
        download::download_pin(
            &self.client,
            self.path(),
            item.description().ok_or(Error::MissingDescriptionError)?,
        )
        .await?
    }

    /// Saves this cushion to disk at `self.save_path()`.
    #[throws]
    pub async fn save(&self) {
        let mut save_file = fs::File::create(self.save_path()).await?;
        let toml_str = toml::to_string_pretty(&self)?;
        save_file.write(toml_str.as_bytes()).await?;
        save_file.flush().await?;
    }

    /// Loads a cushion from disk at `<pin_dir>/<user>/<board>/.cushion.toml`
    /// where `<pin_dir>` is specified in `cfg`.
    #[throws]
    pub async fn load(cfg: &Config, user: &str, board: &str) -> Self {
        let mut path = Self::construct_path(cfg, user, board);
        path.reserve_exact(1);
        path.push(CUSHION_PATH);
        let toml_str = fs::read_to_string(path).await?;
        toml::from_str(&toml_str)?
    }

    /// Return the location of this cushion's board on disk.
    fn path(&self) -> &Path {
        &self.path
    }

    /// Like `path` but appends the actual filepath of this cushion
    /// on disk.
    fn save_path(&self) -> impl AsRef<Path> {
        self.path.join(CUSHION_PATH)
    }

    /// Constructs the path `<pin_dir>/<user>/<board>` where `<pin_dir>`
    /// is specified in `cfg`.
    fn construct_path(cfg: &Config, user: &str, board: &str) -> PathBuf {
        [
            cfg.pin_dir()
                .to_str()
                .expect("path needs to be valid UTF-8"),
            user,
            board,
        ]
        .iter()
        .collect()
    }

    /// Returns the name of this cushion's board.
    pub fn _board(&self) -> &str {
        &self.board
    }
}
