use crate::app::domain::entity::{
    InvestmentGoal, InvestmentMode, InvestmentPreferences, RiskTolerance, User,
};
use crate::app::infra::utils::string_to_date;
use crate::error::Result;
use crate::ports::inbound::rest::user::UpdateProfileRequest;
use uuid::Uuid;

#[derive(Clone)]
pub struct UserRepository {
    pub postgres: sqlx::PgPool,
}

impl UserRepository {
    pub fn new(postgres: sqlx::PgPool) -> Self {
        Self { postgres }
    }

    pub async fn get_current_user(&self, user_id: Uuid) -> Result<Option<User>> {
        let user = sqlx::query!(
            r#"select first_name, last_name, email, birthdate from "users" where id = $1"#,
            user_id
        )
        .fetch_one(&self.postgres)
        .await?;

        Ok(Some(User {
            first_name: user.first_name,
            last_name: user.last_name,
            email: user.email,
            birth_date: user.birthdate,
        }))
    }

    pub async fn update_current_user(
        &self,
        user_id: Uuid,
        req: UpdateProfileRequest,
    ) -> Result<()> {
        let birth_date = req.birth_date.and_then(|date| string_to_date(&date).ok());

        let query = sqlx::query!(
            r#"
        UPDATE "users" 
        SET 
            first_name = COALESCE($1, first_name),
            email = COALESCE($2, email),
            birthdate = COALESCE($3, birthdate)
        WHERE id = $4
        "#,
            req.full_name,
            req.email,
            birth_date,
            user_id
        );

        query.execute(&self.postgres).await?;
        Ok(())
    }

    pub async fn get_investment_preferences(
        &self,
        user_id: Uuid,
    ) -> Result<Option<InvestmentPreferences>> {
        let investment_preferences = sqlx::query_as!(
            InvestmentPreferences,
            r#"
            SELECT 
                risk_tolerance as "risk_tolerance: RiskTolerance",
                investment_horizon,
                investment_mode as "investment_mode: InvestmentMode",
                investment_goal as "investment_goal: InvestmentGoal",
                ai_enabled
            FROM investment_preferences
            WHERE user_id = $1
            "#,
            user_id
        )
        .fetch_one(&self.postgres)
        .await?;

        Ok(Some(InvestmentPreferences {
            risk_tolerance: investment_preferences.risk_tolerance,
            investment_horizon: investment_preferences.investment_horizon,
            investment_mode: investment_preferences.investment_mode,
            investment_goal: investment_preferences.investment_goal,
            ai_enabled: investment_preferences.ai_enabled,
        }))
    }
}
