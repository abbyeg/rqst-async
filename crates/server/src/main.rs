use miniserve::{http::StatusCode, Content, Request, Response};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct Chat {
    messages: Vec<String>
}

async fn index(_req: Request) -> Response {
    let content = include_str!("../index.html").to_string();
    Ok(Content::Html(content))
}

async fn post_chat(req: Request) -> Response {
    match req {
        Request::Post(body) => {
            let mut chat: Chat = serde_json::from_str(&body)
                .map_err(|_| StatusCode::BAD_REQUEST)?;
            
            chat.messages.push("How does that make you feel?".to_string());
            
            let chat_str = serde_json::to_string(&chat)
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            return Ok(Content::Json(chat_str))
        },
        _ => {
            return Err(StatusCode::BAD_REQUEST);
        }
    }
    
}

#[tokio::main]
async fn main() {
    miniserve::Server::new()
        .route("/", index)
        .route("/chat", post_chat)
        .run().await
}
