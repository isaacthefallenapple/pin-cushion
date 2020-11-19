use {
    super::{cushion, Error},
    dirs,
    fehler::throws,
    serde::{Deserialize, Serialize},
    std::{
        collections::HashMap,
        path::{Path, PathBuf},
    },
    tokio::{
        fs,
        io::{self, AsyncWriteExt},
    },
};

const CONFIG_PATH: &str = ".pin-cushion.toml";

#[derive(Serialize, Deserialize)]
pub struct Config {
    pin_dir: String,
    default_user: Option<String>,
    boards: HashMap<String, Vec<String>>,
}

impl Config {
    #[throws]
    pub fn _init(pin_dir: impl AsRef<Path>) -> Self {
        let pin_dir = pin_dir
            .as_ref()
            .to_str()
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "pin_dir path must be valid UTF-8",
                )
            })?
            .to_string();

        Self {
            pin_dir,
            default_user: None,
            boards: HashMap::new(),
        }
    }

    #[throws]
    pub fn _init_with_default_user(pin_dir: impl AsRef<Path>, default_user: String) -> Self {
        let mut cfg = Self::_init(pin_dir)?;
        cfg.default_user.replace(default_user);
        cfg
    }

    #[throws]
    pub fn add_board(&mut self, user: &str, board: &str, url: &str) -> cushion::Cushion {
        self.boards
            .entry(user.to_string())
            .or_default()
            .push(board.to_string());
        cushion::Cushion::new(&self, user.to_string(), board.to_string(), url.to_string())?
    }

    #[throws]
    pub async fn save(&self) {
        let toml_str = toml::to_string(&self)?;
        let mut save_file = fs::File::create(Self::save_path()?).await?;
        save_file.write(toml_str.as_bytes()).await?;
        save_file.flush().await?;
    }

    #[throws]
    pub async fn load() -> Self {
        let config_path = Self::save_path()?;
        let s = fs::read_to_string(config_path).await?;
        toml::from_str(&s)?
    }

    #[throws]
    fn home_dir() -> PathBuf {
        dirs::home_dir().ok_or_else(|| std::io::Error::from(std::io::ErrorKind::NotFound))?
    }

    #[throws]
    fn save_path() -> impl AsRef<Path> {
        let mut home = Self::home_dir()?;
        home.reserve_exact(1);
        home.push(CONFIG_PATH);
        home
    }

    pub fn boards(&self) -> &HashMap<String, Vec<String>> {
        &self.boards
    }

    pub fn default_user(&self) -> Option<&str> {
        self.default_user.as_deref()
    }

    pub fn pin_dir(&self) -> &Path {
        &self.pin_dir.as_ref()
    }
}
