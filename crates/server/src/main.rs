use miniserve::{http::StatusCode, Content, Request, Response};
use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    sync::{Arc, LazyLock},
};
use tokio::{
    fs, join,
    sync::{mpsc, oneshot},
    task::JoinSet,
};

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
enum MessagesResponse {
    Success { messages: Vec<String> },
    Cancelled,
}

#[derive(Serialize, Deserialize)]
struct MessagesRequest {
    messages: Vec<String>,
}

type Payload = (Arc<Vec<String>>, oneshot::Sender<Option<Vec<String>>>);

async fn index(_req: Request) -> Response {
    let content = include_str!("../index.html").to_string();
    Ok(Content::Html(content))
}

async fn load_docs(paths: Vec<PathBuf>) -> Vec<String> {
    let mut doc_futures = paths
        .into_iter()
        .map(fs::read_to_string)
        .collect::<JoinSet<_>>();
    let mut docs = Vec::new();
    while let Some(result) = doc_futures.join_next().await {
        docs.push(result.unwrap().unwrap());
    }
    docs
}

fn chatbot_thread() -> (mpsc::Sender<Payload>, mpsc::Sender<()>) {
    let (req_tx, mut req_rx) = mpsc::channel::<Payload>(1024);
    let (cancel_tx, mut cancel_rx) = mpsc::channel::<()>(1);
    tokio::spawn(async move {
        let mut chatbot = chatbot::Chatbot::new(vec!["😵‍💫".into(), "🤔".into()]);
        while let Some((messages, responder)) = req_rx.recv().await {
            let paths = chatbot.retrieval_documents(&messages);
            let docs = load_docs(paths).await;

            let chat_fut = chatbot.query_chat(&messages, &docs);
            let cancel_fut = cancel_rx.recv();
            tokio::select! {
                response = chat_fut => {
                    responder.send(Some(response)).unwrap();
                },
                _ = cancel_fut => {
                    responder.send(None).unwrap();
                }
            }
        }
    });
    (req_tx, cancel_tx)
}

static CHATBOT_THREAD: LazyLock<(mpsc::Sender<Payload>, mpsc::Sender<()>)> =
    LazyLock::new(chatbot_thread);

async fn query_chat(messages: &Arc<Vec<String>>) -> Option<Vec<String>> {
    let (tx, rx) = oneshot::channel();
    CHATBOT_THREAD
        .0
        .send((Arc::clone(messages), tx))
        .await
        .unwrap();

    rx.await.unwrap()
}

async fn post_chat(req: Request) -> Response {
    let Request::Post(body) = req else {
        return Err(StatusCode::METHOD_NOT_ALLOWED);
    };

    let Ok(mut data) = serde_json::from_str::<MessagesRequest>(&body) else {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    };

    let messages = Arc::new(data.messages);
    let (i, responses_opt) = join!(chatbot::gen_random_number(), query_chat(&messages));

    let response = match responses_opt {
        Some(mut responses) => {
            let response = responses.remove(i % responses.len());
            data.messages = Arc::into_inner(messages).unwrap();
            data.messages.push(response);

            MessagesResponse::Success {
                messages: data.messages,
            }
        }
        None => MessagesResponse::Cancelled,
    };

    Ok(Content::Json(
        serde_json::to_string(&response).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    ))
}

async fn post_cancel(req: Request) -> Response {
    let Request::Post(_) = req else {
        return Err(StatusCode::METHOD_NOT_ALLOWED);
    };
    CHATBOT_THREAD.1.send(()).await.unwrap();

    Ok(Content::Html("success".into()))
}

#[tokio::main]
async fn main() {
    miniserve::Server::new()
        .route("/", index)
        .route("/chat", post_chat)
        .route("/cancel", post_cancel)
        .run()
        .await
}
