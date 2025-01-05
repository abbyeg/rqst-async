use miniserve::{http::StatusCode, Content, Request, Response};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, LazyLock};
use tokio::{
    join,
    sync::{mpsc, oneshot},
};

#[derive(Serialize, Deserialize)]
struct Chat {
    messages: Vec<String>,
}

async fn index(_req: Request) -> Response {
    let content = include_str!("../index.html").to_string();
    Ok(Content::Html(content))
}

async fn query_chat(messages: &Arc<Vec<String>>) -> Vec<String> {
    type Payload = (Arc<Vec<String>>, oneshot::Sender<Vec<String>>);
    static SENDER: LazyLock<mpsc::Sender<Payload>> = LazyLock::new(|| {
        let (tx, mut rx) = mpsc::channel::<Payload>(1024);
        tokio::spawn(async move {
            let mut chatbot = chatbot::Chatbot::new(vec![":-)".into(), "^^".into()]);
            while let Some((messages, responder)) = rx.recv().await {
                let response = chatbot.query_chat(&messages).await;
                responder.send(response).unwrap();
            }
        });
        tx
    });

    let (tx, rx) = oneshot::channel();
    SENDER.send((Arc::clone(messages), tx)).await.unwrap();
    rx.await.unwrap()
}

async fn post_chat(req: Request) -> Response {
    let Request::Post(body) = req else {
        return Err(StatusCode::METHOD_NOT_ALLOWED);
    };

    let mut chat: Chat = serde_json::from_str(&body).map_err(|_| StatusCode::BAD_REQUEST)?;

    let messages = Arc::new(chat.messages);
    let (i, mut responses) = join!(chatbot::gen_random_number(), query_chat(&messages));

    let response = responses.remove(i % responses.len());
    chat.messages = Arc::into_inner(messages).unwrap();
    chat.messages.push(response);

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
