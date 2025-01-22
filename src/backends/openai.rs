//! OpenAI API client implementation for chat and completion functionality.
//!
//! This module provides integration with OpenAI's GPT models through their API.

#[cfg(feature = "openai")]
use crate::{
    chat::Tool,
    chat::{ChatMessage, ChatProvider, ChatRole},
    completion::{CompletionProvider, CompletionRequest, CompletionResponse},
    embedding::EmbeddingProvider,
    error::LLMError,
    LLMProvider,
};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

/// Client for interacting with OpenAI's API.
///
/// Provides methods for chat and completion requests using OpenAI's models.
pub struct OpenAI {
    pub api_key: String,
    pub model: String,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub system: Option<String>,
    pub timeout_seconds: Option<u64>,
    pub stream: Option<bool>,
    pub top_p: Option<f32>,
    pub top_k: Option<u32>,
    pub tools: Option<Vec<Tool>>,
    /// Embedding parameters
    pub embedding_encoding_format: Option<String>,
    pub embedding_dimensions: Option<u32>,
    client: Client,
}

/// Individual message in an OpenAI chat conversation.
#[derive(Serialize)]
struct OpenAIChatMessage<'a> {
    #[allow(dead_code)]
    role: &'a str,
    content: &'a str,
}

#[derive(Serialize)]
struct OpenAIEmbeddingRequest {
    model: String,
    input: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    encoding_format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dimensions: Option<u32>,
}

/// Request payload for OpenAI's chat API endpoint.
#[derive(Serialize)]
struct OpenAIChatRequest<'a> {
    model: &'a str,
    messages: Vec<OpenAIChatMessage<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_k: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<Tool>>,
}

/// Tool call from OpenAI's API.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ToolCall {
    /// The ID of the tool call.
    pub id: String,
    /// The type of the tool call.
    #[serde(rename = "type")]
    pub call_type: String,
    /// The function to call.
    pub function: FunctionCall,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FunctionCall {
    /// The name of the function to call.
    pub name: String,
    /// The arguments to pass to the function.
    pub arguments: String,
}

/// Response from OpenAI's chat API endpoint.
#[derive(Deserialize)]
struct OpenAIChatResponse {
    choices: Vec<OpenAIChatChoice>,
}

/// Individual choice within an OpenAI chat API response.
#[derive(Deserialize)]
struct OpenAIChatChoice {
    message: OpenAIChatMsg,
}

/// Message content within an OpenAI chat API response.
#[derive(Deserialize)]
struct OpenAIChatMsg {
    #[allow(dead_code)]
    role: String,
    content: Option<String>,
    tool_calls: Option<Vec<ToolCall>>,
}

#[derive(Deserialize)]
struct OpenAIEmbeddingData {
    embedding: Vec<f32>,
}
#[derive(Deserialize)]
struct OpenAIEmbeddingResponse {
    data: Vec<OpenAIEmbeddingData>,
}

impl OpenAI {
    /// Creates a new OpenAI client with the specified configuration.
    ///
    /// # Arguments
    ///
    /// * `api_key` - OpenAI API key
    /// * `model` - Model to use (defaults to "gpt-3.5-turbo")
    /// * `max_tokens` - Maximum tokens to generate
    /// * `temperature` - Sampling temperature
    /// * `timeout_seconds` - Request timeout in seconds
    /// * `system` - System prompt
    /// * `stream` - Whether to stream responses
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        api_key: impl Into<String>,
        model: Option<String>,
        max_tokens: Option<u32>,
        temperature: Option<f32>,
        timeout_seconds: Option<u64>,
        system: Option<String>,
        stream: Option<bool>,
        top_p: Option<f32>,
        top_k: Option<u32>,
        embedding_encoding_format: Option<String>,
        embedding_dimensions: Option<u32>,
        tools: Option<Vec<Tool>>,
    ) -> Self {
        let mut builder = Client::builder();
        if let Some(sec) = timeout_seconds {
            builder = builder.timeout(std::time::Duration::from_secs(sec));
        }
        Self {
            api_key: api_key.into(),
            model: model.unwrap_or("gpt-3.5-turbo".to_string()),
            max_tokens,
            temperature,
            system,
            timeout_seconds,
            stream,
            top_p,
            top_k,
            tools,
            embedding_encoding_format,
            embedding_dimensions,
            client: builder.build().expect("Failed to build reqwest Client"),
        }
    }
}

impl ChatProvider for OpenAI {
    /// Sends a chat request to OpenAI's API.
    ///
    /// # Arguments
    ///
    /// * `messages` - Slice of chat messages representing the conversation
    /// * `tools` - Optional slice of tools to use in the chat
    /// # Returns
    ///
    /// The model's response text or an error
    fn chat_with_tools(&self, messages: &[ChatMessage], tools: Option<&[Tool]>) -> Result<String, LLMError> {
        if self.api_key.is_empty() {
            return Err(LLMError::AuthError("Missing OpenAI API key".to_string()));
        }

        let mut openai_msgs: Vec<OpenAIChatMessage> = messages
            .iter()
            .map(|m| OpenAIChatMessage {
                role: match m.role {
                    ChatRole::User => "user",
                    ChatRole::Assistant => "assistant",
                },
                content: &m.content,
            })
            .collect();

        if let Some(system) = &self.system {
            openai_msgs.insert(
                0,
                OpenAIChatMessage {
                    role: "system",
                    content: system,
                },
            );
        }



        let body = OpenAIChatRequest {
            model: &self.model,
            messages: openai_msgs,
            max_tokens: self.max_tokens,
            temperature: self.temperature,
            stream: self.stream.unwrap_or(false),
            top_p: self.top_p,
            top_k: self.top_k,
            tools: tools.map(|t| t.to_vec()),
        };

        let mut request = self
            .client
            .post("https://api.openai.com/v1/chat/completions")
            .bearer_auth(&self.api_key)
            .json(&body);

        if let Some(timeout) = self.timeout_seconds {
            request = request.timeout(std::time::Duration::from_secs(timeout));
        }

        let resp = request.send()?.error_for_status()?;
        let json_resp: OpenAIChatResponse = resp.json()?;

        let first_choice = json_resp
            .choices
            .first()
            .ok_or_else(|| LLMError::ProviderError("No choices returned by OpenAI".to_string()))?;

        match (
            &first_choice.message.tool_calls,
            &first_choice.message.content,
        ) {
            (Some(tool_calls), _) => {
                // Prioritize tool calls if present
                serde_json::to_string(tool_calls).map_err(|e| {
                    LLMError::ProviderError(format!("Failed to serialize tool calls: {}", e))
                })
            }
            (None, Some(content)) => {
                // Return content if no tool calls but content exists
                Ok(content.clone())
            }
            (None, None) => {
                // Neither tool calls nor content present
                Err(LLMError::ProviderError(
                    "OpenAI response contained neither content nor tool calls".to_string(),
                ))
            }
        }
    }

    fn chat(&self, messages: &[ChatMessage]) -> Result<String, LLMError> {
        self.chat_with_tools(messages, None)
    }
}

impl CompletionProvider for OpenAI {
    /// Sends a completion request to OpenAI's API.
    ///
    /// Currently not implemented.
    fn complete(&self, _req: &CompletionRequest) -> Result<CompletionResponse, LLMError> {
        Ok(CompletionResponse {
            text: "OpenAI completion not implemented.".into(),
        })
    }
}

#[cfg(feature = "openai")]
impl EmbeddingProvider for OpenAI {
    fn embed(&self, input: Vec<String>) -> Result<Vec<Vec<f32>>, LLMError> {
        if self.api_key.is_empty() {
            return Err(LLMError::AuthError("Missing OpenAI API key".into()));
        }

        let emb_format = self
            .embedding_encoding_format
            .clone()
            .unwrap_or_else(|| "float".to_string());

        let body = OpenAIEmbeddingRequest {
            model: self.model.clone(),
            input,
            encoding_format: Some(emb_format),
            dimensions: self.embedding_dimensions,
        };

        let resp = self
            .client
            .post("https://api.openai.com/v1/embeddings")
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()?
            .error_for_status()?;

        let json_resp: OpenAIEmbeddingResponse = resp.json()?;

        let embeddings = json_resp.data.into_iter().map(|d| d.embedding).collect();
        Ok(embeddings)
    }
}

impl LLMProvider for OpenAI {
    fn tools(&self) -> Option<&[Tool]> {
        self.tools.as_ref().map(|t| t.as_slice())
    }
}
