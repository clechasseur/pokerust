//! Helper macros used to generate Pokemon-related `struct`s.

/// Macro to generate a struct used to insert or update a Pokemon in the database.
///
/// # Examples
///
/// ```ignore
/// use pokedex_rs::implement_pokemon_upsert;
///
/// implement_pokemon_upsert! {
///     pub struct CreatePokemon(
///         doc = "Model used to insert a new pokemon.",
///         openapi_doc = "Information to create a Pokemon"
///     );
/// }
/// ```
#[macro_export]
macro_rules! implement_pokemon_upsert {
    (
        $(#[$attr:meta])*
        $vis:vis struct $name:ident(
            doc = $doc:expr,
            openapi_doc = $openapi_doc:expr
        );
    ) => {
        paste::paste! {
            $(#[$attr])*
            #[cfg_attr(doc, doc = r"
                " $doc r"

                All fields must be specified except for [`type_2`](" $name r"::type_2), which is
                nullable (if not specified, `NULL` will be inserted).
            ")]
            #[cfg_attr(not(doc), doc = $openapi_doc)]
            #[derive(
                std::fmt::Debug,
                std::clone::Clone,
                std::cmp::PartialEq,
                std::cmp::Eq,
                diesel_derives::Insertable,
                diesel_derives::AsChangeset,
                serde::Serialize,
                serde::Deserialize,
                validator::Validate,
                utoipa::ToSchema,
            )]
            #[diesel(
                table_name = $crate::schema::pokemons,
                treat_none_as_null = true,
            )]
            #[serde(deny_unknown_fields)]
            $vis struct $name {
                /// Pokemon number, as specified in Pokedex
                ///
                /// Non-unique: all variants of the same pokemon share the same number
                #[validate(range(min = 1))]
                #[schema(example = 1)]
                pub number: i32,

                /// Pokemon name
                #[validate(length(min = 1))]
                #[schema(example = "Bulbasaur")]
                pub name: String,

                /// Pokemon first type
                #[validate(custom = "crate::models::pokemon::validations::validate_pokemon_type")]
                #[schema(example = "Grass")]
                pub type_1: String,

                /// Pokemon second type (if it has one)
                #[serde(default)]
                #[validate(custom = "crate::models::pokemon::validations::validate_pokemon_type")]
                #[schema(nullable, example = "Poison")]
                pub type_2: Option<String>,

                /// Total of all pokemon's stats
                #[schema(example = 318)]
                pub total: i32,

                /// Pokemon's hit points
                #[validate(range(min = 1))]
                #[schema(example = 45)]
                pub hp: i32,

                /// Pokemon's attack stat
                #[schema(example = 49)]
                pub attack: i32,

                /// Pokemon's defense stat
                #[schema(example = 49)]
                pub defense: i32,

                /// Pokemon's special attack stat
                #[schema(example = 65)]
                pub sp_atk: i32,

                /// Pokemon's special defense stat
                #[schema(example = 65)]
                pub sp_def: i32,

                /// Pokemon's speed stat
                #[schema(example = 15)]
                pub speed: i32,

                /// Pokemon's generation number
                #[validate(range(min = 1))]
                #[schema(example = 1)]
                pub generation: i32,

                /// Whether pokemon is legendary
                #[schema(example = false)]
                pub legendary: bool,
            }

            $crate::implement_pokemon_upsert_from! {
                #[doc = r"
                    Converts a [`Pokemon`]($crate::models::pokemon::Pokemon) struct into a
                    [`" $name r"`], dropping its [`id`]($crate::models::pokemon::Pokemon::id).
                "]
                $crate::models::pokemon::Pokemon => $name
            }
        }
    }
}

/// Macro to generate [`From`] implementations for insert/update Pokemon structs.
///
/// Will generate two `impl From`s:
///
/// * `impl From<CreateStruct> for UpdateStruct`
/// * `impl From<UpdateStruct> for CreateStruct`
///
/// # Examples
///
/// ```ignore
/// use pokedex_rs::{implement_pokemon_upsert, implement_pokemon_upsert_from};
///
/// implement_pokemon_upsert! {
///     pub struct CreatePokemon(
///         doc = "Model used to insert a new pokemon.",
///         openapi_doc = "Information to create a Pokemon"
///     );
/// }
/// implement_pokemon_upsert! {
///     pub struct UpdatePokemon(
///         doc = "Model used to update a pokemon.",
///         openapi_doc = "Information to update a Pokemon"
///     );
/// }
/// implement_pokemon_upsert_from!(CreatePokemon, UpdatePokemon);
/// ```
#[macro_export]
macro_rules! implement_pokemon_upsert_from {
    ( $create_ty:ty, $update_ty:ty ) => {
        $crate::implement_pokemon_upsert_from! { $create_ty => $update_ty }
        $crate::implement_pokemon_upsert_from! { $update_ty => $create_ty }
    };

    (
        $(#[$attr:meta])*
        $create_ty:ty => $update_ty:ty
    ) => {
        impl std::convert::From<$create_ty> for $update_ty {
            $(#[$attr])*
            fn from(value: $create_ty) -> Self {
                Self {
                    number: value.number,
                    name: value.name,
                    type_1: value.type_1,
                    type_2: value.type_2,
                    total: value.total,
                    hp: value.hp,
                    attack: value.attack,
                    defense: value.defense,
                    sp_atk: value.sp_atk,
                    sp_def: value.sp_def,
                    speed: value.speed,
                    generation: value.generation,
                    legendary: value.legendary,
                }
            }
        }
    }
}

//noinspection DuplicatedCode
#[cfg(test)]
mod tests {
    use crate::models::pokemon::Pokemon;
    implement_pokemon_upsert! {
        struct TestCreatePokemon(
            doc = "TestCreatePokemon doc",
            openapi_doc = "TestCreatePokemon openapi doc"
        );
    }
    implement_pokemon_upsert! {
        struct TestUpdatePokemon(
            doc = "TestUpdatePokemon doc",
            openapi_doc = "TestUpdatePokemon openapi doc"
        );
    }

    #[test]
    fn test_pokemon_to_create_pokemon() {
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

        let expected_create_pokemon = TestCreatePokemon {
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
        let actual_create_pokemon: TestCreatePokemon = pokemon.into();
        assert_eq!(actual_create_pokemon, expected_create_pokemon);
    }

    #[test]
    fn test_pokemon_to_update_pokemon() {
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

        let expected_update_pokemon = TestUpdatePokemon {
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
        let actual_update_pokemon: TestUpdatePokemon = pokemon.into();
        assert_eq!(actual_update_pokemon, expected_update_pokemon);
    }

    mod implement_pokemon_upsert_from {
        use super::*;

        implement_pokemon_upsert_from!(TestCreatePokemon, TestUpdatePokemon);

        #[test]
        fn test_create_pokemon_to_update_pokemon() {
            let create_pokemon = TestCreatePokemon {
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

            let expected_update_pokemon = TestUpdatePokemon {
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
            let actual_update_pokemon: TestUpdatePokemon = create_pokemon.into();
            assert_eq!(actual_update_pokemon, expected_update_pokemon);
        }

        #[test]
        fn test_update_pokemon_to_create_pokemon() {
            let update_pokemon = TestUpdatePokemon {
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

            let expected_create_pokemon = TestCreatePokemon {
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
            let actual_create_pokemon: TestCreatePokemon = update_pokemon.into();
            assert_eq!(actual_create_pokemon, expected_create_pokemon);
        }
    }
}
