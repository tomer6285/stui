use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::process::Command;

use regex::Regex;

use crate::models::{Game, State};

pub fn get_config_dir() -> PathBuf {
    if let Some(config) = dirs::config_dir() {
        config.join("stui")
    } else if let Some(home) = dirs::home_dir() {
        home.join(".config").join("stui")
    } else {
        PathBuf::from(".config/stui")
    }
}

pub fn get_games_path() -> PathBuf {
    get_config_dir().join("games.json")
}

pub fn get_state_path() -> PathBuf {
    get_config_dir().join("state.json")
}

pub fn ensure_config_files() -> io::Result<()> {
    let config_dir = get_config_dir();
    if !config_dir.exists() {
        fs::create_dir_all(&config_dir)?;
    }

    let games_path = get_games_path();
    if !games_path.exists() {
        fs::write(&games_path, "[]")?;
    }

    Ok(())
}

pub fn load_games() -> Result<Vec<Game>, Box<dyn std::error::Error>> {
    let games_path = get_games_path();
    if !games_path.exists() {
        return Ok(Vec::new());
    }

    let data = fs::read_to_string(games_path)?;
    if data.trim().is_empty() {
        return Ok(Vec::new());
    }

    let games: Vec<Game> = serde_json::from_str(&data)?;
    Ok(games)
}

pub fn save_games(games: &[Game]) -> Result<(), Box<dyn std::error::Error>> {
    let config_dir = get_config_dir();
    if !config_dir.exists() {
        fs::create_dir_all(&config_dir)?;
    }

    let games_path = get_games_path();
    let data = serde_json::to_string_pretty(games)?;
    fs::write(games_path, data)?;
    Ok(())
}

pub fn load_state() -> State {
    let state_path = get_state_path();
    if !state_path.exists() {
        return State::default();
    }

    match fs::read_to_string(state_path) {
        Ok(data) => serde_json::from_str(&data).unwrap_or_default(),
        Err(_) => State::default(),
    }
}

pub fn save_state(last_selected_id: &str) {
    let config_dir = get_config_dir();
    if !config_dir.exists() {
        let _ = fs::create_dir_all(&config_dir);
    }

    let state_path = get_state_path();
    let state = State {
        last_selected_id: last_selected_id.to_string(),
    };

    if let Ok(data) = serde_json::to_string(&state) {
        let _ = fs::write(state_path, data);
    }
}

pub fn get_steam_steamapps_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".local/share/Steam/steamapps"))
}

pub fn get_steam_userdata_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".local/share/Steam/userdata"))
}

pub fn parse_playtimes_from_vdf(content: &str) -> HashMap<String, u32> {
    let mut playtimes = HashMap::new();
    let re = Regex::new(r#""(\d+)"\s+\{\s+[^}]*"Playtime"\s+"(\d+)""#).unwrap();

    for cap in re.captures_iter(content) {
        if cap.len() >= 3 {
            let app_id = cap[1].to_string();
            if let Ok(pt) = cap[2].parse::<u32>() {
                playtimes.insert(app_id, pt);
            }
        }
    }

    playtimes
}

pub fn parse_persona_state_from_vdf(content: &str) -> bool {
    let re = Regex::new(r#"ePersonaState\\?":\s*(\d+)"#).unwrap();
    if let Some(cap) = re.captures(content) {
        if let Some(m) = cap.get(1) {
            let state = m.as_str();
            if state == "7" || state == "0" {
                return false;
            }
        }
    }
    true
}

pub fn get_playtimes_map() -> Result<HashMap<String, u32>, Box<dyn std::error::Error>> {
    let userdata_dir = match get_steam_userdata_dir() {
        Some(dir) if dir.exists() => dir,
        _ => return Err("Steam userdata directory not found".into()),
    };

    let entries = fs::read_dir(userdata_dir)?;
    let mut user_dirs = Vec::new();
    for entry in entries.flatten() {
        if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
            user_dirs.push(entry.path());
        }
    }

    if user_dirs.is_empty() {
        return Err("No user directory found in Steam userdata".into());
    }

    let config_path = user_dirs[0].join("config").join("localconfig.vdf");
    if !config_path.exists() {
        return Err("localconfig.vdf not found".into());
    }

    let content = fs::read_to_string(config_path)?;
    Ok(parse_playtimes_from_vdf(&content))
}

pub fn get_steam_online_status() -> bool {
    let userdata_dir = match get_steam_userdata_dir() {
        Some(dir) if dir.exists() => dir,
        _ => return true,
    };

    let entries = match fs::read_dir(userdata_dir) {
        Ok(e) => e,
        Err(_) => return true,
    };

    let mut user_dirs = Vec::new();
    for entry in entries.flatten() {
        if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
            user_dirs.push(entry.path());
        }
    }

    if user_dirs.is_empty() {
        return true;
    }

    let config_path = user_dirs[0].join("config").join("localconfig.vdf");
    if !config_path.exists() {
        return true;
    }

    match fs::read_to_string(config_path) {
        Ok(content) => parse_persona_state_from_vdf(&content),
        Err(_) => true,
    }
}

pub fn parse_acf_file(content: &str) -> Option<(String, String)> {
    let re_id = Regex::new(r#""appid"\s+"(\d+)""#).ok()?;
    let re_name = Regex::new(r#""name"\s+"([^"]+)""#).ok()?;

    let id_match = re_id.captures(content)?;
    let name_match = re_name.captures(content)?;

    Some((id_match[1].to_string(), name_match[1].to_string()))
}

pub fn scan_steam_games() -> Result<Vec<Game>, Box<dyn std::error::Error>> {
    let steamapps_dir = match get_steam_steamapps_dir() {
        Some(dir) if dir.exists() => dir,
        _ => return Err("Steam steamapps directory not found".into()),
    };

    let playtimes = get_playtimes_map().unwrap_or_default();
    let mut games = Vec::new();

    for entry in fs::read_dir(steamapps_dir)?.flatten() {
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("acf") {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Some((id, name)) = parse_acf_file(&content) {
                    let playtime = playtimes.get(&id).copied().unwrap_or(0);
                    games.push(Game {
                        id,
                        name,
                        hidden: false,
                        playtime,
                        favorite: false,
                    });
                }
            }
        }
    }

    Ok(games)
}

pub fn refresh_playtimes(games: &mut [Game]) {
    if let Ok(playtimes) = get_playtimes_map() {
        for game in games.iter_mut() {
            if let Some(&pt) = playtimes.get(&game.id) {
                game.playtime = pt;
            }
        }
    }
}

pub fn merge_scanned_games(existing_games: &[Game], scanned_games: Vec<Game>) -> Vec<Game> {
    let existing_map: HashMap<String, &Game> =
        existing_games.iter().map(|g| (g.id.clone(), g)).collect();

    let mut updated_games = Vec::new();
    for mut game in scanned_games {
        if let Some(existing) = existing_map.get(&game.id) {
            game.hidden = existing.hidden;
            game.favorite = existing.favorite;
        }
        updated_games.push(game);
    }

    updated_games
}

pub fn sync_and_load_games() -> Vec<Game> {
    let _ = ensure_config_files();
    let mut games = load_games().unwrap_or_default();

    if let Ok(scanned) = scan_steam_games() {
        games = merge_scanned_games(&games, scanned);
        let _ = save_games(&games);
    }

    refresh_playtimes(&mut games);
    games.sort_by(|a, b| a.name.cmp(&b.name));
    games
}

pub fn launch_game(game_id: &str) -> io::Result<()> {
    Command::new("steam")
        .arg(format!("steam://rungameid/{}", game_id))
        .spawn()?;
    Ok(())
}

pub fn set_steam_status(online: bool) -> io::Result<()> {
    let status = if online { "online" } else { "offline" };
    Command::new("steam")
        .arg(format!("steam://friends/status/{}", status))
        .spawn()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_acf_file() {
        let sample_acf = r#"
"AppState"
{
	"appid"		"105600"
	"universe"		"1"
	"name"		"Terraria"
	"StateFlags"		"4"
}
"#;
        let result = parse_acf_file(sample_acf);
        assert_eq!(result, Some(("105600".to_string(), "Terraria".to_string())));
    }

    #[test]
    fn test_parse_playtimes_from_vdf() {
        let sample_vdf = r#"
"UserLocalConfigStore"
{
	"apps"
	{
		"218"
		{
			"LastPlayed"		"1722046689"
			"Playtime"		"166"
		}
		"400"
		{
			"LastPlayed"		"1744530322"
			"Playtime"		"274"
		}
	}
}
"#;
        let map = parse_playtimes_from_vdf(sample_vdf);
        assert_eq!(map.get("218"), Some(&166));
        assert_eq!(map.get("400"), Some(&274));
        assert_eq!(map.get("99999"), None);
    }

    #[test]
    fn test_parse_persona_state() {
        let online_vdf = r#"
"FriendStoreLocalPrefs_866787171"		"{\"ePersonaState\":1,\"strNonFriendsAllowedToMsg\":\"\"}"
"#;
        assert!(parse_persona_state_from_vdf(online_vdf));

        let offline_vdf = r#"
"FriendStoreLocalPrefs_866787171"		"{\"ePersonaState\":0,\"strNonFriendsAllowedToMsg\":\"\"}"
"#;
        assert!(!parse_persona_state_from_vdf(offline_vdf));

        let invisible_vdf = r#"
"FriendStoreLocalPrefs_866787171"		"{\"ePersonaState\":7,\"strNonFriendsAllowedToMsg\":\"\"}"
"#;
        assert!(!parse_persona_state_from_vdf(invisible_vdf));
    }

    #[test]
    fn test_merge_scanned_games() {
        let existing = vec![
            Game {
                id: "105600".into(),
                name: "Terraria".into(),
                hidden: true,
                playtime: 100,
                favorite: true,
            },
            Game {
                id: "620".into(),
                name: "Portal 2 (Uninstalled)".into(),
                hidden: false,
                playtime: 50,
                favorite: false,
            },
        ];

        let scanned = vec![
            Game {
                id: "105600".into(),
                name: "Terraria".into(),
                hidden: false,
                playtime: 100,
                favorite: false,
            },
            Game {
                id: "1113000".into(),
                name: "Persona 4 Golden".into(),
                hidden: false,
                playtime: 200,
                favorite: false,
            },
        ];

        let merged = merge_scanned_games(&existing, scanned);
        assert_eq!(merged.len(), 2);
        // Preserved favorite and hidden for Terraria
        assert_eq!(merged[0].id, "105600");
        assert!(merged[0].hidden);
        assert!(merged[0].favorite);

        // New game keeps default
        assert_eq!(merged[1].id, "1113000");
        assert!(!merged[1].hidden);
        assert!(!merged[1].favorite);
    }
}
