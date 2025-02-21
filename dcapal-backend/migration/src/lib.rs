pub use sea_orm_migration::prelude::*;

mod m20250131_084915_create_user_table;
mod m20250201_132150_create_table_portfolios;
mod m20250201_132246_create_table_portfolio_asset;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20250131_084915_create_user_table::Migration),
            Box::new(m20250201_132150_create_table_portfolios::Migration),
            Box::new(m20250201_132246_create_table_portfolio_asset::Migration),
        ]
    }
}
