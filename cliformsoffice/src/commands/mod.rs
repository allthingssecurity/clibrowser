pub mod info;
pub mod text;
pub mod pages;
pub mod tables;
pub mod images;
pub mod search;
pub mod markdown;
pub mod styles;
pub mod comments;
pub mod links;
pub mod toc;
pub mod create;
pub mod write_cmd;
pub mod add_text;
pub mod add_table;
pub mod add_image;
pub mod convert;
pub mod merge;
pub mod split;
pub mod rotate;
pub mod protect;

use anyhow::Result;
use crate::cli::Command;
use crate::output::OutputConfig;

pub fn dispatch(cmd: Command, out: &OutputConfig, format_override: Option<&str>) -> Result<i32> {
    match cmd {
        Command::Info(args) => info::execute(args, out, format_override),
        Command::Text(args) => text::execute(args, out, format_override),
        Command::Pages(args) => pages::execute(args, out, format_override),
        Command::Tables(args) => tables::execute(args, out, format_override),
        Command::Images(args) => images::execute(args, out, format_override),
        Command::Search(args) => search::execute(args, out, format_override),
        Command::Markdown(args) => markdown::execute(args, out, format_override),
        Command::Styles(args) => styles::execute(args, out, format_override),
        Command::Comments(args) => comments::execute(args, out, format_override),
        Command::Links(args) => links::execute(args, out, format_override),
        Command::Toc(args) => toc::execute(args, out, format_override),
        Command::Create(args) => create::execute(args, out),
        Command::Write(args) => write_cmd::execute(args, out),
        Command::AddText(args) => add_text::execute(args, out, format_override),
        Command::AddTable(args) => add_table::execute(args, out, format_override),
        Command::AddImage(args) => add_image::execute(args, out, format_override),
        Command::Convert(args) => convert::execute(args, out, format_override),
        Command::Merge(args) => merge::execute(args, out),
        Command::Split(args) => split::execute(args, out),
        Command::Rotate(args) => rotate::execute(args, out),
        Command::Protect(args) => protect::execute(args, out),
    }
}
