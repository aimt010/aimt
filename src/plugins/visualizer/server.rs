use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::sync::Arc;

use crate::store::Store;
use crate::writer;

const INDEX_HTML: &str = include_str!("index.html");

// ---------------------------------------------------------------------------
// JSON helpers — hand-rolled escaping, no serde
// ---------------------------------------------------------------------------

fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            _ => out.push(c),
        }
    }
    out
}

fn json_string(s: &str) -> String {
    format!("\"{}\"", json_escape(s))
}

// ---------------------------------------------------------------------------
// Tree building
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct Node {
    id: String,
    level: String,
    title: String,
    filename: String,
    parent: Option<String>,
    children: Vec<Node>,
}

fn build_tree(store: &Store) -> Vec<Node> {
    // parent -> children ids
    let mut children_map: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut roots: Vec<String> = Vec::new();

    // Map id -> (level, title, filename, effective_parent)
    // For @frame use `file` field to place under @file; otherwise use `parent`.
    let mut info: BTreeMap<String, (String, String, String, Option<String>)> = BTreeMap::new();
    for id in store.ids() {
        if let Some(ie) = store.get_indexed(&id) {
            let e = &ie.entity;
            let level = e.level.as_str().to_string();
            let title = e
                .field("title")
                .map(|f| f.value.clone())
                .unwrap_or_default();
            let filename = ie
                .path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| format!("{}.pmap", id));
            let effective_parent = if level == "frame" {
                e.field("file")
                    .map(|f| f.value.clone())
                    .filter(|s| !s.is_empty())
                    .or_else(|| {
                        e.field("parent")
                            .map(|f| f.value.clone())
                            .filter(|s| !s.is_empty())
                    })
            } else {
                e.field("parent")
                    .map(|f| f.value.clone())
                    .filter(|s| !s.is_empty())
            };
            info.insert(
                id.clone(),
                (level, title, filename, effective_parent.clone()),
            );
        }
    }

    for (id, (_, _, _, parent)) in &info {
        if let Some(p) = parent {
            if info.contains_key(p) {
                children_map.entry(p.clone()).or_default().push(id.clone());
            } else {
                roots.push(id.clone());
            }
        } else {
            roots.push(id.clone());
        }
    }

    // Sort everything lexical
    for v in children_map.values_mut() {
        v.sort();
    }
    roots.sort();

    fn build_node(
        id: &str,
        info: &BTreeMap<String, (String, String, String, Option<String>)>,
        children_map: &BTreeMap<String, Vec<String>>,
    ) -> Node {
        let (level, title, filename, parent) = info.get(id).unwrap().clone();
        let child_ids = children_map.get(id).cloned().unwrap_or_default();
        let children = child_ids
            .iter()
            .map(|cid| build_node(cid, info, children_map))
            .collect();
        Node {
            id: id.to_string(),
            level,
            title,
            filename,
            parent,
            children,
        }
    }

    roots
        .iter()
        .map(|id| build_node(id, &info, &children_map))
        .collect()
}

fn node_to_json(node: &Node) -> String {
    let mut s = String::new();
    s.push('{');
    s.push_str(&format!("\"id\":{},", json_string(&node.id)));
    s.push_str(&format!("\"level\":{},", json_string(&node.level)));
    s.push_str(&format!("\"title\":{},", json_string(&node.title)));
    s.push_str(&format!("\"filename\":{},", json_string(&node.filename)));
    if let Some(p) = &node.parent {
        s.push_str(&format!("\"parent\":{},", json_string(p)));
    } else {
        s.push_str("\"parent\":null,");
    }
    s.push_str("\"children\":[");
    for (i, c) in node.children.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        s.push_str(&node_to_json(c));
    }
    s.push_str("]}");
    s
}

// ---------------------------------------------------------------------------
// Entity JSON
// ---------------------------------------------------------------------------

fn entity_to_json(store: &Store, id: &str) -> Option<String> {
    let ie = store.get_indexed(id)?;
    let e = &ie.entity;
    let level = e.level.as_str();
    let title = e.field("title").map(|f| f.value.as_str()).unwrap_or("");
    let filename = ie
        .path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| format!("{}.pmap", id));
    let raw = writer::serialize(e).unwrap_or_default();
    let mut s = String::new();
    s.push('{');
    s.push_str(&format!("\"id\":{},", json_string(id)));
    s.push_str(&format!("\"level\":{},", json_string(level)));
    s.push_str(&format!("\"title\":{},", json_string(title)));
    s.push_str(&format!("\"filename\":{},", json_string(&filename)));
    s.push_str(&format!("\"raw\":{},", json_string(&raw)));
    // parent
    if let Some(p) = e.field("parent") {
        s.push_str(&format!("\"parent\":{},", json_string(&p.value)));
    } else {
        s.push_str("\"parent\":null,");
    }
    // path (from IndexedEntity)
    s.push_str(&format!(
        "\"path\":{},",
        json_string(&ie.path.display().to_string())
    ));
    // fields: all header + body
    s.push_str("\"fields\":[");
    let mut first = true;
    for f in e.all_fields() {
        if !first {
            s.push(',');
        }
        first = false;
        s.push_str(&format!(
            "{{\"name\":{},\"value\":{}}}",
            json_string(&f.name),
            json_string(&f.value)
        ));
    }
    s.push_str("],");
    // relations
    s.push_str("\"relations\":[");
    for (i, rel) in e.relations.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        s.push_str("{\"fields\":[");
        for (j, f) in rel.fields.iter().enumerate() {
            if j > 0 {
                s.push(',');
            }
            s.push_str(&format!(
                "{{\"name\":{},\"value\":{}}}",
                json_string(&f.name),
                json_string(&f.value)
            ));
        }
        s.push_str("]}");
    }
    s.push(']');
    s.push('}');
    Some(s)
}

// ---------------------------------------------------------------------------
// Search
// ---------------------------------------------------------------------------

fn search_json(store: &Store, q: &str) -> String {
    let lower = q.to_lowercase();
    let mut results: Vec<(String, String, String)> = Vec::new();
    for id in store.ids() {
        if let Some(e) = store.get(&id) {
            // Avoid cloning `Field::value` before lowercasing; use `as_str()` directly.
            // Keeps Unicode case-insensitive semantics via `to_lowercase()` on borrowed `&str`.
            let title = e.field("title").map(|f| f.value.as_str()).unwrap_or("");
            let level = e.level.as_str();
            if id.to_lowercase().contains(&lower)
                || title.to_lowercase().contains(&lower)
                || level.to_lowercase().contains(&lower)
            {
                // Need owned title/level for results; clone only when pushing
                let title_owned = title.to_string();
                let level_owned = level.to_string();
                results.push((id, level_owned, title_owned));
            }
        }
    }
    results.sort();
    let mut s = String::new();
    s.push('[');
    for (i, (id, level, title)) in results.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        s.push_str(&format!(
            "{{\"id\":{},\"level\":{},\"title\":{}}}",
            json_string(id),
            json_string(level),
            json_string(title)
        ));
    }
    s.push(']');
    s
}

// ---------------------------------------------------------------------------
// Status
// ---------------------------------------------------------------------------

fn status_json(store: &Store, filename: &str) -> String {
    let count = store.len();
    let validation = store.validate();
    let valid = validation.is_ok();
    let mut s = String::new();
    s.push('{');
    s.push_str(&format!("\"filename\":{},", json_string(filename)));
    s.push_str(&format!(
        "\"valid\":{},",
        if valid { "true" } else { "false" }
    ));
    s.push_str(&format!("\"count\":{},", count));
    s.push_str("\"errors\":[");
    if let Err(errs) = validation {
        for (i, e) in errs.iter().enumerate() {
            if i > 0 {
                s.push(',');
            }
            s.push_str(&format!(
                "{{\"id\":{},\"field\":{},\"target\":{},\"kind\":{},\"message\":{}}}",
                json_string(&e.id),
                match &e.field {
                    Some(f) => json_string(f),
                    None => "null".to_string(),
                },
                json_string(&e.target),
                json_string(e.kind.as_str()),
                json_string(&e.message)
            ));
        }
    }
    s.push_str("]}");
    s
}

// ---------------------------------------------------------------------------
// HTTP helpers
// ---------------------------------------------------------------------------

fn http_response(status: &str, content_type: &str, body: &str) -> Vec<u8> {
    http_response_bytes(status, content_type, body.as_bytes())
}

fn http_response_bytes(status: &str, content_type: &str, body: &[u8]) -> Vec<u8> {
    let header = format!(
        "HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\nAccess-Control-Allow-Origin: *\r\n\r\n",
        status,
        content_type,
        body.len()
    );
    let mut out = header.into_bytes();
    out.extend_from_slice(body);
    out
}

fn url_decode(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '%' {
            let h1 = chars.next().unwrap_or('0');
            let h2 = chars.next().unwrap_or('0');
            let hex = format!("{}{}", h1, h2);
            if let Ok(b) = u8::from_str_radix(&hex, 16) {
                out.push(b as char);
            } else {
                out.push('%');
                out.push(h1);
                out.push(h2);
            }
        } else if c == '+' {
            out.push(' ');
        } else {
            out.push(c);
        }
    }
    out
}

fn parse_query_param(path: &str, key: &str) -> Option<String> {
    let q_start = path.find('?')?;
    let query = &path[q_start + 1..];
    for pair in query.split('&') {
        let mut split = pair.splitn(2, '=');
        let k = split.next().unwrap_or("");
        let v = split.next().unwrap_or("");
        if k == key {
            return Some(url_decode(v));
        }
    }
    None
}

fn route_path(path: &str) -> String {
    if let Some(q) = path.find('?') {
        path[..q].to_string()
    } else {
        path.to_string()
    }
}

// ---------------------------------------------------------------------------
// Request handling
// ---------------------------------------------------------------------------

fn handle_client(mut stream: TcpStream, store: Arc<Store>, filename: Arc<String>) {
    let mut buf = vec![0u8; 8192];
    let n = match stream.read(&mut buf) {
        Ok(0) | Err(_) => return,
        Ok(n) => n,
    };
    let req = String::from_utf8_lossy(&buf[..n]).to_string();
    let first_line = req.lines().next().unwrap_or("");
    let parts: Vec<&str> = first_line.split_whitespace().collect();
    if parts.len() < 2 {
        let body = "{\"error\":\"bad request\"}";
        let resp = http_response("400 Bad Request", "application/json", body);
        let _ = stream.write_all(&resp);
        return;
    }
    let method = parts[0];
    let raw_path = parts[1];

    if method != "GET" {
        let body = "{\"error\":\"method not allowed\"}";
        let resp = http_response("405 Method Not Allowed", "application/json", body);
        let _ = stream.write_all(&resp);
        return;
    }

    let path_only = route_path(raw_path);

    // Routing
    let response = if path_only == "/" || path_only == "/index.html" {
        http_response_bytes("200 OK", "text/html; charset=utf-8", INDEX_HTML.as_bytes())
    } else if path_only == "/api/tree" {
        let roots = build_tree(&store);
        let mut body = String::new();
        body.push_str(&format!(
            "{{\"filename\":{},\"count\":{},\"roots\":[",
            json_string(&filename),
            store.len()
        ));
        for (i, node) in roots.iter().enumerate() {
            if i > 0 {
                body.push(',');
            }
            body.push_str(&node_to_json(node));
        }
        body.push_str("]}");
        http_response("200 OK", "application/json", &body)
    } else if let Some(id_part) = path_only.strip_prefix("/api/entity/") {
        let id = url_decode(id_part);
        if let Some(json) = entity_to_json(&store, &id) {
            http_response("200 OK", "application/json", &json)
        } else {
            let body = format!(
                "{{\"error\":\"entity not found\",\"id\":{}}}",
                json_string(&id)
            );
            http_response("404 Not Found", "application/json", &body)
        }
    } else if path_only == "/api/search" {
        let q = parse_query_param(raw_path, "q").unwrap_or_default();
        if q.trim().is_empty() {
            http_response("200 OK", "application/json", "[]")
        } else {
            let body = search_json(&store, &q);
            http_response("200 OK", "application/json", &body)
        }
    } else if path_only == "/api/status" {
        let body = status_json(&store, &filename);
        http_response("200 OK", "application/json", &body)
    } else {
        let body = "{\"error\":\"not found\"}";
        http_response("404 Not Found", "application/json", body)
    };

    let _ = stream.write_all(&response);
}

fn open_browser(url: &str) {
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(url).spawn();
    }
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("xdg-open").arg(url).spawn();
    }
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("cmd")
            .args(["/C", "start", url])
            .spawn();
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        let _ = url;
    }
}

pub fn serve(path: &Path, port: u16, no_open: bool) -> std::io::Result<()> {
    // Open store first — fail fast before binding
    let store = Store::open(path)
        .map_err(|e| std::io::Error::other(format!("failed to open {}: {}", path.display(), e)))?;

    let filename = path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string());

    // Validate for log
    match store.validate() {
        Ok(_) => eprintln!("✓ {} is valid ({} entities)", filename, store.len()),
        Err(errs) => eprintln!("✗ {} has {} validation errors", filename, errs.len()),
    }

    let addr = format!("127.0.0.1:{}", port);
    let listener = match TcpListener::bind(&addr) {
        Ok(l) => l,
        Err(e) if port != 0 => {
            // Try fallback to OS-assigned port
            eprintln!("Failed to bind {}: {}, trying 127.0.0.1:0", addr, e);
            TcpListener::bind("127.0.0.1:0")?
        }
        Err(e) => return Err(e),
    };

    // Ensure loopback only
    let local = listener.local_addr()?;
    if !local.ip().is_loopback() {
        return Err(std::io::Error::other("server must bind to loopback only"));
    }

    let url = format!("http://{}", local);
    println!("Listening on {}", url);
    println!("Opened {}", path.display());
    if !no_open {
        open_browser(&url);
    }
    println!("Press Ctrl-C to stop");

    let store = Arc::new(store);
    let filename = Arc::new(filename);

    for stream in listener.incoming() {
        match stream {
            Ok(s) => {
                let st = Arc::clone(&store);
                let fnm = Arc::clone(&filename);
                // Handle sequentially to avoid thread overhead; spawn lightweight thread for concurrency
                std::thread::spawn(move || handle_client(s, st, fnm));
            }
            Err(e) => eprintln!("accept error: {}", e),
        }
    }
    Ok(())
}
