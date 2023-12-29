mod list {
    use actix_web::http::StatusCode;
    use actix_web::test;
    use diesel::insert_into;
    use diesel::result::Error as DieselError;
    use diesel_async::RunQueryDsl;
    use pokedex_rs::helpers::db::paginate::{reset_mock_error_producer, set_mock_error_producer};
    use pokedex_rs::services::pokemon::PokemonsPage;
    use serial_test::file_serial;

    use crate::init_test_service;
    use crate::integration_helpers::factories::pokemon::build_create_pokemons;

    #[test_log::test(actix_web::test)]
    #[file_serial(api_v1_pokemons)]
    async fn test_empty_list() {
        init_test_service!(app, service);

        let req = test::TestRequest::with_uri("/api/v1/pokemons").to_request();
        let page: PokemonsPage = test::call_and_read_body_json(&service, req).await;

        assert!(page.pokemons.is_empty());
        assert_eq!(1, page.page);
        assert_eq!(0, page.total_pages);
    }

    #[test_log::test(actix_web::test)]
    #[file_serial(api_v1_pokemons)]
    async fn test_paginated_list() {
        use pokedex_rs::schema::pokemons::dsl::*;

        init_test_service!(app, service);

        {
            let new_pokemons = build_create_pokemons(10);
            let mut connection = app.get_pooled_connection().await;
            let inserted_count = insert_into(pokemons)
                .values(&new_pokemons)
                .execute(&mut connection)
                .await
                .unwrap();
            assert_eq!(10, inserted_count);
        }

        for page_number in 1i64..=2 {
            let req = test::TestRequest::with_uri(&format!(
                "/api/v1/pokemons?page={}&page_size={}",
                page_number, 5
            ))
            .to_request();
            let page: PokemonsPage = test::call_and_read_body_json(&service, req).await;

            assert_eq!(5, page.pokemons.len());
            for expected_number in 1i32..=5 {
                let pokemon = &page.pokemons[(expected_number - 1) as usize];
                assert_eq!(expected_number + ((page_number - 1) as i32 * 5), pokemon.number);
            }
            assert_eq!(page_number, page.page);
            assert_eq!(5, page.page_size);
            assert_eq!(2, page.total_pages);
        }

        let req = test::TestRequest::with_uri("/api/v1/pokemons?page=3&page_size=5").to_request();
        let page: PokemonsPage = test::call_and_read_body_json(&service, req).await;

        assert!(page.pokemons.is_empty());
        assert_eq!(3, page.page);
        assert_eq!(5, page.page_size);
        assert_eq!(2, page.total_pages);
    }

    #[test_log::test(actix_web::test)]
    #[file_serial(api_v1_pokemons)]
    async fn test_invalid_query_params() {
        init_test_service!(app, service);

        let req = test::TestRequest::with_uri("/api/v1/pokemons?foo=bar").to_request();
        let result = test::call_service(&service, req).await;

        assert_eq!(StatusCode::BAD_REQUEST, result.status());
    }

    #[test_log::test(actix_web::test)]
    #[file_serial(api_v1_pokemons)]
    async fn test_invalid_query_param_values() {
        init_test_service!(app, service);

        let req =
            test::TestRequest::with_uri("/api/v1/pokemons?page=foo&page_size=bar").to_request();
        let result = test::call_service(&service, req).await;

        assert_eq!(StatusCode::BAD_REQUEST, result.status());
    }

    #[test_log::test(actix_web::test)]
    #[file_serial(api_v1_pokemons)]
    async fn test_invalid_query_param_validation() {
        init_test_service!(app, service);

        let req = test::TestRequest::with_uri("/api/v1/pokemons?page=0&page_size=0").to_request();
        let result = test::call_service(&service, req).await;

        assert_eq!(StatusCode::BAD_REQUEST, result.status());
    }

    #[test_log::test(actix_web::test)]
    #[file_serial(api_v1_pokemons)]
    async fn test_broken_db_connection() {
        init_test_service!(app, service);

        let result = {
            set_mock_error_producer(Box::new(|| Some(DieselError::BrokenTransactionManager)));

            let req = test::TestRequest::with_uri("/api/v1/pokemons").to_request();
            let result = test::call_service(&service, req).await;

            reset_mock_error_producer();

            result
        };

        assert_eq!(StatusCode::INTERNAL_SERVER_ERROR, result.status());
    }

    #[test_log::test(actix_web::test)]
    #[file_serial(api_v1_pokemons)]
    async fn test_working_db_connection() {
        init_test_service!(app, service);

        let result = {
            set_mock_error_producer(Box::new(|| None));

            let req = test::TestRequest::with_uri("/api/v1/pokemons").to_request();
            let result = test::call_service(&service, req).await;

            reset_mock_error_producer();

            result
        };

        assert!(result.status().is_success());
    }
}

mod get {
    use actix_web::http::StatusCode;
    use actix_web::test;
    use diesel::insert_into;
    use diesel_async::RunQueryDsl;
    use pokedex_rs::models::pokemon::Pokemon;
    use serial_test::file_serial;

    use crate::init_test_service;
    use crate::integration_helpers::factories::pokemon::build_create_pokemon;

    #[test_log::test(actix_web::test)]
    #[file_serial(api_v1_pokemons)]
    async fn test_exists() {
        use pokedex_rs::schema::pokemons::dsl::*;

        init_test_service!(app, service);

        let new_pokemon = build_create_pokemon();
        let new_pokemon_id: i64;
        {
            let mut connection = app.get_pooled_connection().await;
            new_pokemon_id = insert_into(pokemons)
                .values(&new_pokemon)
                .returning(id)
                .get_result(&mut connection)
                .await
                .unwrap();
        }

        let req = test::TestRequest::with_uri(&format!("/api/v1/pokemons/{}", new_pokemon_id))
            .to_request();
        let api_pokemon: Pokemon = test::call_and_read_body_json(&service, req).await;

        assert_eq!(new_pokemon_id, api_pokemon.id);
        assert_eq!(new_pokemon, api_pokemon.into());
    }

    #[test_log::test(actix_web::test)]
    #[file_serial(api_v1_pokemons)]
    async fn test_does_not_exist() {
        init_test_service!(app, service);

        let pokemon_id = i64::MAX;
        let req =
            test::TestRequest::with_uri(&format!("/api/v1/pokemons/{}", pokemon_id)).to_request();
        let result = test::call_service(&service, req).await;

        assert_eq!(StatusCode::NOT_FOUND, result.status());
    }

    #[test_log::test(actix_web::test)]
    #[file_serial(api_v1_pokemons)]
    async fn test_invalid_path_param() {
        init_test_service!(app, service);

        let req = test::TestRequest::with_uri("/api/v1/pokemons/foobar").to_request();
        let result = test::call_service(&service, req).await;

        assert_eq!(StatusCode::BAD_REQUEST, result.status());
    }

    #[test_log::test(actix_web::test)]
    #[file_serial(api_v1_pokemons)]
    async fn test_invalid_path_param_validation() {
        init_test_service!(app, service);

        let req = test::TestRequest::with_uri("/api/v1/pokemons/-1").to_request();
        let result = test::call_service(&service, req).await;

        assert_eq!(StatusCode::BAD_REQUEST, result.status());
    }
}

mod create {
    use actix_web::http::StatusCode;
    use actix_web::test;
    use assert_matches::assert_matches;
    use diesel::QueryDsl;
    use diesel_async::RunQueryDsl;
    use pokedex_rs::models::pokemon::Pokemon;
    use serde_json::json;
    use serial_test::file_serial;

    use crate::init_test_service;
    use crate::integration_helpers::factories::pokemon::build_create_pokemon;

    #[test_log::test(actix_web::test)]
    #[file_serial(api_v1_pokemons)]
    async fn test_create_pokemon() {
        use pokedex_rs::schema::pokemons::dsl::*;

        init_test_service!(app, service);

        let new_pokemon = build_create_pokemon();

        let req = test::TestRequest::post()
            .uri("/api/v1/pokemons")
            .set_json(new_pokemon)
            .to_request();
        let api_pokemon: Pokemon = test::call_and_read_body_json(&service, req).await;

        assert!(api_pokemon.name.starts_with("Pikafoo"));
        assert_eq!("Grass", api_pokemon.type_1);
        assert_matches!(api_pokemon.type_2, Some(ref value) if value == "Electric");

        let mut connection = app.get_pooled_connection().await;
        let db_pokemon: Pokemon = pokemons
            .find(api_pokemon.id)
            .first(&mut connection)
            .await
            .unwrap();

        assert_eq!(db_pokemon, api_pokemon);
    }

    #[test_log::test(actix_web::test)]
    #[file_serial(api_v1_pokemons)]
    async fn test_invalid_payload() {
        init_test_service!(app, service);

        let invalid_payload = json!({
            "foo": "bar"
        });

        let req = test::TestRequest::post()
            .uri("/api/v1/pokemons")
            .set_json(invalid_payload)
            .to_request();
        let result = test::call_service(&service, req).await;

        assert_eq!(StatusCode::BAD_REQUEST, result.status());
    }

    #[test_log::test(actix_web::test)]
    #[file_serial(api_v1_pokemons)]
    async fn test_invalid_payload_values() {
        init_test_service!(app, service);

        let invalid_payload = json!({
            "number": "foobar",
            "name": "foobar",
            "type_1": "foobar",
            "type_2": "foobar",
            "total": "foobar",
            "hp": "foobar",
            "attack": "foobar",
            "defense": "foobar",
            "sp_atk": "foobar",
            "sp_def": "foobar",
            "speed": "foobar",
            "generation": "foobar",
            "legendary": "foobar"
        });

        let req = test::TestRequest::post()
            .uri("/api/v1/pokemons")
            .set_json(invalid_payload)
            .to_request();
        let result = test::call_service(&service, req).await;

        assert_eq!(StatusCode::BAD_REQUEST, result.status());
    }

    #[test_log::test(actix_web::test)]
    #[file_serial(api_v1_pokemons)]
    async fn test_invalid_payload_values_validation() {
        init_test_service!(app, service);

        let invalid_payload = json!({
            "number": 0,
            "name": "",
            "type_1": "Love",
            "type_2": "Patience",
            "total": 0,
            "hp": 0,
            "attack": 0,
            "defense": 0,
            "sp_atk": 0,
            "sp_def": 0,
            "speed": 0,
            "generation": 0,
            "legendary": false
        });

        let req = test::TestRequest::post()
            .uri("/api/v1/pokemons")
            .set_json(invalid_payload)
            .to_request();
        let result = test::call_service(&service, req).await;

        assert_eq!(StatusCode::UNPROCESSABLE_ENTITY, result.status());
    }
}

mod update {
    use actix_web::http::StatusCode;
    use actix_web::test;
    use assert_matches::assert_matches;
    use diesel::insert_into;
    use diesel_async::RunQueryDsl;
    use pokedex_rs::models::pokemon::Pokemon;
    use serde_json::json;
    use serial_test::file_serial;

    use crate::init_test_service;
    use crate::integration_helpers::factories::pokemon::{
        build_create_pokemon, build_update_pokemon,
    };

    #[test_log::test(actix_web::test)]
    #[file_serial(api_v1_pokemons)]
    async fn test_update_existing() {
        use pokedex_rs::schema::pokemons::dsl::*;

        init_test_service!(app, service);

        let new_pokemon = build_create_pokemon();
        let new_pokemon_id: i64;
        {
            let mut connection = app.get_pooled_connection().await;
            new_pokemon_id = insert_into(pokemons)
                .values(&new_pokemon)
                .returning(id)
                .get_result(&mut connection)
                .await
                .unwrap();
        }

        let update_pokemon = build_update_pokemon(&new_pokemon);
        let req = test::TestRequest::put()
            .uri(&format!("/api/v1/pokemons/{}", new_pokemon_id))
            .set_json(update_pokemon)
            .to_request();
        let api_pokemon: Pokemon = test::call_and_read_body_json(&service, req).await;

        assert_eq!(format!("{}_updated", new_pokemon.name), api_pokemon.name);
        assert_matches!(api_pokemon.type_2, None);
    }

    #[test_log::test(actix_web::test)]
    #[file_serial(api_v1_pokemons)]
    async fn test_update_nonexistent() {
        init_test_service!(app, service);

        let pokemon_id = i64::MAX;
        let update_pokemon = build_update_pokemon(&build_create_pokemon());
        let req = test::TestRequest::put()
            .uri(&format!("/api/v1/pokemons/{}", pokemon_id))
            .set_json(update_pokemon)
            .to_request();
        let result = test::call_service(&service, req).await;

        assert_eq!(StatusCode::NOT_FOUND, result.status());
    }

    #[test_log::test(actix_web::test)]
    #[file_serial(api_v1_pokemons)]
    async fn test_invalid_path_param() {
        init_test_service!(app, service);

        let update_pokemon = build_update_pokemon(&build_create_pokemon());
        let req = test::TestRequest::put()
            .uri("/api/v1/pokemons/foobar")
            .set_json(update_pokemon)
            .to_request();
        let result = test::call_service(&service, req).await;

        assert_eq!(StatusCode::BAD_REQUEST, result.status());
    }

    #[test_log::test(actix_web::test)]
    #[file_serial(api_v1_pokemons)]
    async fn test_invalid_path_param_validation() {
        init_test_service!(app, service);

        let update_pokemon = build_update_pokemon(&build_create_pokemon());
        let req = test::TestRequest::put()
            .uri("/api/v1/pokemons/-1")
            .set_json(update_pokemon)
            .to_request();
        let result = test::call_service(&service, req).await;

        assert_eq!(StatusCode::BAD_REQUEST, result.status());
    }

    #[test_log::test(actix_web::test)]
    #[file_serial(api_v1_pokemons)]
    async fn test_invalid_payload() {
        use pokedex_rs::schema::pokemons::dsl::*;

        init_test_service!(app, service);

        let new_pokemon = build_create_pokemon();
        let new_pokemon_id: i64;
        {
            let mut connection = app.get_pooled_connection().await;
            new_pokemon_id = insert_into(pokemons)
                .values(&new_pokemon)
                .returning(id)
                .get_result(&mut connection)
                .await
                .unwrap();
        }

        let invalid_payload = json!({
            "foo": "bar"
        });

        let req = test::TestRequest::put()
            .uri(&format!("/api/v1/pokemons/{}", new_pokemon_id))
            .set_json(invalid_payload)
            .to_request();
        let result = test::call_service(&service, req).await;

        assert_eq!(StatusCode::BAD_REQUEST, result.status());
    }

    #[test_log::test(actix_web::test)]
    #[file_serial(api_v1_pokemons)]
    async fn test_invalid_payload_values() {
        use pokedex_rs::schema::pokemons::dsl::*;

        init_test_service!(app, service);

        let new_pokemon = build_create_pokemon();
        let new_pokemon_id: i64;
        {
            let mut connection = app.get_pooled_connection().await;
            new_pokemon_id = insert_into(pokemons)
                .values(&new_pokemon)
                .returning(id)
                .get_result(&mut connection)
                .await
                .unwrap();
        }

        let invalid_payload = json!({
            "number": "foobar",
            "name": "foobar",
            "type_1": "foobar",
            "type_2": "foobar",
            "total": "foobar",
            "hp": "foobar",
            "attack": "foobar",
            "defense": "foobar",
            "sp_atk": "foobar",
            "sp_def": "foobar",
            "speed": "foobar",
            "generation": "foobar",
            "legendary": "foobar"
        });

        let req = test::TestRequest::put()
            .uri(&format!("/api/v1/pokemons/{}", new_pokemon_id))
            .set_json(invalid_payload)
            .to_request();
        let result = test::call_service(&service, req).await;

        assert_eq!(StatusCode::BAD_REQUEST, result.status());
    }

    #[test_log::test(actix_web::test)]
    #[file_serial(api_v1_pokemons)]
    async fn test_invalid_payload_values_validation() {
        use pokedex_rs::schema::pokemons::dsl::*;

        init_test_service!(app, service);

        let new_pokemon = build_create_pokemon();
        let new_pokemon_id: i64;
        {
            let mut connection = app.get_pooled_connection().await;
            new_pokemon_id = insert_into(pokemons)
                .values(&new_pokemon)
                .returning(id)
                .get_result(&mut connection)
                .await
                .unwrap();
        }

        let invalid_payload = json!({
            "number": 0,
            "name": "",
            "type_1": "Love",
            "type_2": "Patience",
            "total": 0,
            "hp": 0,
            "attack": 0,
            "defense": 0,
            "sp_atk": 0,
            "sp_def": 0,
            "speed": 0,
            "generation": 0,
            "legendary": false
        });

        let req = test::TestRequest::put()
            .uri(&format!("/api/v1/pokemons/{}", new_pokemon_id))
            .set_json(invalid_payload)
            .to_request();
        let result = test::call_service(&service, req).await;

        assert_eq!(StatusCode::UNPROCESSABLE_ENTITY, result.status());
    }
}

mod patch {
    use actix_web::http::StatusCode;
    use actix_web::test;
    use diesel::insert_into;
    use diesel_async::RunQueryDsl;
    use serde_json::json;
    use serial_test::file_serial;

    use crate::init_test_service;
    use crate::integration_helpers::factories::pokemon::{
        build_create_pokemon, build_patch_pokemon,
    };

    mod existing {
        use pokedex_rs::models::pokemon::Pokemon;

        use super::*;

        async fn test_patch_existing(patched_type_2: Option<Option<String>>) {
            use pokedex_rs::schema::pokemons::dsl::*;

            init_test_service!(app, service);

            let new_pokemon = build_create_pokemon();
            let new_pokemon_id: i64;
            {
                let mut connection = app.get_pooled_connection().await;
                new_pokemon_id = insert_into(pokemons)
                    .values(&new_pokemon)
                    .returning(id)
                    .get_result(&mut connection)
                    .await
                    .unwrap();
            }

            let patch_pokemon = build_patch_pokemon(&new_pokemon, patched_type_2.clone());
            let req = test::TestRequest::patch()
                .uri(&format!("/api/v1/pokemons/{}", new_pokemon_id))
                .set_json(patch_pokemon)
                .to_request();
            let api_pokemon: Pokemon = test::call_and_read_body_json(&service, req).await;

            assert_eq!(format!("{}_patched", new_pokemon.name), api_pokemon.name);
            match &patched_type_2 {
                None => assert_eq!(new_pokemon.type_2, api_pokemon.type_2),
                Some(None) => assert_eq!(None, api_pokemon.type_2),
                Some(Some(ref value)) => assert_eq!(Some(value), api_pokemon.type_2.as_ref()),
            }
        }

        #[test_log::test(actix_web::test)]
        #[file_serial(api_v1_pokemons)]
        async fn test_patch_with_none() {
            test_patch_existing(None).await;
        }

        #[test_log::test(actix_web::test)]
        #[file_serial(api_v1_pokemons)]
        async fn test_patch_with_some_none() {
            test_patch_existing(Some(None)).await;
        }

        #[test_log::test(actix_web::test)]
        #[file_serial(api_v1_pokemons)]
        async fn test_patch_with_some_some_value() {
            test_patch_existing(Some(Some("Fire".into()))).await;
        }
    }

    #[test_log::test(actix_web::test)]
    #[file_serial(api_v1_pokemons)]
    async fn test_update_nonexistent() {
        init_test_service!(app, service);

        let pokemon_id = i64::MAX;
        let patch_pokemon = build_patch_pokemon(&build_create_pokemon(), None);
        let req = test::TestRequest::patch()
            .uri(&format!("/api/v1/pokemons/{}", pokemon_id))
            .set_json(patch_pokemon)
            .to_request();
        let result = test::call_service(&service, req).await;

        assert_eq!(StatusCode::NOT_FOUND, result.status());
    }

    #[test_log::test(actix_web::test)]
    #[file_serial(api_v1_pokemons)]
    async fn test_invalid_path_param() {
        init_test_service!(app, service);

        let patch_pokemon = build_patch_pokemon(&build_create_pokemon(), None);
        let req = test::TestRequest::patch()
            .uri("/api/v1/pokemons/foobar")
            .set_json(patch_pokemon)
            .to_request();
        let result = test::call_service(&service, req).await;

        assert_eq!(StatusCode::BAD_REQUEST, result.status());
    }

    #[test_log::test(actix_web::test)]
    #[file_serial(api_v1_pokemons)]
    async fn test_invalid_path_param_validation() {
        init_test_service!(app, service);

        let patch_pokemon = build_patch_pokemon(&build_create_pokemon(), None);
        let req = test::TestRequest::patch()
            .uri("/api/v1/pokemons/-1")
            .set_json(patch_pokemon)
            .to_request();
        let result = test::call_service(&service, req).await;

        assert_eq!(StatusCode::BAD_REQUEST, result.status());
    }

    #[test_log::test(actix_web::test)]
    #[file_serial(api_v1_pokemons)]
    async fn test_invalid_payload() {
        use pokedex_rs::schema::pokemons::dsl::*;

        init_test_service!(app, service);

        let new_pokemon = build_create_pokemon();
        let new_pokemon_id: i64;
        {
            let mut connection = app.get_pooled_connection().await;
            new_pokemon_id = insert_into(pokemons)
                .values(&new_pokemon)
                .returning(id)
                .get_result(&mut connection)
                .await
                .unwrap();
        }

        let invalid_payload = json!({
            "foo": "bar"
        });

        let req = test::TestRequest::patch()
            .uri(&format!("/api/v1/pokemons/{}", new_pokemon_id))
            .set_json(invalid_payload)
            .to_request();
        let result = test::call_service(&service, req).await;

        assert_eq!(StatusCode::BAD_REQUEST, result.status());
    }

    #[test_log::test(actix_web::test)]
    #[file_serial(api_v1_pokemons)]
    async fn test_invalid_payload_values() {
        use pokedex_rs::schema::pokemons::dsl::*;

        init_test_service!(app, service);

        let new_pokemon = build_create_pokemon();
        let new_pokemon_id: i64;
        {
            let mut connection = app.get_pooled_connection().await;
            new_pokemon_id = insert_into(pokemons)
                .values(&new_pokemon)
                .returning(id)
                .get_result(&mut connection)
                .await
                .unwrap();
        }

        let invalid_payload = json!({
            "number": "foobar",
            "name": "foobar",
            "type_1": "foobar",
            "type_2": "foobar",
            "total": "foobar",
            "hp": "foobar",
            "attack": "foobar",
            "defense": "foobar",
            "sp_atk": "foobar",
            "sp_def": "foobar",
            "speed": "foobar",
            "generation": "foobar",
            "legendary": "foobar"
        });

        let req = test::TestRequest::patch()
            .uri(&format!("/api/v1/pokemons/{}", new_pokemon_id))
            .set_json(invalid_payload)
            .to_request();
        let result = test::call_service(&service, req).await;

        assert_eq!(StatusCode::BAD_REQUEST, result.status());
    }

    #[test_log::test(actix_web::test)]
    #[file_serial(api_v1_pokemons)]
    async fn test_invalid_payload_values_validation() {
        use pokedex_rs::schema::pokemons::dsl::*;

        init_test_service!(app, service);

        let new_pokemon = build_create_pokemon();
        let new_pokemon_id: i64;
        {
            let mut connection = app.get_pooled_connection().await;
            new_pokemon_id = insert_into(pokemons)
                .values(&new_pokemon)
                .returning(id)
                .get_result(&mut connection)
                .await
                .unwrap();
        }

        let invalid_payload = json!({
            "number": 0,
            "name": "",
            "type_1": "Love",
            "type_2": "Patience",
            "total": 0,
            "hp": 0,
            "attack": 0,
            "defense": 0,
            "sp_atk": 0,
            "sp_def": 0,
            "speed": 0,
            "generation": 0,
            "legendary": false
        });

        let req = test::TestRequest::patch()
            .uri(&format!("/api/v1/pokemons/{}", new_pokemon_id))
            .set_json(invalid_payload)
            .to_request();
        let result = test::call_service(&service, req).await;

        assert_eq!(StatusCode::UNPROCESSABLE_ENTITY, result.status());
    }
}

mod delete {
    use actix_web::http::StatusCode;
    use actix_web::test;
    use diesel::{insert_into, QueryDsl};
    use diesel_async::RunQueryDsl;
    use pokedex_rs::models::pokemon::Pokemon;
    use serial_test::file_serial;

    use crate::init_test_service;
    use crate::integration_helpers::factories::pokemon::build_create_pokemon;

    #[test_log::test(actix_web::test)]
    #[file_serial(api_v1_pokemons)]
    async fn test_delete_existing() {
        use pokedex_rs::schema::pokemons::dsl::*;

        init_test_service!(app, service);

        let new_pokemon = build_create_pokemon();
        let new_pokemon_id: i64;
        {
            let mut connection = app.get_pooled_connection().await;
            new_pokemon_id = insert_into(pokemons)
                .values(&new_pokemon)
                .returning(id)
                .get_result(&mut connection)
                .await
                .unwrap();
        }

        let req = test::TestRequest::delete()
            .uri(&format!("/api/v1/pokemons/{}", new_pokemon_id))
            .to_request();
        let result = test::call_service(&service, req).await;

        assert!(result.status().is_success());

        let mut connection = app.get_pooled_connection().await;
        let result: Result<Pokemon, _> = pokemons.find(new_pokemon_id).first(&mut connection).await;

        assert_eq!(Err(diesel::NotFound), result);
    }

    #[test_log::test(actix_web::test)]
    #[file_serial(api_v1_pokemons)]
    async fn test_delete_nonexistent() {
        init_test_service!(app, service);

        let pokemon_id = i64::MAX;
        let req = test::TestRequest::delete()
            .uri(&format!("/api/v1/pokemons/{}", pokemon_id))
            .to_request();
        let result = test::call_service(&service, req).await;

        assert_eq!(StatusCode::NOT_FOUND, result.status());
    }

    #[test_log::test(actix_web::test)]
    #[file_serial(api_v1_pokemons)]
    async fn test_invalid_path_param() {
        init_test_service!(app, service);

        let req = test::TestRequest::delete()
            .uri("/api/v1/pokemons/foobar")
            .to_request();
        let result = test::call_service(&service, req).await;

        assert_eq!(StatusCode::BAD_REQUEST, result.status());
    }

    #[test_log::test(actix_web::test)]
    #[file_serial(api_v1_pokemons)]
    async fn test_invalid_path_param_validation() {
        init_test_service!(app, service);

        let req = test::TestRequest::delete()
            .uri("/api/v1/pokemons/-1")
            .to_request();
        let result = test::call_service(&service, req).await;

        assert_eq!(StatusCode::BAD_REQUEST, result.status());
    }
}
