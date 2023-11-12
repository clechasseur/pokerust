use pokedex_rs::models::pokemon::{CreatePokemon, PatchPokemon, UpdatePokemon};
use validator::Validate;

pub fn build_create_pokemon() -> CreatePokemon {
    build_create_pokemons(1).remove(0)
}

pub fn build_create_pokemons(count: usize) -> Vec<CreatePokemon> {
    (1..=count)
        .map(|number| CreatePokemon {
            number: number as i32,
            name: format!("Pikafoo_{}", number),
            type_1: "Grass".into(),
            type_2: Some("Electric".into()),
            total: 640,
            hp: 66,
            attack: 7,
            defense: 11,
            sp_atk: 23,
            sp_def: 67,
            speed: 3,
            generation: 1,
            legendary: false,
        })
        .inspect(|pokemon| pokemon.validate().unwrap())
        .collect()
}

pub fn build_update_pokemon(orig_pokemon: &CreatePokemon) -> UpdatePokemon {
    let mut update_pokemon: UpdatePokemon = orig_pokemon.clone().into();
    update_pokemon.name.push_str("_updated");
    update_pokemon.type_2 = None;

    update_pokemon.validate().unwrap();
    update_pokemon
}

pub fn build_patch_pokemon(
    orig_pokemon: &CreatePokemon,
    patched_type_2: Option<Option<String>>,
) -> PatchPokemon {
    let patch_pokemon = PatchPokemon {
        number: None,
        name: Some(format!("{}_patched", orig_pokemon.name)),
        type_1: None,
        type_2: patched_type_2,
        total: None,
        hp: None,
        attack: None,
        defense: None,
        sp_atk: None,
        sp_def: None,
        speed: None,
        generation: None,
        legendary: None,
    };

    patch_pokemon.validate().unwrap();
    patch_pokemon
}
