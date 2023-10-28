use git2::{ObjectType, Oid, Repository, TreeWalkResult};
use std::{collections::HashMap, path::Path};
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

#[derive(Debug)]
struct Links {
    previous_commit: Oid,
    links: Vec<Link>,
}

fn read_links(content: &str) -> Result<Links, String> {
    let mut lines = content.split('\n').filter(|s| !s.is_empty());

    let previous_commit = lines
        .next()
        .ok_or_else(|| "unable to read links from empty file!")?;
    let previous_commit = Oid::from_str(&previous_commit).map_err(|err| {
        format!(
            "links file starts with a line that isn't a commit id! Error: {}",
            err
        )
    })?;

    let links: Vec<Link> = lines
        .map(|line| line.parse::<Link>())
        .collect::<Result<_, _>>()?;

    Ok(Links {
        previous_commit,
        links,
    })
}

fn maybe_read_links<P: AsRef<Path>>(path: P) -> Option<Links> {
    match ::std::fs::read_to_string(&path) {
        Ok(content) => Some(read_links(&content).unwrap()),
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
            Some(ObjectType::Blob) => Some(
                read_links(
                    ::std::str::from_utf8(
                        entry
                            .to_object(&repo)
                            .unwrap()
                            .into_blob()
                            .unwrap()
                            .content(),
                    )
                    .unwrap(),
                )
                .unwrap(),
            ),
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

    let mut next_commits = HashMap::new();

    let mut commit = repo.head().unwrap().peel_to_commit().unwrap();
    let mut next_commit_id = None;

    loop {
        if let Some(cm) = next_commit_id {
            next_commits.insert(commit.id(), cm);
        }

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

        next_commit_id = Some(commit.id());
        commit = commit.parent(0).unwrap();
    }

    let commit = repo
        .find_commit(links_from_workdir.as_ref().unwrap().previous_commit)
        .unwrap();
    let commit = repo.find_commit(next_commits[&commit.id()]).unwrap();
    let commit_tree = commit.tree().unwrap();

    for link in links_from_workdir.as_ref().unwrap().links.iter() {
        match link.1 {
            Object::Path(_) => (),
            Object::Fragment { blob_oid, .. } => {
                let blob_entry = commit_tree.get_id(blob_oid).unwrap();
                let blob_name = blob_entry.name().unwrap();

                let mut next_commit_with_change =
                    repo.find_commit(next_commits[&commit.id()]).unwrap();
                let (next_commit_with_change, next_blob_id) = loop {
                    let blob_id_on_next_commit = next_commit_with_change
                        .tree()
                        .unwrap()
                        .get_name(blob_name)
                        .unwrap()
                        .id();

                    if blob_id_on_next_commit != blob_oid {
                        break (next_commit_with_change, blob_id_on_next_commit);
                    } else {
                        next_commit_with_change = repo
                            .find_commit(next_commits[&next_commit_with_change.id()])
                            .unwrap();
                    }
                };

                println!(
                    "Link {} has blob {} that changes on commit {} and becomes blob {}!",
                    link.0,
                    blob_oid,
                    next_commit_with_change.id(),
                    next_blob_id
                );

                let old_blob = repo.find_blob(blob_oid).unwrap();
                let new_blob = repo.find_blob(next_blob_id).unwrap();
                repo.diff_blobs(
                    Some(&old_blob),
                    Some(blob_name),
                    Some(&new_blob),
                    Some(blob_name),
                    None,
                    Some(&mut |delta, num| {
                        println!("We're inside file_cb, num is {}", num);
                        true
                    }),
                    Some(&mut |delta, bin| {
                        println!("We're inside binary_cb!");
                        true
                    }),
                    Some(&mut |delta, hunk| {
                        println!("We're inside hunk_cb!");
                        true
                    }),
                    Some(&mut |delta, hunk, line| {
                        println!("We're inside line_cb!");
                        true
                    }),
                )
                .unwrap();
            }
        }
    }
}
