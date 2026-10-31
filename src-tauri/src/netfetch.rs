use std::io::Read;
use std::time::Duration;

const BROWSER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36";

pub fn fetch_bytes(url: &str, limit: u64) -> Result<Vec<u8>, String> {
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(10))
        .timeout_read(Duration::from_secs(20))
        .redirects(5)
        .user_agent(BROWSER_AGENT)
        .build();
    let response = agent
        .get(url)
        .set("Accept", "application/zip,application/octet-stream,*/*")
        .call()
        .map_err(|e| e.to_string())?;
    let mut data = Vec::new();
    response.into_reader().take(limit + 1).read_to_end(&mut data).map_err(|e| e.to_string())?;
    if data.len() as u64 > limit {
        return Err("too big".into());
    }
    Ok(data)
}

pub enum Fetched {
    Fresh(Vec<u8>, Option<String>),
    NotModified,
}

pub fn fetch_conditional(url: &str, limit: u64, etag: Option<&str>, timeout: Duration) -> Result<Fetched, String> {
    let agent = ureq::AgentBuilder::new().timeout(timeout).redirects(3).user_agent("Plumbob-Launcher").build();
    let mut request = agent.get(url).set("Accept", "application/json,*/*");
    if let Some(tag) = etag {
        request = request.set("If-None-Match", tag);
    }
    let response = request.call().map_err(|e| e.to_string())?;
    if response.status() == 304 {
        return Ok(Fetched::NotModified);
    }
    let tag = response.header("ETag").map(str::to_string);
    let mut data = Vec::new();
    response.into_reader().take(limit + 1).read_to_end(&mut data).map_err(|e| e.to_string())?;
    if data.len() as u64 > limit {
        return Err("too big".into());
    }
    Ok(Fetched::Fresh(data, tag))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read as _, Write};
    use std::net::TcpListener;

    fn serve(body: &'static [u8], status: &'static str) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buf = [0u8; 2048];
                let _ = stream.read(&mut buf);
                let head = format!("HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
                let _ = stream.write_all(head.as_bytes());
                let _ = stream.write_all(body);
            }
        });
        format!("http://127.0.0.1:{port}/file.zip")
    }

    #[test]
    fn a_local_server_response_is_returned_unchanged() {
        let url = serve(b"PK\x03\x04payload", "200 OK");
        assert_eq!(fetch_bytes(&url, 1024).unwrap(), b"PK\x03\x04payload");
    }

    #[test]
    fn oversized_responses_and_http_errors_are_rejected() {
        let url = serve(b"0123456789", "200 OK");
        assert!(fetch_bytes(&url, 4).is_err());
        let url = serve(b"nope", "403 Forbidden");
        assert!(fetch_bytes(&url, 1024).is_err());
    }

    #[test]
    fn an_unreachable_server_is_an_error() {
        assert!(fetch_bytes("http://127.0.0.1:1/x.zip", 1024).is_err());
    }

    #[test]
    fn conditional_fetch_returns_fresh_data_and_limits_size() {
        let url = serve(b"{}", "200 OK");
        match fetch_conditional(&url, 1024, None, Duration::from_secs(3)).unwrap() {
            Fetched::Fresh(data, _) => assert_eq!(data, b"{}"),
            Fetched::NotModified => panic!("unexpected 304"),
        }
        let url = serve(b"0123456789", "200 OK");
        assert!(fetch_conditional(&url, 4, None, Duration::from_secs(3)).is_err());
    }

    #[test]
    fn conditional_fetch_reports_not_modified() {
        let url = serve(b"", "304 Not Modified");
        assert!(matches!(fetch_conditional(&url, 1024, Some("\"abc\""), Duration::from_secs(3)).unwrap(), Fetched::NotModified));
    }
}
