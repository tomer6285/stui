use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Game {
    #[serde(rename = "Name", alias = "name")]
    pub name: String,
    #[serde(rename = "ID", alias = "id")]
    pub id: String,
    #[serde(rename = "Hidden", alias = "hidden", default)]
    pub hidden: bool,
    #[serde(rename = "Playtime", alias = "playtime", default)]
    pub playtime: u32, // in minutes
    #[serde(rename = "Favorite", alias = "favorite", default)]
    pub favorite: bool,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq, Eq)]
pub struct State {
    #[serde(rename = "last_selected_id", default)]
    pub last_selected_id: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_game_serde_pascal_case() {
        let json_data = r#"{
            "Name": "Terraria",
            "ID": "105600",
            "Hidden": false,
            "Playtime": 120,
            "Favorite": true
        }"#;

        let game: Game = serde_json::from_str(json_data).unwrap();
        assert_eq!(game.name, "Terraria");
        assert_eq!(game.id, "105600");
        assert!(!game.hidden);
        assert_eq!(game.playtime, 120);
        assert!(game.favorite);

        let serialized = serde_json::to_string(&game).unwrap();
        assert!(serialized.contains(r#""Name":"Terraria""#));
        assert!(serialized.contains(r#""ID":"105600""#));
        assert!(serialized.contains(r#""Favorite":true"#));
    }

    #[test]
    fn test_game_serde_camel_case_alias() {
        let json_data = r#"{
            "name": "Portal 2",
            "id": "620",
            "hidden": true,
            "playtime": 45,
            "favorite": false
        }"#;

        let game: Game = serde_json::from_str(json_data).unwrap();
        assert_eq!(game.name, "Portal 2");
        assert_eq!(game.id, "620");
        assert!(game.hidden);
        assert_eq!(game.playtime, 45);
        assert!(!game.favorite);
    }

    #[test]
    fn test_state_serde() {
        let json_data = r#"{"last_selected_id":"105600"}"#;
        let state: State = serde_json::from_str(json_data).unwrap();
        assert_eq!(state.last_selected_id, "105600");

        let serialized = serde_json::to_string(&state).unwrap();
        assert_eq!(serialized, r#"{"last_selected_id":"105600"}"#);
    }
}
