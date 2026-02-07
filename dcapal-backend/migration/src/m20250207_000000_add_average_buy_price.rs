use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(PortfolioAsset::Table)
                    .add_column(ColumnDef::new(PortfolioAsset::AverageBuyPrice).decimal().null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(PortfolioAsset::Table)
                    .drop_column(PortfolioAsset::AverageBuyPrice)
                    .to_owned(),
            )
            .await
    }
}

#[derive(Iden)]
enum PortfolioAsset {
    Table,
    AverageBuyPrice,
}
