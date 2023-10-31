// @generated automatically by Diesel CLI.

diesel::table! {
    pokemons (id) {
        id -> Int8,
        number -> Int4,
        name -> Text,
        type_1 -> Text,
        type_2 -> Nullable<Text>,
        total -> Int4,
        hp -> Int4,
        attack -> Int4,
        defense -> Int4,
        sp_atk -> Int4,
        sp_def -> Int4,
        speed -> Int4,
        generation -> Int4,
        legendary -> Bool,
    }
}
