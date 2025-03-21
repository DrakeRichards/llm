use llm::{
    builder::{LLMBackend, LLMBuilder},
    chat::{ChatMessage, StructuredOutputFormat},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or("sk-TESTKEY".into());

    let schema = r#"
    {
        "name": "Student",
        "schema": {
            "type": "object",
            "properties": {
                "name": {
                    "type": "string"
                },
                "age": {
                    "type": "integer"
                },
                "is_student": {
                    "type": "boolean"
                }
            },
            "required": ["name", "age", "is_student"]
        }
    }
"#;
    let schema: StructuredOutputFormat = serde_json::from_str(schema)?;

    let llm = LLMBuilder::new()
        .backend(LLMBackend::OpenAI)
        .api_key(api_key)
        .model("gpt-4o")
        .max_tokens(512)
        .temperature(0.7)
        .stream(false)
        .system("You are an AI assistant that can provide structured output to generate random students as example data. Respond in JSON format using the provided JSON schema.")
        .schema(schema)
        .build()
        .expect("Failed to build LLM (OpenAI)");

    let messages = vec![ChatMessage::user()
        .content("Generate a random student")
        .build()];

    match llm.chat(&messages).await {
        Ok(text) => println!("Chat response:\n{}", text),
        Err(e) => eprintln!("Chat error: {}", e),
    }

    Ok(())
}
