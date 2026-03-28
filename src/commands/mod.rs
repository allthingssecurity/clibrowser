pub mod navigate;
pub mod select;
pub mod text;
pub mod links;
pub mod tables;
pub mod click;
pub mod forms;
pub mod headers;
pub mod cookies;
pub mod status;
pub mod session_cmd;
pub mod crawl;
pub mod search;
pub mod rss;
pub mod sitemap;
pub mod markdown;
pub mod pipe;

use anyhow::Result;
use crate::cli::Command;
use crate::output::OutputConfig;
use crate::session::Session;

pub async fn dispatch(cmd: Command, session: &mut Session, out: &OutputConfig, stealth: bool) -> Result<i32> {
    match cmd {
        Command::Get(args) => navigate::execute(args, session, out).await,
        Command::Select(args) => select::execute(args, session, out),
        Command::Text(args) => text::execute(args, session, out),
        Command::Links(args) => links::execute(args, session, out),
        Command::Tables(args) => tables::execute(args, session, out),
        Command::Click(args) => click::execute(args, session, out, stealth).await,
        Command::Forms(args) => forms::execute(args, session, out),
        Command::Fill(args) => forms::fill(args, session, out),
        Command::Submit(args) => forms::submit(args, session, out, stealth).await,
        Command::Headers(args) => headers::execute(args, session, out),
        Command::Cookies(args) => cookies::execute(args, session, out),
        Command::Status => status::execute(session, out),
        Command::Session(args) => session_cmd::execute(args, session, out),
        Command::Crawl(args) => crawl::execute(args, session, out, stealth).await,
        Command::Search(args) => search::execute(args, session, out, stealth).await,
        Command::Rss(args) => rss::execute(args, session, out, stealth).await,
        Command::Sitemap(args) => sitemap::execute(args, session, out, stealth).await,
        Command::Markdown(args) => markdown::execute(args, session, out),
        Command::Pipe(args) => pipe::execute(args, session, out, stealth).await,
    }
}
