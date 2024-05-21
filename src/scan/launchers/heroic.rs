pub mod gog;
pub mod legendary;
pub mod nile;
pub mod sideload;

use std::collections::{HashMap, HashSet};

use crate::prelude::StrictPath;

use crate::{
    resource::{config::RootsConfig, manifest::Os},
    scan::{launchers::LauncherGame, TitleFinder},
};

mod games_config {
    use std::collections::HashMap;

    pub fn path(id: &str) -> String {
        format!("GamesConfig/{id}.json")
    }

    #[derive(serde::Deserialize, Debug)]
    pub struct Data(pub HashMap<String, Game>);

    #[derive(serde::Deserialize, Debug)]
    #[serde(untagged)]
    pub enum Game {
        #[serde(rename_all = "camelCase")]
        Config {
            wine_prefix: String,
            wine_version: Wine,
        },
        IgnoreOther(serde::de::IgnoredAny),
    }

    #[derive(serde::Deserialize, Debug)]
    pub struct Wine {
        #[serde(rename = "type")]
        pub wine_type: String,
    }
}

pub fn scan(
    root: &RootsConfig,
    title_finder: &TitleFinder,
    legendary: Option<&StrictPath>,
) -> HashMap<String, HashSet<LauncherGame>> {
    let mut games = HashMap::<String, HashSet<LauncherGame>>::new();

    for (title, info) in legendary::scan(root, title_finder, legendary) {
        games.entry(title).or_default().extend(info);
    }

    for (title, info) in gog::scan(root, title_finder) {
        games.entry(title).or_default().extend(info);
    }

    for (title, info) in nile::scan(root, title_finder) {
        games.entry(title).or_default().extend(info);
    }

    for (title, info) in sideload::scan(root, title_finder) {
        games.entry(title).or_default().extend(info);
    }

    games
}

fn find_prefix(
    heroic_path: &StrictPath,
    game_name: &str,
    platform: Option<&str>,
    app_name: &str,
) -> Option<StrictPath> {
    let platform = platform?;

    match Os::from(platform) {
        Os::Windows => {
            log::trace!(
                "Will try to find prefix for Heroic Windows game: {} ({})",
                game_name,
                app_name
            );

            let games_config_path = heroic_path.joined(&games_config::path(app_name));
            match serde_json::from_str::<games_config::Data>(&games_config_path.read().unwrap_or_default()) {
                Ok(games_config_wrapper) => {
                    let game_config = games_config_wrapper.0.get(app_name)?;

                    match game_config {
                        games_config::Game::Config {
                            wine_version,
                            wine_prefix,
                        } => match wine_version.wine_type.as_str() {
                            "wine" => {
                                log::trace!(
                                    "Found Heroic Wine prefix for {} ({}) -> adding {}",
                                    game_name,
                                    app_name,
                                    wine_prefix
                                );
                                Some(StrictPath::new(wine_prefix.clone()))
                            }

                            "proton" => {
                                let prefix = format!("{}/pfx", wine_prefix);
                                log::trace!(
                                    "Found Heroic Proton prefix for {} ({}), adding {}",
                                    game_name,
                                    app_name,
                                    &prefix
                                );
                                Some(StrictPath::new(prefix))
                            }

                            _ => {
                                log::info!(
                                    "Found Heroic Windows game {} ({}) with unknown wine_type: {:#?}",
                                    game_name,
                                    app_name,
                                    wine_version.wine_type
                                );
                                None
                            }
                        },
                        games_config::Game::IgnoreOther(_) => None,
                    }
                }
                Err(e) => {
                    log::trace!("Failed to read {:?}: {}", &games_config_path, e);
                    None
                }
            }
        }

        Os::Linux => {
            log::trace!("Found Heroic Linux game {}, ignoring prefix", game_name);
            None
        }

        Os::Mac => {
            log::trace!("Found Heroic Mac game {}, ignoring prefix", game_name);
            None
        }

        _ => {
            log::trace!(
                "Found Heroic game {} with unhandled platform {}, ignoring prefix",
                game_name,
                platform,
            );
            None
        }
    }
}
