use std::sync::Arc;

use miniserve::{http::StatusCode, Content, Request, Response};
use serde::{Deserialize, Serialize};
use tokio::join;

#[derive(Serialize, Deserialize)]
struct Chat {
    messages: Vec<String>,
}

async fn index(_req: Request) -> Response {
    let content = include_str!("../index.html").to_string();
    Ok(Content::Html(content))
}

async fn generate_chat_from_body(body: &str) -> Result<Chat, Response> {
    let mut chat: Chat = serde_json::from_str(body).map_err(|_| Err(StatusCode::BAD_REQUEST))?;
    let messages = Arc::new(chat.messages);
    let messages_ref = Arc::clone(&messages);
    let query = tokio::spawn(async move { chatbot::query_chat(&messages_ref).await });

    let (query_result, i) = join!(query, chatbot::gen_random_number());

    let mut resp_vec = query_result.map_err(|_| Err(StatusCode::INTERNAL_SERVER_ERROR))?;
    let resp = resp_vec.remove(i % resp_vec.len());
    chat.messages = Arc::into_inner(messages).unwrap();
    chat.messages.push(resp);

    Ok(chat)
}

async fn post_chat(req: Request) -> Response {
    let Request::Post(body) = req else {
        return Err(StatusCode::METHOD_NOT_ALLOWED);
    };

    let chat = generate_chat_from_body(&body)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let chat_str = serde_json::to_string(&chat).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Content::Json(chat_str))
}

#[tokio::main]
async fn main() {
    miniserve::Server::new()
        .route("/", index)
        .route("/chat", post_chat)
        .run()
        .await
}
