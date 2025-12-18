// 日本語翻訳モジュール
use once_cell::sync::Lazy;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize)]
struct Translations {
    pokemon: HashMap<String, String>,
    moves: HashMap<String, String>,
    items: HashMap<String, String>,
    abilities: HashMap<String, String>,
    natures: HashMap<String, String>,
    types: HashMap<String, String>,
}

static TRANSLATIONS: Lazy<Translations> = Lazy::new(|| {
    let json_str = include_str!("../../translations/ja.json");
    serde_json::from_str(json_str).expect("Failed to parse translations/ja.json")
});

fn normalize_key(name: &str) -> String {
    name.to_ascii_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect()
}

pub fn translate_pokemon(name: &str) -> String {
    let key = normalize_key(name);
    TRANSLATIONS
        .pokemon
        .get(&key)
        .cloned()
        .unwrap_or_else(|| name.to_string())
}

pub fn translate_move(name: &str) -> String {
    let key = normalize_key(name);
    TRANSLATIONS
        .moves
        .get(&key)
        .cloned()
        .unwrap_or_else(|| name.to_string())
}

pub fn translate_item(name: &str) -> String {
    let key = normalize_key(name);
    TRANSLATIONS
        .items
        .get(&key)
        .cloned()
        .unwrap_or_else(|| name.to_string())
}

pub fn translate_ability(name: &str) -> String {
    let key = normalize_key(name);
    TRANSLATIONS
        .abilities
        .get(&key)
        .cloned()
        .unwrap_or_else(|| name.to_string())
}

pub fn translate_nature(name: &str) -> String {
    let key = normalize_key(name);
    TRANSLATIONS
        .natures
        .get(&key)
        .cloned()
        .unwrap_or_else(|| name.to_string())
}

pub fn translate_type(name: &str) -> String {
    let key = normalize_key(name);
    TRANSLATIONS
        .types
        .get(&key)
        .cloned()
        .unwrap_or_else(|| name.to_string())
}
