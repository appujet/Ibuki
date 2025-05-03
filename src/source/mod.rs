use http::Http;

pub mod http;

pub struct SourceManager {
    pub http: Http,
}

impl SourceManager {
    pub fn new() -> Self {
        Self {
            http: Http::new(None),
        }
    }
}

impl Default for SourceManager {
    fn default() -> Self {
        SourceManager::new()
    }
}
