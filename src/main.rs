mod cli;
mod print_stack;
mod repo;
mod walker;

use cli::get_args;
use repo::Git2Reporter;

use std::{env::current_dir, error::Error};
use walker::{SimpleWalker, Walker};

fn main() -> Result<(), Box<dyn Error>> {
    let args = get_args();
    let path = &args.path.unwrap_or(current_dir().unwrap());
    let reporter = Git2Reporter::new();
    let mut out = std::io::stdout();
    let walker = SimpleWalker::new(&reporter);

    walker.report(path, args.depth, &mut out);

    Ok(())
}
