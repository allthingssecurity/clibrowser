use anyhow::Result;

use crate::cli::{ClickArgs, GetArgs};
use crate::dom::Document;
use crate::error::BrowserError;
use crate::output::OutputConfig;
use crate::session::Session;

pub async fn execute(args: ClickArgs, session: &mut Session, out: &OutputConfig, stealth: bool) -> Result<i32> {
    let html = session
        .page_html()
        .ok_or(BrowserError::NoPageLoaded)?;

    let doc = Document::parse(&html);
    let elements = doc.select(&args.selector)?;

    if elements.is_empty() {
        return Err(BrowserError::NoMatch(args.selector.clone()).into());
    }

    let idx = args.index.unwrap_or(0);
    if idx >= elements.len() {
        return Err(BrowserError::IndexOutOfRange {
            index: idx,
            count: elements.len(),
        }
        .into());
    }

    let el = &elements[idx];
    let href = el
        .attributes
        .get("href")
        .ok_or_else(|| BrowserError::Other(format!(
            "Element at index {} has no href attribute", idx
        )))?;

    let get_args = GetArgs {
        url: href.clone(),
        method: "GET".to_string(),
        headers: vec![],
        data: None,
        data_json: None,
        no_follow: false,
        max_redirects: 10,
        timeout: 30,
        user_agent: None,
        stealth,
    };

    super::navigate::execute(get_args, session, out).await
}
