use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(PortfolioAsset::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(PortfolioAsset::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .default(Expr::cust("gen_random_uuid()")),
                    )
                    .col(ColumnDef::new(PortfolioAsset::Symbol).text().not_null())
                    .col(
                        ColumnDef::new(PortfolioAsset::PortfolioId)
                            .uuid()
                            .not_null(),
                    )
                    .col(ColumnDef::new(PortfolioAsset::Name).text().not_null())
                    .col(ColumnDef::new(PortfolioAsset::AssetClass).text().not_null())
                    .col(ColumnDef::new(PortfolioAsset::Currency).text().not_null())
                    .col(ColumnDef::new(PortfolioAsset::Provider).text().not_null())
                    .col(
                        ColumnDef::new(PortfolioAsset::Quantity)
                            .decimal()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(PortfolioAsset::TargetWeight)
                            .decimal()
                            .not_null(),
                    )
                    .col(ColumnDef::new(PortfolioAsset::Price).decimal().not_null())
                    .col(
                        ColumnDef::new(PortfolioAsset::MaxFeeImpact)
                            .decimal()
                            .null(),
                    )
                    .col(ColumnDef::new(PortfolioAsset::FeeType).text().null())
                    .col(ColumnDef::new(PortfolioAsset::FeeAmount).decimal().null())
                    .col(ColumnDef::new(PortfolioAsset::FeeRate).decimal().null())
                    .col(ColumnDef::new(PortfolioAsset::MinFee).decimal().null())
                    .col(ColumnDef::new(PortfolioAsset::MaxFee).decimal().null())
                    .col(
                        ColumnDef::new(PortfolioAsset::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::cust("now()")),
                    )
                    .col(
                        ColumnDef::new(PortfolioAsset::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::cust("now()")),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_portfolio_asset_portfolio_id")
                            .from(PortfolioAsset::Table, PortfolioAsset::PortfolioId)
                            .to(Portfolios::Table, Portfolios::Id),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(PortfolioAsset::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum PortfolioAsset {
    Table,
    Id,
    Symbol,
    PortfolioId,
    Name,
    AssetClass,
    Currency,
    Provider,
    Quantity,
    TargetWeight,
    Price,
    MaxFeeImpact,
    FeeType,
    FeeAmount,
    FeeRate,
    MinFee,
    MaxFee,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum Portfolios {
    Table,
    Id,
}
