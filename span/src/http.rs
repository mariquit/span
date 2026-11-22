use axum::{
    Router,
    body::{Body, to_bytes},
    extract::{Request, State},
    http::{StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::any,
};
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
};
use sha2::{Digest, Sha256};
use span_core::MemoryStore;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::OwnedSemaphorePermit;

tokio::task_local! {
    pub static MCP_PERMIT: Arc<OwnedSemaphorePermit>;
}

pub fn spawn_mcp_blocking<F, T>(work: F) -> tokio::task::JoinHandle<T>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    let permit = MCP_PERMIT.with(Clone::clone);
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        work()
    })
}

#[derive(Clone)]
struct HttpState {
    store: MemoryStore,
    paths: super::RuntimePaths,
    token_hash: [u8; 32],
    permits: Arc<tokio::sync::Semaphore>,
    rate: Arc<Mutex<(Instant, usize)>>,
}

const MAX_OPERATIONS: u32 = 8;

pub async fn serve(
    store: MemoryStore,
    paths: super::RuntimePaths,
    listener: tokio::net::TcpListener,
    token: String,
    generated: bool,
) -> anyhow::Result<()> {
    let mcp_store = store.clone();
    let config = StreamableHttpServerConfig::default()
        .with_legacy_session_mode(false)
        .with_json_response(true)
        .with_max_request_body_bytes(1_048_576);
    let shutdown = config.cancellation_token.clone();
    let mcp = StreamableHttpService::new(
        move || {
            Ok(super::MemoryService {
                store: mcp_store.clone(),
            })
        },
        Arc::new(LocalSessionManager::default()),
        config,
    );
    let state = HttpState {
        store,
        paths,
        token_hash: Sha256::digest(token.as_bytes()).into(),
        permits: Arc::new(tokio::sync::Semaphore::new(MAX_OPERATIONS as usize)),
        rate: Arc::new(Mutex::new((Instant::now(), 0))),
    };
    let app = Router::new()
        .route_service("/mcp", mcp)
        .fallback(any(handle))
        .with_state(state.clone())
        .layer(middleware::from_fn_with_state(state, guard));
    let mut terminate = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    if generated {
        eprintln!("Generated SPAN_HTTP_TOKEN={token}");
    }
    tracing::info!(
        port = listener.local_addr()?.port(),
        "authenticated loopback HTTP server ready"
    );
    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {},
                _ = terminate.recv() => {},
            }
            shutdown.cancel();
        })
        .await?;
    Ok(())
}

async fn guard(State(state): State<HttpState>, request: Request, next: Next) -> Response {
    let supplied = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .unwrap_or("");
    let hash: [u8; 32] = Sha256::digest(supplied.as_bytes()).into();
    let mismatch = hash
        .iter()
        .zip(state.token_hash)
        .fold(0u8, |acc, (a, b)| acc | (a ^ b));
    if mismatch != 0 {
        return error(StatusCode::UNAUTHORIZED, "unauthorized");
    }
    if request.headers().contains_key(header::ORIGIN) {
        return error(
            StatusCode::FORBIDDEN,
            "browser cross-origin requests are disabled",
        );
    }
    {
        let Ok(mut rate) = state.rate.lock() else {
            return error(StatusCode::SERVICE_UNAVAILABLE, "rate limiter unavailable");
        };
        if rate.0.elapsed() >= Duration::from_secs(60) {
            *rate = (Instant::now(), 0);
        }
        if rate.1 >= 120 {
            return error(StatusCode::TOO_MANY_REQUESTS, "request rate exceeded");
        }
        rate.1 += 1;
    }
    let reindex =
        request.method() == axum::http::Method::POST && request.uri().path() == "/reindex";
    let permits = if reindex { MAX_OPERATIONS } else { 1 };
    let Ok(permit) = state.permits.clone().try_acquire_many_owned(permits) else {
        return error(StatusCode::SERVICE_UNAVAILABLE, "server busy");
    };
    let (mut parts, body) = request.into_parts();
    if parts
        .headers
        .get(header::CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
        .is_some_and(|length| length > 1_048_576)
    {
        return error(StatusCode::PAYLOAD_TOO_LARGE, "invalid or oversized body");
    }
    let bytes = match tokio::time::timeout(Duration::from_secs(10), to_bytes(body, 1_048_576)).await
    {
        Err(_) => return error(StatusCode::REQUEST_TIMEOUT, "request body timed out"),
        Ok(Err(_)) => return error(StatusCode::PAYLOAD_TOO_LARGE, "invalid or oversized body"),
        Ok(Ok(bytes)) => bytes,
    };
    parts.extensions.insert(Arc::new(permit));
    let dispatch = next.run(Request::from_parts(parts, Body::from(bytes)));
    let mut response = match if reindex {
        Ok(dispatch.await)
    } else {
        tokio::time::timeout(Duration::from_secs(30), dispatch).await
    } {
        Ok(response) => response,
        Err(_) => error(
            StatusCode::GATEWAY_TIMEOUT,
            "operation timed out; verify mutation state before retrying",
        ),
    };
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("no-store"),
    );
    response
}

async fn handle(State(state): State<HttpState>, request: Request) -> Response {
    let method = request.method().to_string();
    let path = request.uri().path().to_owned();
    let reindex = method == "POST" && path == "/reindex";
    let permit = request
        .extensions()
        .get::<Arc<tokio::sync::OwnedSemaphorePermit>>()
        .cloned();
    let bytes = match to_bytes(request.into_body(), 1_048_576).await {
        Ok(bytes) => bytes,
        Err(_) => return error(StatusCode::PAYLOAD_TOO_LARGE, "invalid or oversized body"),
    };
    let Ok(body) = String::from_utf8(bytes.to_vec()) else {
        return error(StatusCode::BAD_REQUEST, "body must be UTF-8");
    };
    let started = Instant::now();
    let operation = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        let result =
            super::handle_http_request(&method, &path, &body, &state.store, Some(&state.paths));
        let status = result.status_code().0;
        let mut bytes = Vec::new();
        std::io::Read::read_to_end(&mut result.into_reader(), &mut bytes)?;
        Ok::<_, std::io::Error>((status, bytes))
    });
    let response = match if reindex {
        Ok(operation.await)
    } else {
        tokio::time::timeout(Duration::from_secs(30), operation).await
    } {
        Ok(Ok(Ok((status, bytes)))) => (
            StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            [
                (header::CONTENT_TYPE, "application/json"),
                (header::CACHE_CONTROL, "no-store"),
            ],
            Body::from(bytes),
        )
            .into_response(),
        Err(_) => error(
            StatusCode::GATEWAY_TIMEOUT,
            "operation timed out; verify mutation state before retrying",
        ),
        _ => error(StatusCode::INTERNAL_SERVER_ERROR, "operation failed"),
    };
    tracing::info!(
        status = response.status().as_u16(),
        elapsed_ms = started.elapsed().as_millis(),
        "HTTP request completed"
    );
    response
}

fn error(status: StatusCode, message: &str) -> Response {
    (status, axum::Json(serde_json::json!({"error": message}))).into_response()
}
