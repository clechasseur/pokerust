//! Helpers to validate Pokemon data.

use std::borrow::Cow;

use validator::ValidationError;

/// The valid Pokemon types.
///
/// Can be used to validate the `type_1` and `type_2` fields of a Pokemon struct.
///
/// # Notes
///
/// The list of valid types has been picked from [this site](https://www.toynk.com/blogs/news/how-many-pokemon-types-are-there).
pub const POKEMON_TYPES: [&str; 18] = [
    "Normal", "Fire", "Water", "Grass", "Flying", "Fighting", "Poison", "Electric", "Ground",
    "Rock", "Psychic", "Ice", "Bug", "Ghost", "Steel", "Dragon", "Dark", "Fairy",
];

/// Validates a Pokemon type value.
///
/// A type value is only considered valid if it appears in [`POKEMON_TYPES`]. The type values
/// are case-sensitive.
pub fn validate_pokemon_type(type_value: &str) -> Result<(), ValidationError> {
    if POKEMON_TYPES.contains(&type_value) {
        Ok(())
    } else {
        let error_message = format!(
            "type field must match one of {} or {}",
            POKEMON_TYPES[..POKEMON_TYPES.len() - 1].join(", "),
            POKEMON_TYPES.last().cloned().unwrap(),
        );

        let mut validation_error = ValidationError::new("invalid_type");
        validation_error.message = Some(Cow::from(error_message));

        Err(validation_error)
    }
}

#[cfg(test)]
mod tests {
    use validator::Validate;

    use super::*;

    #[derive(Debug, Validate)]
    struct TestPokemon {
        #[validate(custom = "validate_pokemon_type")]
        pub type_1: String,
        #[validate(custom = "validate_pokemon_type")]
        pub type_2: Option<String>,
    }

    mod validate_pokemon_type {
        use validator::ValidationErrors;

        use super::*;

        #[test]
        fn test_valid_type() {
            let pokemon = TestPokemon { type_1: "Grass".into(), type_2: Some("Poison".into()) };

            let validation_result = pokemon.validate();
            assert!(validation_result.is_ok());
        }

        #[test]
        fn test_invalid_type() {
            let pokemon = TestPokemon { type_1: "Love".into(), type_2: Some("Patience".into()) };

            let validation_result = pokemon.validate();
            assert!(validation_result.is_err());
            assert!(ValidationErrors::has_error(&validation_result, "type_1"));
            assert!(ValidationErrors::has_error(&validation_result, "type_2"));
        }

        #[test]
        fn test_null() {
            let pokemon = TestPokemon { type_1: "Dark".into(), type_2: None };

            let validation_result = pokemon.validate();
            assert!(validation_result.is_ok());
        }
    }
}
