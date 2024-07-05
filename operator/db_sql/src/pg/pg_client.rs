use sea_orm::*;
use sea_orm_migration::prelude::*;
use super::migrator::Migrator;

pub async fn setup_db(request_url: &str, db_name: &str) -> Result<DatabaseConnection, DbErr>  {
    let db = Database::connect(request_url).await?;
    let db = match db.get_database_backend() {
       DbBackend::MySql => {
           db.execute(Statement::from_string(
               db.get_database_backend(),
               format!("CREATE DATABASE IF NOT EXISTS `{}`;", db_name),
           ))
           .await?;

           let url = format!("{}/{}", request_url, db_name);
           Database::connect(&url).await?
       }
       DbBackend::Postgres => {
           db.execute(Statement::from_string(
               db.get_database_backend(),
               format!("DROP DATABASE IF EXISTS \"{}\";", db_name),
           ))
           .await?;
           db.execute(Statement::from_string(
               db.get_database_backend(),
               format!("CREATE DATABASE \"{}\";", db_name),
           ))
           .await?;

           let url = format!("{}/{}", request_url, db_name);
           Database::connect(&url).await?
       }
       DbBackend::Sqlite => db,
    };

    let schema_manager = SchemaManager::new(&db); // To investigate the schema

    Migrator::up(&db.clone(), None).await?;
    assert!(schema_manager.has_table("clock_infos").await?);

    Ok(db)
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::entities::{prelude::*, *};
    use futures::executor::block_on;
    use common::ordinary_clock;

    const DATABASE_PG_URL: &str = "postgres://postgres:hetu@0.0.0.0:5432";
    const DB_NAME: &str = "operator_db";

    #[tokio::test]
    #[ignore]
    async fn set_up_db() {   // could add the function to server cli command
        match block_on(setup_db(DATABASE_PG_URL, DB_NAME)) {
            Err(err) => {
                panic!("{}", err);
            }
            Ok(_db) => async {  }.await,
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_run() {
        let url = format!("{}/{}", DATABASE_PG_URL, DB_NAME);
        let db = Database::connect(&url).await.expect("failed to connect to database");
        {
            let clock = ordinary_clock::OrdinaryClock::new();
            clock.update(vec![].iter(), 0);
            clock.update(vec![].iter(), 1);
            let clock_str = serde_json::to_string(&clock).unwrap();
            let clock_info = clock_infos::ActiveModel {
                clock: ActiveValue::Set(clock_str.clone()),
                clock_hash: ActiveValue::Set("todo".to_owned()),
                node_id: ActiveValue::Set("todo".to_owned()),
                message_id: ActiveValue::Set("todo".to_owned()),
                raw_message: ActiveValue::Set(Vec::from("todo")),
                event_count: ActiveValue::Set(1),
                // create_at: ActiveValue::Set(current_time),
                ..Default::default()
            };
            let res = ClockInfos::insert(clock_info).exec(&db).await.expect("insert error");
            
            let clock_vec = ClockInfos::find().all(&db).await.expect("query error");
            println!("clock_vec-1 = {:?}", clock_vec);
            
            let clock_info2 = clock_infos::ActiveModel {
                id: ActiveValue::Set(res.last_insert_id),
                clock_hash: ActiveValue::Set("todo1".to_owned()),
                node_id: ActiveValue::Set("todo1".to_owned()),
                message_id: ActiveValue::Set("todo1".to_owned()),
                raw_message: ActiveValue::Set(Vec::from("todo1")),
                event_count: ActiveValue::Set(2),
                ..Default::default()
            };
            let _ = clock_info2.clone().update(&db).await;

            let mut clock3 = clock_info2;
            clock3.id = ActiveValue::Set(2);
            clock3.event_count = ActiveValue::Set(2);
            clock3.clock_hash = ActiveValue::Set("todo2".to_owned());
            clock3.clock = ActiveValue::Set(clock_str);
            println!("clock3 = {:?}", clock3);
            ClockInfos::insert(clock3).exec(&db).await.expect("insert error");
            let clock_vec = ClockInfos::find().all(&db).await.expect("query error");
            println!("clock_vec-2 = {:?}", clock_vec);
        }
    }

    #[tokio::test]
    #[ignore]
    async fn get_table_counts() {
        let url = format!("{}/{}", DATABASE_PG_URL, DB_NAME);
        let db = Database::connect(&url).await.expect("failed to connect to database");

        let clocks = ClockInfos::find()
            .count(&db)
            .await
            .expect("query error");

        println!("clocks = {:?}", clocks);
    }

    #[tokio::test]
    #[ignore]
    async fn get_last_clock() {
        let url = format!("{}/{}", DATABASE_PG_URL, DB_NAME);
        let db = Database::connect(&url).await.expect("failed to connect to database");

        let clocks = ClockInfos::find()
            .order_by_desc(clock_infos::Column::Id)
            .one(&db)
            .await
            .expect("query error");

        println!("clocks = {:?}", clocks);
    }
}