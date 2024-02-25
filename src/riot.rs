use std::{
    collections::{HashMap, HashSet},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::Context;
use chrono::{DateTime, TimeZone};
use futures::future::join_all;
use riven::{
    consts::{GameMode, PlatformRoute, RegionalRoute, Team},
    RiotApi,
};

use crate::config::Config;

async fn get_match_ids_of_summoner(
    api: &RiotApi,
    summoner_name: &str,
) -> anyhow::Result<Vec<String>> {
    tracing::info!("Getting match for {}", summoner_name);

    let summoner = api
        .summoner_v4()
        .get_by_summoner_name(PlatformRoute::NA1, summoner_name)
        .await
        .context("Failed to get summoner")?
        .context("Summoner not found")?;
    const SECONDS_IN_DAY: u64 = 60 * 60 * 24;
    api.match_v5()
        .get_match_ids_by_puuid(
            RegionalRoute::AMERICAS,
            &summoner.puuid,
            Some(5),
            None,
            None,
            Some(
                (SystemTime::now() - Duration::from_secs(SECONDS_IN_DAY))
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64
                    - 3600,
            ),
            None,
            None,
        )
        .await
        .context(format!(
            "Failed to get previous matches for {summoner_name}"
        ))
}

async fn get_relevant_match_ids(api: &RiotApi) -> Vec<String> {
    let results = join_all(
        Config::get()
            .players
            .iter()
            .map(|player| get_match_ids_of_summoner(api, &player.summoner_name)),
    )
    .await;

    results
        .into_iter()
        .filter_map(|r| {
            if let Err(err) = &r {
                tracing::warn!("Fail to get match IDs: {:?}", err);
            }

            r.ok()
        })
        .flatten()
        .collect::<HashSet<_>>()
        .into_iter()
        .collect()
}

pub struct Match {
    #[allow(dead_code)]
    id: String,
    time: DateTime<chrono::Local>,
    game_mode: GameMode,
    is_victory: bool,
    participants: Vec<ParticipantData>,
}

impl std::fmt::Display for Match {
    /// Returns the match data that will be given to Gen AI.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Game Mode: {}\n", self.game_mode)?;
        writeln!(
            f,
            "Game Result: {}\n",
            if self.is_victory { "Victory" } else { "Defeat" }
        )?;
        writeln!(
            f,
            "{}",
            self.participants
                .iter()
                .map(|p| p.display(self.game_mode.clone()).to_string())
                .collect::<Vec<_>>()
                .join("\n")
        )?;
        Ok(())
    }
}

impl Match {
    /// Displays the match data that will be shown in discord
    pub fn human_format(&self) -> impl std::fmt::Display + '_ {
        struct Format<'a>(&'a Match);
        impl std::fmt::Display for Format<'_> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                let inner = self.0;
                writeln!(
                    f,
                    "Game Mode: {}, Time: {}, Result: {}",
                    inner.game_mode,
                    inner.time.format("%Y/%m/%d %H:%M"),
                    if inner.is_victory {
                        "Victory"
                    } else {
                        "Defeat"
                    }
                )?;
                writeln!(
                    f,
                    "{}",
                    inner
                        .participants
                        .iter()
                        .map(|p| p.human_format(inner.game_mode.clone()).to_string())
                        .collect::<Vec<_>>()
                        .join("\n")
                )?;
                Ok(())
            }
        }

        Format(self)
    }
}

struct ParticipantData {
    summoner_name: String,
    #[allow(dead_code)]
    discord_name: String,
    champion_name: String,
    team_position: String,
    kills: i32,
    deaths: i32,
    assists: i32,
    gold_earned: i32,
    total_minions_killed: i32,
    total_damage_dealt_to_champions: i32,
    vision_score: i32,
}

impl ParticipantData {
    fn new(participant: riven::models::match_v5::Participant, discord_name: String) -> Self {
        Self {
            summoner_name: participant.summoner_name,
            discord_name,
            champion_name: participant.champion_name,
            team_position: participant.team_position,
            kills: participant.kills,
            deaths: participant.deaths,
            assists: participant.assists,
            gold_earned: participant.gold_earned,
            total_minions_killed: participant.total_minions_killed,
            total_damage_dealt_to_champions: participant.total_damage_dealt_to_champions,
            vision_score: participant.vision_score,
        }
    }

    pub fn display(&self, mode: GameMode) -> impl std::fmt::Display + '_ {
        struct Format<'a> {
            inner: &'a ParticipantData,
            mode: GameMode,
        }
        impl std::fmt::Display for Format<'_> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                let inner = self.inner;
                let mode = self.mode.clone();
                writeln!(f, "{}:", inner.summoner_name)?;
                writeln!(f, "  Champion: {}", inner.champion_name)?;
                if mode != GameMode::ARAM {
                    writeln!(f, "  Position: {}", inner.team_position)?;
                }
                writeln!(f, "  Kills: {}", inner.kills)?;
                writeln!(f, "  Deaths: {}", inner.deaths)?;
                writeln!(f, "  Assists: {}", inner.assists)?;
                writeln!(f, "  Gold: {}", inner.gold_earned)?;
                writeln!(f, "  CS: {}", inner.total_minions_killed)?;
                writeln!(
                    f,
                    "  Damage to Champions: {}",
                    inner.total_damage_dealt_to_champions
                )?;
                if mode != GameMode::ARAM {
                    writeln!(f, "  Vision Score: {}", inner.vision_score)?;
                }

                Ok(())
            }
        }
        Format { inner: self, mode }
    }

    pub fn human_format(&self, mode: GameMode) -> impl std::fmt::Display + '_ {
        struct Format<'a> {
            inner: &'a ParticipantData,
            mode: GameMode,
        }
        impl std::fmt::Display for Format<'_> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                let inner = self.inner;
                let mode = self.mode.clone();
                writeln!(f, "{}:", inner.summoner_name)?;
                write!(f, "  Champion: {}", inner.champion_name)?;
                if mode != GameMode::ARAM {
                    write!(f, ", Position: {}", inner.team_position)?;
                }
                writeln!(
                    f,
                    ", KDA: {}/{}/{}",
                    inner.kills, inner.deaths, inner.assists
                )?;
                write!(
                    f,
                    "  Gold: {}, CS: {}",
                    inner.gold_earned, inner.total_minions_killed
                )?;
                write!(
                    f,
                    ", Damage to Champions: {}",
                    inner.total_damage_dealt_to_champions
                )?;
                if mode != GameMode::ARAM {
                    write!(f, ", Vision Score: {}", inner.vision_score)?;
                }
                Ok(())
            }
        }
        Format { inner: self, mode }
    }
}

/// Retrieves the match data for the given match ID.
///
/// Returns `None` if the match does not satisfy the requirements:
/// - The game mode must be `CLASSIC` or `ARAM`.
/// - There must be at least 2 users (friends) participating in the match.
async fn process_match(
    api: &RiotApi,
    match_id: &str,
    all_users: &HashMap<String, String>,
) -> anyhow::Result<Option<Match>> {
    tracing::info!("Processing match {}", match_id);

    let info = api
        .match_v5()
        .get_match(RegionalRoute::AMERICAS, match_id)
        .await?
        .context("Failed to get match info")?
        .info;
    if info.game_mode != GameMode::CLASSIC && info.game_mode != GameMode::ARAM {
        return Ok(None);
    }

    let mut participants = vec![];
    let mut team_id = Team::ZERO;
    for participant in info.participants {
        let summoner_name = participant.summoner_name.clone();
        let Some(discord_name) = all_users.get(&summoner_name) else {
            continue;
        };
        team_id = participant.team_id;
        let data = ParticipantData::new(participant, discord_name.clone());
        participants.push(data);
    }
    if participants.len() <= 1 {
        return Ok(None);
    }
    let is_victory = info
        .teams
        .iter()
        .find(|team| team.team_id == team_id)
        .map(|team| team.win)
        .unwrap_or(false);

    Ok(Some(Match {
        id: match_id.to_owned(),
        time: chrono::Local
            .timestamp_opt(info.game_start_timestamp, 0)
            .single()
            .context("Failed to parse timestamp")?,
        game_mode: info.game_mode,
        is_victory,
        participants,
    }))
}

/// Gets the most recent match that satisfies the requirements.
///
/// See `process_match` for the requirements.
pub async fn get_most_recent_match() -> anyhow::Result<Option<Match>> {
    let api = RiotApi::new(&Config::get().riot_apikey);
    let all_users = Config::get()
        .players
        .iter()
        .map(|user| (user.summoner_name.clone(), user.discord_name.clone()))
        .collect::<HashMap<_, _>>();

    let relevant_match_ids = get_relevant_match_ids(&api).await;
    tracing::info!("Found {} relevant matches", relevant_match_ids.len());
    let all_matches = join_all(relevant_match_ids.into_iter().map(|id| {
        let api = &api;
        let all_users = &all_users;

        async move { process_match(api, &id, all_users).await }
    }))
    .await;

    let most_recent_match = all_matches
        .into_iter()
        .filter_map(|r| match r {
            Ok(Some(m)) => Some(m),
            _ => None,
        })
        .max_by_key(|m| m.time);

    Ok(most_recent_match)
}
