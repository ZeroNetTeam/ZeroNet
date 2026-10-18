//! The config finder: search the public feeds, starting with the verified
//! list, for a server that works on this network right now, the way the
//! Android app does. Nothing is sent anywhere about the result.
//!
//! ```text
//! known ─ real-test, all at once, the found servers that worked here
//!   │     before
//!   │
//! search ─ only when that was not enough: discovery over the public feeds
//!          (tiered, streaming TCP → real tests), skipping what was tested
//! ```
//!
//! Everything reaches the frame loop as a [`FinderEvent`]; the finder holds
//! no UI state and does no database work. The user's own profiles are never
//! part of a search: only servers from the public feeds are.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use serde_json::Value;
use tokio::sync::mpsc::UnboundedSender;
use zero_discovery::feed::FeedSource;
use zero_discovery::{CancellationToken, DiscoverRequest, LinkInfo, TestRequest};

/// Not Cloudflare: Worker-served configs cannot reach Cloudflare addresses.
pub const PROBE_URL: &str = "http://www.gstatic.com/generate_204";
/// Found servers from earlier searches tested before searching.
pub const HISTORY_LINKS: usize = 12;

/// Cloudflare-fronted variants of the user's own history links to also test in
/// the known stage. A CDN-fronted config answers on many edge IPs and alternate
/// HTTPS ports, and the TLS SNI (not the IP) routes it — so a blocked address or
/// port is routed around without a new server. Bounded: the whole known stage
/// still tests at most a few dozen endpoints. Non-frontable links (REALITY,
/// plain TCP-TLS) produce none, so this is a no-op for them.
pub const FRONT_VARIANTS: usize = 18;

/// The edge sample when a request carries no per-install seed (tests).
const DEFAULT_FRONT_SEED: u64 = 0x5a52_4159_4346_524e;

/// What a search step reports.
#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::large_enum_variant)]
pub enum FinderEvent {
    /// `known`, then the discovery stages: `history`, `fetch`, `parse`, `tcp`, `real`.
    Stage(String),
    Progress(Progress),
    /// A server that carried a real request, with its delay.
    Alive {
        info: LinkInfo,
        delay_ms: u32,
    },
    /// A known server that did not answer (successes come as [`FinderEvent::Alive`]).
    Failed {
        key: String,
    },
    /// Something the user may want to know (a feed that failed, …).
    Note(String),
    /// The search is over.
    Done {
        alive: usize,
        reason: String,
    },
}

/// Counters of the running search.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Progress {
    pub candidates: usize,
    pub tcp_done: usize,
    pub tcp_open: usize,
    pub real_done: usize,
    pub alive: usize,
}

/// One search.
#[derive(Debug, Clone)]
pub struct FinderRequest {
    /// Share links of found servers that worked before, best first.
    pub history: Vec<String>,
    pub sources: Vec<FeedSource>,
    /// Feed cache (`<data dir>/finder`).
    pub cache_dir: Option<PathBuf>,
    /// Stop after this many working servers.
    pub want_alive: usize,
    pub max_seconds: u64,
    /// Picks which Cloudflare edges fronting tries. Fixed per install, so
    /// probe evidence builds up against the same edges, but different between
    /// installs, so not every client leans on the same few addresses.
    pub front_seed: u64,
    /// The user's "Allow insecure" switch, for the tests and the search.
    pub allow_insecure: bool,
}

impl Default for FinderRequest {
    fn default() -> Self {
        Self {
            history: Vec::new(),
            sources: zero_discovery::sources::enabled(&[], 2),
            cache_dir: None,
            want_alive: 3,
            max_seconds: 90,
            front_seed: DEFAULT_FRONT_SEED,
            allow_insecure: false,
        }
    }
}

type Tx = UnboundedSender<FinderEvent>;

/// Run one search to completion (or cancellation); ends with [`FinderEvent::Done`].
pub async fn run(request: FinderRequest, tx: Tx, cancel: CancellationToken) {
    let want = request.want_alive.max(1);
    let mut alive = 0usize;
    let mut tested: Vec<String> = Vec::new();

    // ---- 1. known servers: this machine's earlier finds
    // One entry per server, keyed by its link key.
    let mut known: HashMap<String, String> = HashMap::new();
    for link in request.history.iter().take(HISTORY_LINKS) {
        known
            .entry(zero_discovery::link_key(link))
            .or_insert_with(|| link.clone());
    }
    // BPB-style fronting: front the user's own history links across a bounded
    // set of Cloudflare edges and test those too, so a config whose written
    // address or port is blocked is still reached through an edge that is not.
    for link in
        zero_discovery::front_via_edges(&request.history, request.front_seed, 1, FRONT_VARIANTS)
    {
        known.entry(zero_discovery::link_key(&link)).or_insert(link);
    }
    if !known.is_empty() && !cancel.is_cancelled() {
        let _ = tx.send(FinderEvent::Stage("known".into()));
        let links: Vec<String> = known.values().cloned().collect();
        tested.extend(known.keys().cloned());
        let test = TestRequest {
            concurrency: links.len().clamp(1, 32),
            timeout_ms: 4000,
            probe_url: PROBE_URL.into(),
            links,
            allow_insecure: request.allow_insecure,
            ..TestRequest::default()
        };
        let events = collect(
            |sink, cancel| zero_discovery::test_links(test, sink, cancel),
            &cancel,
        )
        .await;
        for event in events {
            if event["t"] != "result" {
                continue;
            }
            let key = event["key"].as_str().unwrap_or_default().to_string();
            let Some(link) = known.get(&key) else {
                continue;
            };
            let delay = event["delay_ms"].as_i64().unwrap_or(-1);
            if delay < 0 {
                let _ = tx.send(FinderEvent::Failed { key });
                continue;
            }
            let Ok(candidate) = zero_discovery::link::parse_candidate(link) else {
                continue;
            };
            alive += 1;
            let _ = tx.send(FinderEvent::Alive {
                info: candidate.info,
                delay_ms: delay as u32,
            });
        }
    }
    if cancel.is_cancelled() {
        let _ = tx.send(FinderEvent::Done {
            alive,
            reason: "cancelled".into(),
        });
        return;
    }
    if alive >= want {
        let _ = tx.send(FinderEvent::Done {
            alive,
            reason: "enough".into(),
        });
        return;
    }

    // ---- 2. search the public feeds
    let discover = DiscoverRequest {
        sources: request.sources.clone(),
        cache_dir: request.cache_dir.as_ref().map(|dir| dir.join("feeds")),
        exclude_keys: tested,
        want_alive: want - alive,
        max_seconds: request.max_seconds,
        tcp_concurrency: 256,
        tcp_timeout_ms: 1500,
        tcp_stop_after_open: 1500,
        real_concurrency: 48,
        real_timeout_ms: 3000,
        probe_url: PROBE_URL.into(),
        next_tier_if_alive_below: want.min(3),
        fetch: true,
        allow_insecure: request.allow_insecure,
        ..DiscoverRequest::default()
    };
    let forward = {
        let tx = tx.clone();
        let found = Arc::new(Mutex::new(0usize));
        let counter = Arc::clone(&found);
        let callback = move |event: Value| match event["t"].as_str() {
            Some("stage") => {
                let _ = tx.send(FinderEvent::Stage(
                    event["stage"].as_str().unwrap_or_default().into(),
                ));
            }
            Some("progress") => {
                let n = |k: &str| event[k].as_u64().unwrap_or(0) as usize;
                let _ = tx.send(FinderEvent::Progress(Progress {
                    candidates: n("candidates"),
                    tcp_done: n("tcp_done"),
                    tcp_open: n("tcp_open"),
                    real_done: n("real_done"),
                    alive: n("alive"),
                }));
            }
            Some("alive") => {
                if let Ok(info) = serde_json::from_value::<LinkInfoWire>(event["info"].clone()) {
                    *counter.lock().unwrap_or_else(|p| p.into_inner()) += 1;
                    let _ = tx.send(FinderEvent::Alive {
                        info: info.into(),
                        delay_ms: event["delay_ms"].as_u64().unwrap_or(0) as u32,
                    });
                }
            }
            Some("error") => {
                let _ = tx.send(FinderEvent::Note(
                    event["message"].as_str().unwrap_or_default().into(),
                ));
            }
            _ => {}
        };
        (found, callback)
    };
    let (found, callback) = forward;
    let reason = stream(
        move |sink, cancel| async move {
            zero_discovery::discover(discover, sink, cancel)
                .await
                .as_str()
                .to_string()
        },
        callback,
        &cancel,
    )
    .await;
    alive += *found.lock().unwrap_or_else(|p| p.into_inner());
    let _ = tx.send(FinderEvent::Done { alive, reason });
}

/// `LinkInfo` as it crosses the event stream (it is `Serialize` only).
#[derive(serde::Deserialize)]
struct LinkInfoWire {
    key: String,
    link: String,
    name: String,
    protocol: String,
    transport: String,
    security: String,
    host: String,
    port: u16,
    country: String,
    class: String,
}

impl From<LinkInfoWire> for LinkInfo {
    fn from(w: LinkInfoWire) -> Self {
        LinkInfo {
            key: w.key,
            link: w.link,
            name: w.name,
            protocol: w.protocol,
            transport: w.transport,
            security: w.security,
            host: w.host,
            port: w.port,
            country: w.country,
            class: w.class,
            fp: None,
        }
    }
}

/// Run a job that writes to an event sink and collect every event.
async fn collect<F, Fut>(job: F, cancel: &CancellationToken) -> Vec<Value>
where
    F: FnOnce(zero_discovery::EventSink, CancellationToken) -> Fut,
    Fut: std::future::Future<Output = zero_discovery::EndReason>,
{
    let store: Arc<Mutex<Vec<Value>>> = Arc::default();
    let sink_store = Arc::clone(&store);
    stream(
        move |sink, cancel| async move {
            job(sink, cancel).await;
        },
        move |event| {
            sink_store
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .push(event)
        },
        cancel,
    )
    .await;
    let events = std::mem::take(&mut *store.lock().unwrap_or_else(|p| p.into_inner()));
    events
}

/// Run a job that writes to an event sink, handing each event to `on_event`
/// as it arrives. Returns the job's own result once every event is delivered.
async fn stream<F, Fut, T, E>(job: F, on_event: E, cancel: &CancellationToken) -> T
where
    F: FnOnce(zero_discovery::EventSink, CancellationToken) -> Fut,
    Fut: std::future::Future<Output = T>,
    E: Fn(Value) + Send + Sync + 'static,
{
    let callback: zero_discovery::EventCallback = Arc::new(move |batch: String| {
        for line in batch.lines() {
            if let Ok(event) = serde_json::from_str::<Value>(line) {
                on_event(event);
            }
        }
    });
    let (sink, flusher) = zero_discovery::batching_sink(callback);
    let result = job(sink, cancel.child_token()).await;
    // The job's sink handles are gone; the flusher delivers the rest and ends.
    let _ = flusher.await;
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn a_search_with_nothing_to_search_ends_at_once() {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let request = FinderRequest {
            sources: Vec::new(),
            max_seconds: 5,
            ..FinderRequest::default()
        };
        run(request, tx, CancellationToken::new()).await;
        let mut events = Vec::new();
        while let Ok(event) = rx.try_recv() {
            events.push(event);
        }
        assert!(
            matches!(events.last(), Some(FinderEvent::Done { alive: 0, .. })),
            "{events:?}"
        );
    }

    #[tokio::test]
    async fn a_cancelled_search_says_so() {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let cancel = CancellationToken::new();
        cancel.cancel();
        let request = FinderRequest {
            history: vec![
                "vless://00000000-0000-0000-0000-000000000000@127.0.0.1:9?security=none#x".into(),
            ],
            ..FinderRequest::default()
        };
        run(request, tx, cancel).await;
        let mut last = None;
        while let Ok(event) = rx.try_recv() {
            last = Some(event);
        }
        assert_eq!(
            last,
            Some(FinderEvent::Done {
                alive: 0,
                reason: "cancelled".into()
            })
        );
    }
}
