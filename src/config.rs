use std::{path::Path, sync::OnceLock};

use anyhow::Context;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Config {
    pub riot_apikey: String,
    pub gemini_apikey: String,
    pub discord_token: String,
    /// Players whose matches will be processed.
    pub players: Vec<User>,
    /// Guilds which the bot will register the command to.
    pub guild_ids: Vec<u64>,
}

#[derive(Deserialize)]
pub struct User {
    /// League of legends summoner name without the region tag.
    pub summoner_name: String,
    pub discord_name: String,
}

static CONFIG: OnceLock<Config> = OnceLock::new();

impl Config {
    /// Parses the config file into `Config`.
    pub fn parse(path: impl AsRef<Path>) -> anyhow::Result<()> {
        let content = std::fs::read_to_string(path).context("Failed to read config file")?;
        let config = toml::from_str(&content).context("Failed to parse config file")?;
        CONFIG.set(config).map_err(|_| ()).unwrap();

        Ok(())
    }

    /// Returns the `Config`. Must only be called after `parse`.
    pub fn get() -> &'static Config {
        CONFIG.get().expect("Config::parse is not called")
    }
}
