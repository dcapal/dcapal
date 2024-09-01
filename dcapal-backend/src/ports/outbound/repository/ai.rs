use crate::app::domain::entity::{
    Ai, InvestmentGoal, InvestmentMode, InvestmentPreferences, Portfolio, RiskTolerance,
};
use crate::error::Result;
use async_openai::config::OpenAIConfig;
use async_openai::{
    types::{
        ChatCompletionRequestAssistantMessageArgs, ChatCompletionRequestSystemMessageArgs,
        ChatCompletionRequestUserMessageArgs, CreateChatCompletionRequestArgs,
    },
    Client,
};
use uuid::Uuid;

#[derive(Clone)]
pub struct AiRepository {
    pub async_openai: Client<OpenAIConfig>,
    pub postgres: sqlx::PgPool,
}

impl AiRepository {
    pub fn new(async_openai: Client<OpenAIConfig>, postgres: sqlx::PgPool) -> Self {
        Self {
            async_openai,
            postgres,
        }
    }

    async fn get_investment_preferences(
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

    pub async fn get_ai_response(
        &self,
        user_id: Uuid,
        message: String,
        portfolio: Portfolio,
    ) -> Result<Option<Ai>> {
        let user_investment_preferences = self.get_investment_preferences(user_id).await?.unwrap();

        let user_investment_preferences_json =
            serde_json::to_string(&user_investment_preferences).unwrap();

        let portfolio_json = serde_json::to_string(&portfolio).unwrap();

        let request = match message.as_str() {
            "initial" => CreateChatCompletionRequestArgs::default()
                    .max_tokens(512u32)
                    .model("gpt-4")
                    .messages([
                        ChatCompletionRequestSystemMessageArgs::default()
                            .content("This assistant will receive some personal data about users wanting to invest in multiple assets, including ETF, crypto, etc. The users can be of two types: standard users who need a more detailed answer since they are not experts in finance topics, and expert users who have advanced financial knowledge and want technical answers.The data that will be provided include the user's investment portfolio and personal details such as age, investment goal, and so on. The answer should specify that is not financial advice, and it should not contain too many bullet points. Finally, users can also additionally ask for more information with some provided buttons such as \"Explain More\" or \"How to choose a financial advisor\".")
                            .build()?
                            .into(),
                        ChatCompletionRequestAssistantMessageArgs::default()
                            .content(user_investment_preferences_json)
                            .build()?
                            .into(),
                        ChatCompletionRequestUserMessageArgs::default()
                            .content(portfolio_json)
                            .build()?
                            .into(),
                    ])
                    .build()?,
            "market-news" => CreateChatCompletionRequestArgs::default()
                    .max_tokens(512u32)
                    .model("gpt-4")
                    .messages([
                        ChatCompletionRequestSystemMessageArgs::default()
                            .content("When the user asks for market news, the assistant should provide the latest news about the stock market, including the most important events that have happened in the last 24 hours. The answer should be tailored also to the provided data about the user's investment preferences and portfolio.")
                            .build()?
                            .into(),
                        ChatCompletionRequestAssistantMessageArgs::default()
                            .content(user_investment_preferences_json)
                            .build()?
                            .into(),
                        ChatCompletionRequestUserMessageArgs::default()
                            .content(portfolio_json)
                            .build()?
                            .into(),
                    ])
                    .build()?,
            "rebalance-portfolio" => CreateChatCompletionRequestArgs::default()
                    .max_tokens(512u32)
                    .model("gpt-4")
                    .messages([
                        ChatCompletionRequestSystemMessageArgs::default()
                            .content("When the user asks for help rebalancing their portfolio, the assistant should provide a brief explanation of what portfolio rebalancing is and why it is important. The assistant should also provide some general guidelines on how to rebalance a portfolio, including the importance of diversification and the risks of not rebalancing. The assistant should also provide some general guidelines on how to rebalance a portfolio, including the importance of diversification and the risks of not rebalancing. The assistant should also provide some general guidelines on how to rebalance a portfolio, including the importance of diversification and the risks of not rebalancing.")
                            .build()?
                            .into(),
                        ChatCompletionRequestAssistantMessageArgs::default()
                            .content(user_investment_preferences_json)
                            .build()?
                            .into(),
                        ChatCompletionRequestUserMessageArgs::default()
                            .content(portfolio_json)
                            .build()?
                            .into(),
                    ])
                    .build()?,
            _ => CreateChatCompletionRequestArgs::default()
                    .max_tokens(512u32)
                    .model("gpt-4")
                    .messages([
                        ChatCompletionRequestSystemMessageArgs::default()
                            .content("This assistant will receive some personal data about users wanting to invest in multiple assets, including ETF, crypto, etc. The users can be of two types: standard users who need a more detailed answer since they are not experts in finance topics, and expert users who have advanced financial knowledge and want technical answers.The data that will be provided include the user's investment portfolio and personal details such as age, investment goal, and so on. The answer should specify that is not financial advice, and it should not contain too many bullet points. Finally, users can also additionally ask for more information with some provided buttons such as \"Explain More\" or \"How to choose a financial advisor\".")
                            .build()?
                            .into(),
                        ChatCompletionRequestAssistantMessageArgs::default()
                            .content(user_investment_preferences_json)
                            .build()?
                            .into(),
                        ChatCompletionRequestUserMessageArgs::default()
                            .content(portfolio_json)
                            .build()?
                            .into(),
                    ])
                    .build()?,
        };

        let response = self.async_openai.chat().create(request).await?;

        Ok(Some(Ai {
            response: response.choices[0].clone().message.content.unwrap(),
        }))
    }
}
