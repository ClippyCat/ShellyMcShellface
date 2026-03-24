use std::sync::Arc;
use axum::{
    Router,
    extract::State,
    response::{Html, sse::{Event, KeepAlive, Sse}},
    routing::get,
};
use tokio::sync::broadcast;
use tokio_stream::{StreamExt, wrappers::BroadcastStream};
use crate::{event_buffer::EventBuffer, types::SseEvent};

const INDEX_HTML: &str = include_str!("frontend/index.html");
const APP_JS: &str = include_str!("frontend/app.js");
const ANSI_JS: &str = include_str!("frontend/ansi.js");

#[derive(Clone)]
pub struct AppState {
    pub tx: Arc<broadcast::Sender<SseEvent>>,
    pub event_buf: Arc<EventBuffer>,
}

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/", get(serve_index))
        .route("/app.js", get(serve_app_js))
        .route("/ansi.js", get(serve_ansi_js))
        .route("/events", get(sse_handler))
        .with_state(state)
}

async fn serve_index() -> Html<&'static str> {
    Html(INDEX_HTML)
}

async fn serve_app_js() -> ([(&'static str, &'static str); 1], &'static str) {
    ([("content-type", "application/javascript")], APP_JS)
}

async fn serve_ansi_js() -> ([(&'static str, &'static str); 1], &'static str) {
    ([("content-type", "application/javascript")], ANSI_JS)
}

async fn sse_handler(
    State(state): State<AppState>,
) -> Sse<impl futures_core::Stream<Item = Result<Event, anyhow::Error>>> {
    // Subscribe before reading buffer to avoid missing events
    let rx = state.tx.subscribe();
    let buffered = state.event_buf.snapshot();

    let buffered_stream = tokio_stream::iter(buffered)
        .map(|ev| Ok(ev));

    let live_stream = BroadcastStream::new(rx)
        .filter_map(|r| r.ok())
        .map(|ev| Ok(ev));

    let combined = buffered_stream.chain(live_stream)
        .map(|r: Result<SseEvent, anyhow::Error>| {
            r.and_then(|ev| {
                let (event_type, data) = ev.to_sse_parts();
                Ok(Event::default().event(event_type).data(data))
            })
        });

    Sse::new(combined).keep_alive(KeepAlive::default())
}

/// Starts the HTTP server on the given port. Returns only on error.
pub async fn run_server(state: AppState, port: u16) -> anyhow::Result<()> {
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
    let router = build_router(state);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, router).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::SseEvent;
    use std::sync::Arc;
    use tokio::sync::broadcast;

    #[tokio::test]
    async fn test_sse_format_from_event() {
        let ev = SseEvent::output("hello\n");
        let (event_type, data) = ev.to_sse_parts();
        let _sse = axum::response::sse::Event::default()
            .event(event_type)
            .data(data);
        let ev2 = SseEvent::output("hello\n");
        let (t, d) = ev2.to_sse_parts();
        assert_eq!(t, "output");
        assert!(d.contains("hello"));
    }

    #[tokio::test]
    async fn test_app_state_holds_shared_refs() {
        let (tx, _rx) = broadcast::channel::<SseEvent>(1024);
        let buf = Arc::new(crate::event_buffer::EventBuffer::new());
        let state = AppState { tx: Arc::new(tx), event_buf: buf };
        state.event_buf.push(SseEvent::connected());
        assert_eq!(state.event_buf.snapshot().len(), 1);
    }
}
