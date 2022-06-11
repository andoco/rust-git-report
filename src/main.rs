mod printer;

use std::{collections::HashMap, env};

use ansi_term::Colour;
use git2::{BranchType, ErrorCode, Repository, Status};
use std::{error::Error, fs};

use printer::{Printer, SimplePrinter};

#[derive(Debug)]
pub struct RepoReport {
    repo_status: RepoStatus,
    branch_status: HashMap<String, BranchStatus>,
}

#[derive(Debug, PartialEq)]
enum RepoStatus {
    Clean,
    Changed,
    Unpushed,
    NotRepo,
    Error(String),
}

#[derive(Debug)]
enum BranchStatus {
    Current,
    NoUpstream,
    Ahead,
    Error(String),
}

impl std::fmt::Display for RepoStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Clean => write!(f, "Clean"),
            Self::Changed => write!(f, "Changed"),
            Self::Unpushed => write!(f, "Unpushed"),
            Self::NotRepo => write!(f, "Not a repo"),
            Self::Error(e) => write!(f, "Error: {}", e),
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();

    let root_path = match args.get(1) {
        Some(path) => path,
        None => ".",
    };

    for entry in fs::read_dir(root_path)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            let path = path.to_str().ok_or("Not a valid path")?;
            let report = report_on_repo(path);

            match report {
                Ok(report) => {
                    let printer = SimplePrinter;
                    println!("{}", printer.print_report(path, report));
                }
                Err(err) => println!("{} ERROR: {}", path, err),
            }
        }
    }

    Ok(())
}

fn report_on_repo(path: &str) -> Result<RepoReport, Box<dyn Error>> {
    let repo = &Repository::open(path)?;
    let status = get_repo_status(repo);
    let branches = get_branch_status(repo)?;

    Ok(RepoReport {
        repo_status: status,
        branch_status: branches,
    })
}

fn print_repo_report(path: &str, report: Result<RepoReport, Box<dyn Error>>, column_size: usize) {
    let to_print = match report {
        Ok(report) => match report.repo_status {
            RepoStatus::Clean => Colour::Green.paint("Clean"),
            RepoStatus::Changed => Colour::Purple.paint("Changed"),
            RepoStatus::Unpushed => Colour::Purple.paint("Not pushed"),
            RepoStatus::NotRepo => Colour::Yellow.paint("Not a repo"),
            RepoStatus::Error(e) => Colour::Red.paint(format!("Error {}", e)),
        },
        Err(e) => Colour::Red.paint(format!("Error {}", e)),
    };

    println!(
        "{}{: <width$}{}",
        path,
        " ",
        to_print,
        width = column_size - path.len() + 1,
    );
}

fn get_repo_status(repo: &Repository) -> RepoStatus {
    match repo.statuses(None) {
        Ok(statuses) => {
            let mut report_statuses = statuses
                .iter()
                .map(|s| s.status())
                .map(|s| map_git_status_to_report_status(&s));

            if report_statuses.any(|s| s == RepoStatus::Changed) {
                return RepoStatus::Changed;
            }

            return RepoStatus::Clean;
        }
        Err(e) => RepoStatus::Error(e.to_string()),
    }
}

fn get_branch_status(repo: &Repository) -> Result<HashMap<String, BranchStatus>, Box<dyn Error>> {
    let mut branch_changes = HashMap::<String, BranchStatus>::new();

    match repo.branches(Some(BranchType::Local)) {
        Ok(branches) => {
            for (b, _) in branches.map(|x| x.unwrap()) {
                let branch_name = String::from(b.name()?.ok_or("could not get branch name")?);

                if b.upstream().is_err() {
                    branch_changes.insert(branch_name, BranchStatus::NoUpstream);
                    continue;
                }

                let local_oid = repo.refname_to_id(b.get().name().ok_or("invalid ref name")?)?;
                let upstream_oid =
                    repo.refname_to_id(b.upstream()?.get().name().ok_or("invalid ref name")?)?;

                let ahead_behind = repo.graph_ahead_behind(local_oid, upstream_oid);
                let branch_status = match ahead_behind {
                    Ok((ahead, _)) => {
                        if ahead > 0 {
                            BranchStatus::Ahead
                        } else {
                            BranchStatus::Current
                        }
                    }
                    Err(e) => BranchStatus::Error(e.to_string()),
                };

                branch_changes.insert(branch_name, branch_status);
            }
        }
        Err(e) => panic!("{}", e),
    }

    Ok(branch_changes)
}

fn map_git_error_to_repo_status(err: git2::Error) -> RepoStatus {
    if err.code() == ErrorCode::NotFound {
        return RepoStatus::NotRepo;
    }
    RepoStatus::Error(err.to_string())
}

fn map_git_status_to_report_status(status: &Status) -> RepoStatus {
    let changed_mask = Status::INDEX_NEW
        | Status::INDEX_MODIFIED
        | Status::INDEX_DELETED
        | Status::INDEX_RENAMED
        | Status::INDEX_TYPECHANGE
        | Status::WT_NEW
        | Status::WT_MODIFIED
        | Status::WT_DELETED
        | Status::WT_RENAMED
        | Status::WT_TYPECHANGE;

    let has_changes = *status & changed_mask == *status;

    if has_changes {
        return RepoStatus::Changed;
    }

    return RepoStatus::Clean;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_git_status_to_report_status() {
        let expected_changed_statuses = [
            Status::INDEX_NEW,
            Status::INDEX_MODIFIED,
            Status::INDEX_DELETED,
            Status::INDEX_RENAMED,
            Status::INDEX_TYPECHANGE,
            Status::WT_NEW,
            Status::WT_MODIFIED,
            Status::WT_DELETED,
            Status::WT_RENAMED,
            Status::WT_TYPECHANGE,
        ];

        for status in expected_changed_statuses.iter() {
            assert_eq!(RepoStatus::Changed, map_git_status_to_report_status(status))
        }
    }
}
