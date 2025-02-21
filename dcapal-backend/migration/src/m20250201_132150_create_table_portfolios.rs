use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Portfolios::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Portfolios::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .default(SimpleExpr::Custom("gen_random_uuid()".into())),
                    )
                    .col(ColumnDef::new(Portfolios::UserId).uuid().not_null())
                    .col(ColumnDef::new(Portfolios::Name).string().not_null())
                    .col(ColumnDef::new(Portfolios::Currency).string().not_null())
                    .col(
                        ColumnDef::new(Portfolios::Deleted)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(Portfolios::LastUpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(SimpleExpr::Custom("NOW()".into())),
                    )
                    .col(
                        ColumnDef::new(Portfolios::MaxFeeImpact)
                            .decimal_len(20, 10)
                            .null(),
                    )
                    .col(ColumnDef::new(Portfolios::FeeType).string().null())
                    .col(
                        ColumnDef::new(Portfolios::FeeAmount)
                            .decimal_len(20, 10)
                            .null(),
                    )
                    .col(
                        ColumnDef::new(Portfolios::FeeRate)
                            .decimal_len(20, 10)
                            .null(),
                    )
                    .col(
                        ColumnDef::new(Portfolios::MinFee)
                            .decimal_len(20, 10)
                            .null(),
                    )
                    .col(
                        ColumnDef::new(Portfolios::MaxFee)
                            .decimal_len(20, 10)
                            .null(),
                    )
                    .col(
                        ColumnDef::new(Portfolios::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(SimpleExpr::Custom("NOW()".into())),
                    )
                    .col(
                        ColumnDef::new(Portfolios::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(SimpleExpr::Custom("NOW()".into())),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_portfolios_user_id")
                            .from(Portfolios::Table, Portfolios::UserId)
                            .to(Users::Table, Users::Id),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Portfolios::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Portfolios {
    Table,
    Id,
    UserId,
    Name,
    Currency,
    Deleted,
    LastUpdatedAt,
    MaxFeeImpact,
    FeeType,
    FeeAmount,
    FeeRate,
    MinFee,
    MaxFee,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}
