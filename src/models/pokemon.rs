//! Models used to create/update/load pokemons from the database.
//!
//! # Notes
//!
//! This file includes a lot of repetition. I wish there was an easy way to implement this type of
//! structs more easily. I tried with the help of some crates like [`boilermates`](https://crates.io/crates/boilermates)
//! and [`optfield`](https://crates.io/crates/optfield) and _almost_ succeeded, but some things were missing.

pub mod macros;
pub mod validations;

use diesel_derives::{AsChangeset, Insertable, Queryable, Selectable};
use serde::{Deserialize, Serialize};
use utoipa::{ToResponse, ToSchema};
use validations::validate_pokemon_type;
use validator::Validate;

use crate::schema::pokemons;
use crate::{implement_pokemon_upsert, implement_pokemon_upsert_from};

#[cfg_attr(
    doc,
    doc = r"
        Base pokemon entity model.

        Used to validate queries at compile time as well as load pokemons from the database
        (including those returned by update queries).
    "
)]
#[cfg_attr(not(doc), doc = "Information about a Pokemon in the Pokedex")]
#[derive(
    Debug, Clone, PartialEq, Eq, Queryable, Selectable, Serialize, Deserialize, ToSchema, ToResponse,
)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[serde(deny_unknown_fields)]
#[response(
    description = "Pokemon information",
    example = json!({
        "id": 0,
        "number": 1,
        "name": "Bulbasaur",
        "type_1": "Grass",
        "type_2": "Poison",
        "total": 318,
        "hp": 45,
        "attack": 49,
        "defense": 49,
        "sp_atk": 65,
        "sp_def": 65,
        "speed": 45,
        "generation": 1,
        "legendary": false
    }),
)]
pub struct Pokemon {
    /// Unique id of this Pokemon in the Pokedex database
    pub id: i64,

    /// Pokemon number, as specified in Pokedex; different than the id
    ///
    /// Non-unique: all variants of the same pokemon share the same number
    pub number: i32,

    /// Pokemon name
    pub name: String,

    /// Pokemon first type
    #[schema(example = "Grass")]
    pub type_1: String,

    /// Pokemon second type (if it has one)
    #[serde(default)]
    #[schema(example = "Fire")]
    pub type_2: Option<String>,

    /// Total of all Pokemon's stats
    pub total: i32,

    /// Pokemon's hit points
    pub hp: i32,

    /// Pokemon's attack stat
    pub attack: i32,

    /// Pokemon's defense stat
    pub defense: i32,

    /// Pokemon's special attack stat
    pub sp_atk: i32,

    /// Pokemon's special defense stat
    pub sp_def: i32,

    /// Pokemon's speed stat
    pub speed: i32,

    /// Pokemon's generation number
    pub generation: i32,

    /// Whether Pokemon is legendary
    pub legendary: bool,
}

implement_pokemon_upsert! {
    pub struct CreatePokemon(
        doc = "Model used to insert a new pokemon in the database.",
        openapi_doc = "Information to create a new Pokemon in the Pokedex"
    );
}
implement_pokemon_upsert! {
    pub struct UpdatePokemon(
        doc = "Model used to update a pokemon in the database.",
        openapi_doc = "Information to update a Pokemon in the Pokedex, overwriting all fields"
    );
}
implement_pokemon_upsert_from!(CreatePokemon, UpdatePokemon);

#[cfg_attr(
    doc,
    doc = r#"
        Model used to "patch" a pokemon in the database, e.g. update some fields only.

        All fields are optional; fields that are not specified will not be updated.
    "#
)]
#[cfg_attr(not(doc), doc = "Information to update specific fields of a Pokemon in the Pokedex")]
#[derive(Debug, Clone, PartialEq, Eq, AsChangeset, Serialize, Deserialize, Validate, ToSchema)]
#[diesel(table_name = pokemons)]
#[serde(deny_unknown_fields)]
#[schema(example = json!({
    "name": "Bulbasaur",
    "type_2": "Poison"
}))]
pub struct PatchPokemon {
    /// Pokemon number, as specified in Pokedex
    ///
    /// Non-unique: all variants of the same pokemon share the same number
    #[validate(range(min = 1))]
    pub number: Option<i32>,

    /// Pokemon name
    #[validate(length(min = 1))]
    pub name: Option<String>,

    /// Pokemon first type
    #[validate(custom = "validate_pokemon_type")]
    #[schema(example = "Grass")]
    pub type_1: Option<String>,

    /// Pokemon second type (if it has one)
    #[serde(
        with = "serde_with::rust::double_option",
        skip_serializing_if = "Option::is_none",
        default
    )]
    #[validate(custom = "validate_pokemon_type")]
    #[schema(nullable, example = "Fire")]
    pub type_2: Option<Option<String>>,

    /// Total of all pokemon's stats
    pub total: Option<i32>,

    /// Pokemon's hit points
    #[validate(range(min = 1))]
    pub hp: Option<i32>,

    /// Pokemon's attack stat
    pub attack: Option<i32>,

    /// Pokemon's defense stat
    pub defense: Option<i32>,

    /// Pokemon's special attack stat
    pub sp_atk: Option<i32>,

    /// Pokemon's special defense stat
    pub sp_def: Option<i32>,

    /// Pokemon's speed stat
    pub speed: Option<i32>,

    /// Pokemon's generation number
    #[validate(range(min = 1))]
    pub generation: Option<i32>,

    /// Whether pokemon is legendary
    pub legendary: Option<bool>,
}

/// Model used to import pokemons in the database from the seed CSV file.
///
/// Used by the `seed_db` command to seed the database initially.
#[derive(Debug, Clone, Insertable, Deserialize, Validate)]
#[diesel(table_name = pokemons)]
#[serde(rename_all = "PascalCase")]
#[allow(missing_docs)]
pub struct ImportPokemon {
    #[serde(rename = "#")]
    #[validate(range(min = 1))]
    pub number: i32,
    #[validate(length(min = 1))]
    pub name: String,
    #[serde(rename = "Type 1")]
    #[validate(custom = "validate_pokemon_type")]
    pub type_1: String,
    #[serde(rename = "Type 2")]
    #[validate(custom = "validate_pokemon_type")]
    pub type_2: Option<String>,
    pub total: i32,
    #[serde(rename = "HP")]
    #[validate(range(min = 1))]
    pub hp: i32,
    pub attack: i32,
    pub defense: i32,
    #[serde(rename = "Sp. Atk")]
    pub sp_atk: i32,
    #[serde(rename = "Sp. Def")]
    pub sp_def: i32,
    pub speed: i32,
    #[validate(range(min = 1))]
    pub generation: i32,
    // `legendary` is specified as a Python-style bool in the CSV file (e.g., `True`/`False`),
    // so we use a custom deserializer for this.
    #[serde(deserialize_with = "serde_this_or_that::as_bool")]
    pub legendary: bool,
}

//noinspection DuplicatedCode
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_pokemon_for_create_pokemon() {
        let pokemon = Pokemon {
            id: 0,
            number: 1,
            name: "Bulbasaur".into(),
            type_1: "Grass".into(),
            type_2: Some("Poison".into()),
            total: 318,
            hp: 45,
            attack: 49,
            defense: 49,
            sp_atk: 65,
            sp_def: 65,
            speed: 45,
            generation: 1,
            legendary: false,
        };

        let expected_create_pokemon = CreatePokemon {
            number: 1,
            name: "Bulbasaur".into(),
            type_1: "Grass".into(),
            type_2: Some("Poison".into()),
            total: 318,
            hp: 45,
            attack: 49,
            defense: 49,
            sp_atk: 65,
            sp_def: 65,
            speed: 45,
            generation: 1,
            legendary: false,
        };
        let actual_create_pokemon: CreatePokemon = pokemon.into();
        assert_eq!(actual_create_pokemon, expected_create_pokemon);
    }

    #[test]
    fn test_from_pokemon_for_update_pokemon() {
        let pokemon = Pokemon {
            id: 0,
            number: 1,
            name: "Bulbasaur".into(),
            type_1: "Grass".into(),
            type_2: Some("Poison".into()),
            total: 318,
            hp: 45,
            attack: 49,
            defense: 49,
            sp_atk: 65,
            sp_def: 65,
            speed: 45,
            generation: 1,
            legendary: false,
        };

        let expected_update_pokemon = UpdatePokemon {
            number: 1,
            name: "Bulbasaur".into(),
            type_1: "Grass".into(),
            type_2: Some("Poison".into()),
            total: 318,
            hp: 45,
            attack: 49,
            defense: 49,
            sp_atk: 65,
            sp_def: 65,
            speed: 45,
            generation: 1,
            legendary: false,
        };
        let actual_update_pokemon: UpdatePokemon = pokemon.into();
        assert_eq!(actual_update_pokemon, expected_update_pokemon);
    }
}
