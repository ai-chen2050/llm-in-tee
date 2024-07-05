use sea_orm_migration::prelude::*;

mod m20240705_000001_create_clock_infos_table;

/// Use the sea-orm-cli to generate data entity, 
/// command like as follow:
/// 
/// ```sh
/// sea-orm-cli generate entity \
/// -u mysql://root:password@localhost:3306/bakeries_db \
/// -o src/entities
/// ```
/// 
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20240705_000001_create_clock_infos_table::Migration),
        ]
    }
}