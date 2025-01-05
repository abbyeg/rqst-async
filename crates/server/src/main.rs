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

async fn generate_response(messages: &[String]) -> String {
    let query = chatbot::query_chat(messages);
    let rand_n = chatbot::gen_random_number();
    let (resp_vec, rand_result) = join!(query, rand_n);
    resp_vec[rand_result % resp_vec.len()].clone()
}

async fn post_chat(req: Request) -> Response {
    match req {
        Request::Post(body) => {
            let mut chat: Chat =
                serde_json::from_str(&body).map_err(|_| StatusCode::BAD_REQUEST)?;

            let resp = generate_response(&chat.messages).await;
            chat.messages.push(resp);

            let chat_str =
                serde_json::to_string(&chat).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            Ok(Content::Json(chat_str))
        }
        _ => Err(StatusCode::BAD_REQUEST),
    }
}

#[tokio::main]
async fn main() {
    miniserve::Server::new()
        .route("/", index)
        .route("/chat", post_chat)
        .run()
        .await
}
