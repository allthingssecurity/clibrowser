pub mod status;
pub mod log;
pub mod diff;
pub mod show;
pub mod blame;
pub mod branches;
pub mod tags;
pub mod stashes;
pub mod remotes;

pub mod files;
pub mod cat;
pub mod history;
pub mod contributors;
pub mod search;
pub mod find;

pub mod stage;
pub mod unstage;
pub mod commit;
pub mod branch;
pub mod checkout;
pub mod tag;
pub mod stash;
pub mod merge;
pub mod rebase;
pub mod cherry_pick;
pub mod reset;

pub mod fetch;
pub mod pull;
pub mod push;
pub mod clone_cmd;

pub mod summary;
pub mod changes;
pub mod pr_diff;
pub mod conflicts;
pub mod config;

pub mod analyze;
pub mod tree;
pub mod deps;
pub mod languages;
pub mod context;

use anyhow::Result;
use crate::cli::Command;
use crate::git_ctx::GitContext;
use crate::output::OutputConfig;

pub fn dispatch(cmd: Command, ctx: &mut GitContext, out: &OutputConfig) -> Result<i32> {
    match cmd {
        // Read
        Command::Status(a) => status::execute(a, ctx, out),
        Command::Log(a) => log::execute(a, ctx, out),
        Command::Diff(a) => diff::execute(a, ctx, out),
        Command::Show(a) => show::execute(a, ctx, out),
        Command::Blame(a) => blame::execute(a, ctx, out),
        Command::Branches(a) => branches::execute(a, ctx, out),
        Command::Tags(a) => tags::execute(a, ctx, out),
        Command::Stashes(a) => stashes::execute(a, ctx, out),
        Command::Remotes(a) => remotes::execute(a, ctx, out),

        // Files
        Command::Files(a) => files::execute(a, ctx, out),
        Command::Cat(a) => cat::execute(a, ctx, out),
        Command::History(a) => history::execute(a, ctx, out),
        Command::Contributors(a) => contributors::execute(a, ctx, out),
        Command::Search(a) => search::execute(a, ctx, out),
        Command::Find(a) => find::execute(a, ctx, out),

        // Write
        Command::Stage(a) => stage::execute(a, ctx, out),
        Command::Unstage(a) => unstage::execute(a, ctx, out),
        Command::Commit(a) => commit::execute(a, ctx, out),
        Command::Branch(a) => branch::execute(a, ctx, out),
        Command::Checkout(a) => checkout::execute(a, ctx, out),
        Command::Tag(a) => tag::execute(a, ctx, out),
        Command::Stash(a) => stash::execute(a, ctx, out),
        Command::Merge(a) => merge::execute(a, ctx, out),
        Command::Rebase(a) => rebase::execute(a, ctx, out),
        Command::CherryPick(a) => cherry_pick::execute(a, ctx, out),
        Command::Reset(a) => reset::execute(a, ctx, out),

        // Remote
        Command::Fetch(a) => fetch::execute(a, ctx, out),
        Command::Pull(a) => pull::execute(a, ctx, out),
        Command::Push(a) => push::execute(a, ctx, out),

        // Agent
        Command::Summary(a) => summary::execute(a, ctx, out),
        Command::Changes(a) => changes::execute(a, ctx, out),
        Command::PrDiff(a) => pr_diff::execute(a, ctx, out),
        Command::Conflicts(a) => conflicts::execute(a, ctx, out),
        Command::Config(a) => config::execute(a, Some(ctx), out),

        // Analysis (v0.2)
        Command::Analyze(a) => analyze::execute(a, ctx, out),
        Command::Tree(a) => tree::execute(a, ctx, out),
        Command::Deps(a) => deps::execute(a, ctx, out),
        Command::Languages(a) => languages::execute(a, ctx, out),
        Command::Context(a) => context::execute(a, ctx, out),

        // Clone handled separately in main
        Command::Clone(_) => unreachable!("Clone should be handled before dispatch"),
    }
}
