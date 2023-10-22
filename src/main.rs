use git2::{ObjectType, Repository, TreeWalkResult};

fn main() {
    println!("Hello, world!");

    let repo = match Repository::discover(".") {
        Ok(repo) => repo,
        Err(e) => panic!("failed to open: {}", e),
    };

    let mut commit = repo.head().unwrap().peel_to_commit().unwrap();

    loop {
        println!("Commit {}\n  {:?}", commit.id(), commit.message());

        let tree = commit.tree().unwrap();
        tree.walk(git2::TreeWalkMode::PreOrder, |_, entry| {
            if let Some(ObjectType::Blob) = entry.kind() {
                println!("    {} is blob {}", entry.name().unwrap(), entry.id());
            }

            TreeWalkResult::Ok
        })
        .unwrap();

        if commit.parent_count() == 0 {
            println!("End of log!");
            break;
        }
        commit = commit.parent(0).unwrap();
    }
}
