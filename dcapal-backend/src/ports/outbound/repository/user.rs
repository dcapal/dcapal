#[allow(unused_imports)]
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
            r#"select 
                name, 
                email,
                birth_date 
               from "users" where id = $1"#,
            user_id
        )
        .fetch_one(&self.postgres)
        .await?;

        Ok(Some(User {
            name: Option::from(user.name),
            email: user.email,
            birth_date: user.birth_date,
        }))
    }

    pub async fn update_current_user(
        &self,
        user_id: Uuid,
        req: UpdateProfileRequest,
    ) -> Result<()> {
        let birth_date = string_to_date(&req.birth_date).unwrap();

        let query = sqlx::query!(
            r#"
        UPDATE "users" 
        SET 
            name = $1,
            email = $2,
            birth_date = $3
        WHERE id = $4
        "#,
            req.name,
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

    pub async fn upsert_investment_preferences(
        &self,
        user_id: Uuid,
        req: InvestmentPreferences,
    ) -> Result<()> {
        let query = sqlx::query!(
            r#"
            INSERT INTO public.investment_preferences
            (user_id, risk_tolerance, investment_horizon, investment_mode, investment_goal, ai_enabled)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (user_id) DO UPDATE
            SET 
                risk_tolerance     = $2,
                investment_horizon = $3,
                investment_mode    = $4,
                investment_goal    = $5,
                ai_enabled         = $6
            "#,
            user_id as _,
            req.risk_tolerance as _,
            req.investment_horizon as _,
            req.investment_mode as _,
            req.investment_goal as _,
            req.ai_enabled as _
        );

        query.execute(&self.postgres).await?;
        Ok(())
    }
}
