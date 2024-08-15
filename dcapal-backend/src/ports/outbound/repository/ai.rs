use crate::app::domain::entity::Ai;
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

    pub async fn get_ai_response(&self, user_id: Uuid) -> Result<Option<Ai>> {
        let request = CreateChatCompletionRequestArgs::default()
            .max_tokens(512u32)
            .model("gpt-3.5-turbo")
            .messages([
                ChatCompletionRequestSystemMessageArgs::default()
                    .content("I will give you some data and with this you should tell me if the investment is suitable or not.")
                    .build()?
                    .into(),
                ChatCompletionRequestUserMessageArgs::default()
                    .content("Bear in mind that you should also point that is not a financial advice and the answer should be directed to the user.")
                    .build()?
                    .into(),
                ChatCompletionRequestAssistantMessageArgs::default()
                    .content("user data: {\"name\": \"John\", \"age\": 30, \"income\": 50000, \"investment\": 10000}")
                    .build()?
                    .into(),
                ChatCompletionRequestUserMessageArgs::default()
                    .content("portfolio data: {\"stocks\": 5000, \"bonds\": 3000, \"crypto\": 2000}")
                    .build()?
                    .into(),
            ])
            .build()?;

        println!("{}", serde_json::to_string(&request).unwrap());

        let response = self.async_openai.chat().create(request).await?;

        Ok(Some(Ai {
            response: response.choices[0].clone().message.content.unwrap(),
        }))
    }
}
