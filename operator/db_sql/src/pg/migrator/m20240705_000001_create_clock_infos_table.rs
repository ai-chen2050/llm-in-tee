use sea_orm_migration::prelude::*;
use sea_query::Index;
pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m_20240705_000001_create_clock_infos_table"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    // Define how to apply this migration: Create the clock_infos table.
    // or use manager.alter_table(stmt) to update the table for migration.
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let result = manager
            .create_table(
                Table::create()
                    .table(ClockInfos::Table)
                    .col(
                        ColumnDef::new(ClockInfos::Id)
                            .big_integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(ClockInfos::Clock).string().not_null())
                    .col(ColumnDef::new(ClockInfos::ClockHash).char_len(64).unique_key().not_null())
                    .col(ColumnDef::new(ClockInfos::NodeId).char_len(64).not_null())
                    .col(ColumnDef::new(ClockInfos::MessageId).char_len(64).not_null())
                    .col(ColumnDef::new(ClockInfos::RawMessage).binary().not_null())
                    .col(ColumnDef::new(ClockInfos::EventCount).big_unsigned().not_null())
                    .col(ColumnDef::new(ClockInfos::CreateAt).timestamp())
                    .to_owned(),
            ).await;

        result?;    
        
        // create index
        let msgid_index = Index::create()
            .if_not_exists()
            .name("idx-clockinfos-messageid")
            .table(ClockInfos::Table)
            .col(ClockInfos::MessageId)
            .to_owned();
        let result = manager.create_index(msgid_index).await; 
        result?;

        let nodeid_index = Index::create()
            .if_not_exists()
            .name("idx-clockinfos-nodeid")
            .table(ClockInfos::Table)
            .col(ClockInfos::NodeId)
            .to_owned();
        manager.create_index(nodeid_index).await
    }

    // Define how to rollback this migration: Drop the ClockInfo table.
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ClockInfos::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum ClockInfos {
    Table,
    Id,
    Clock,
    ClockHash,
    NodeId,
    MessageId,
    RawMessage,
    EventCount,
    CreateAt
}