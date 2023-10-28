use git2::{ObjectType, Oid, Repository, TreeWalkResult};
use std::path::Path;
use uuid::Uuid;

#[derive(Debug)]
enum Object {
    Fragment {
        blob_oid: Oid,
        starting_line: usize,
        starting_column: usize,
        ending_line: usize,
        ending_column: usize,
    },
    Path(String),
}

impl Object {
    fn from_str_iter_mut<'a, I: Iterator<Item = &'a str>>(iter: &mut I) -> Result<Object, String> {
        let object_header = iter.next();
        match object_header {
            Some("fragment") => {
                let fragment_vals: Vec<_> = iter.take(5).collect();
                if fragment_vals.len() < 5 {
                    Err("fragment object wasn't followed by the 5 expected values!".into())
                } else {
                    let blob_oid = Oid::from_str(fragment_vals[0]).map_err(|err| format!("fragment object was followed by a value that isn't a blob id, error: {}", err))?;
                    let starting_line = fragment_vals[1].parse().map_err(|err| {
                        format!(
                            "fragment object has a starting line that isn't an usize, error: {}",
                            err
                        )
                    })?;
                    let starting_column = fragment_vals[1].parse().map_err(|err| {
                        format!(
                            "fragment object has a starting column that isn't an usize, error: {}",
                            err
                        )
                    })?;
                    let ending_line = fragment_vals[1].parse().map_err(|err| {
                        format!(
                            "fragment object has an ending line that isn't an usize, error: {}",
                            err
                        )
                    })?;
                    let ending_column = fragment_vals[1].parse().map_err(|err| {
                        format!(
                            "fragment object has an ending column that isn't an usize, error: {}",
                            err
                        )
                    })?;

                    Ok(Object::Fragment {
                        blob_oid,
                        starting_line,
                        starting_column,
                        ending_line,
                        ending_column,
                    })
                }
            }
            Some("path") => {
                let path = iter.next();
                if let Some(path_val) = path {
                    Ok(Object::Path(path_val.to_string()))
                } else {
                    Err("path object wasn't followed by a path value!".into())
                }
            }
            Some(val) => Err(format!("unknown value '{}' to parse an object!", val)),
            None => Err("tried to parse an object from an empty value!".into()),
        }
    }
}

#[derive(Debug)]
struct Link(Uuid, Object, Object);

impl ::std::str::FromStr for Link {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut string_items = s.split(' ');
        let id = match string_items.next() {
            None => return Err("tried to parse a link from an empty line!".into()),
            Some(id) => Uuid::parse_str(id)
                .map_err(|err| format!("failed to parse uuid for link, error: {}", err))?,
        };
        let obj1 = Object::from_str_iter_mut(&mut string_items)?;
        let obj2 = Object::from_str_iter_mut(&mut string_items)?;

        // TODO: read any flags here.

        Ok(Link(id, obj1, obj2))
    }
}

fn read_links(content: &str) -> Vec<Link> {
    let result: Result<Vec<Link>, String> = content
        .split('\n')
        .filter(|s| !s.is_empty())
        .map(|line| line.parse())
        .collect();
    result.unwrap()
}

fn maybe_read_links<P: AsRef<Path>>(path: P) -> Option<Vec<Link>> {
    match ::std::fs::read_to_string(&path) {
        Ok(content) => Some(read_links(&content)),
        Err(err) => {
            println!(
                "Encountered an error trying to read links from path {}: {}",
                path.as_ref().to_str().unwrap(),
                err
            );
            None
        }
    }
}

fn main() {
    println!("Hello, world!");

    let repo = match Repository::discover(".") {
        Ok(repo) => repo,
        Err(e) => panic!("failed to open: {}", e),
    };

    let head_tree = repo.head().unwrap().peel_to_tree().unwrap();
    let links_from_head = match head_tree.get_path(&Path::new(".kupli/links")) {
        Ok(entry) => match entry.kind() {
            Some(ObjectType::Blob) => Some(read_links(
                ::std::str::from_utf8(
                    entry
                        .to_object(&repo)
                        .unwrap()
                        .into_blob()
                        .unwrap()
                        .content(),
                )
                .unwrap(),
            )),
            _ => None,
        },
        Err(err) => {
            println!("Couldn't find links file in HEAD's tree! {}", err);
            None
        }
    };
    let links_from_workdir = match repo.workdir() {
        Some(p) => maybe_read_links(p.join(".kupli/links")),
        None => None,
    };

    println!("Links from HEAD: {:?}", links_from_head);
    println!("Links from workdir: {:?}", links_from_workdir);

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
