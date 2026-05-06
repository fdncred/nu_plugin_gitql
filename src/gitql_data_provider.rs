use gitql_core::object::Row;

use gitql_core::values::{Value, boolean::BoolValue, datetime::DateTimeValue, integer::IntValue, null::NullValue, text::TextValue};
use gitql_engine::data_provider::DataProvider;

use gix::refs::Category;

pub struct GitDataProvider {
    pub repos: Vec<gix::Repository>,
}

impl GitDataProvider {
    pub fn new(repos: Vec<gix::Repository>) -> Self {
        Self { repos }
    }
}

impl DataProvider for GitDataProvider {
    fn provide(&self, table: &str, selected_columns: &[String]) -> Result<Vec<Row>, String> {
        let mut rows: Vec<Row> = Vec::new();

        for repository in &self.repos {
            rows.extend(select_gql_objects(repository, table, selected_columns)?);
        }

        Ok(rows)
    }
}

fn select_gql_objects(
    repo: &gix::Repository,
    table: &str,
    selected_columns: &[String],
) -> Result<Vec<Row>, String> {
    match table {
        "refs" => select_references(repo, selected_columns),
        "commits" => select_commits(repo, selected_columns),
        "branches" => select_branches(repo, selected_columns),
        "diffs" => select_diffs(repo, selected_columns),
        "tags" => select_tags(repo, selected_columns),
        _ => Ok(vec![Row { values: vec![] }]),
    }
}

fn select_references(
    repo: &gix::Repository,
    selected_columns: &[String],
) -> Result<Vec<Row>, String> {
    let references = repo.references().map_err(|err| err.to_string())?;
    let repo_path = repo.path().to_string_lossy().into_owned();
    let mut rows: Vec<Row> = Vec::new();

    let reference_iter = references.all().map_err(|err| err.to_string())?;
    for reference in reference_iter.flatten() {
        let mut values: Vec<Box<dyn Value>> = Vec::with_capacity(selected_columns.len());
        for field_name in selected_columns {
            if field_name == "name" {
                let name = reference
                    .name()
                    .category_and_short_name()
                    .map(|(_, sn)| sn)
                    .unwrap_or("".into())
                    .to_string();
                values.push(Box::new(TextValue { value: name }));
                continue;
            }

            if field_name == "full_name" {
                let full_name = reference.name().as_bstr().to_string();
                values.push(Box::new(TextValue { value: full_name }));
                continue;
            }

            if field_name == "type" {
                let category = reference.name().category();
                if matches!(category, Some(Category::LocalBranch)) {
                    values.push(Box::new(TextValue {
                        value: "branch".to_string(),
                    }));
                } else if matches!(category, Some(Category::RemoteBranch)) {
                    values.push(Box::new(TextValue {
                        value: "remote".to_string(),
                    }));
                } else if matches!(category, Some(Category::Tag)) {
                    values.push(Box::new(TextValue {
                        value: "tag".to_string(),
                    }));
                } else if matches!(category, Some(Category::Note)) {
                    values.push(Box::new(TextValue {
                        value: "note".to_string(),
                    }));
                } else {
                    values.push(Box::new(TextValue {
                        value: "other".to_string(),
                    }));
                }
                continue;
            }

            if field_name == "repo" {
                values.push(Box::new(TextValue {
                    value: repo_path.clone(),
                }));
                continue;
            }

            values.push(Box::new(NullValue));
        }

        let row = Row { values };
        rows.push(row);
    }

    Ok(rows)
}

fn select_commits(repo: &gix::Repository, selected_columns: &[String]) -> Result<Vec<Row>, String> {
    let head_id = repo.head_id().map_err(|err| err.to_string())?;
    let repo_path = repo.path().to_string_lossy().into_owned();
    let revwalk = head_id.ancestors().all().map_err(|err| err.to_string())?;
    let mut rows: Vec<Row> = Vec::new();

    for commit_info in revwalk {
        let commit_info = commit_info.map_err(|err| err.to_string())?;
        let commit = repo
            .find_object(commit_info.id)
            .map_err(|err| err.to_string())?
            .into_commit();
        let commit = commit.decode().map_err(|err| err.to_string())?;

        let mut values: Vec<Box<dyn Value>> = Vec::with_capacity(selected_columns.len());
        for column_name in selected_columns {
            if column_name == "commit_id" {
                values.push(Box::new(TextValue {
                    value: commit_info.id.to_string(),
                }));
                continue;
            }

            if column_name == "author_name" {
                let author_name = commit
                    .author()
                    .map(|author| author.name.to_string())
                    .unwrap_or_default();
                values.push(Box::new(TextValue { value: author_name }));
                continue;
            }

            if column_name == "author_email" {
                let author_email = commit
                    .author()
                    .map(|author| author.email.to_string())
                    .unwrap_or_default();
                values.push(Box::new(TextValue { value: author_email }));
                continue;
            }

            if column_name == "committer_name" {
                let committer_name = commit
                    .committer()
                    .map(|committer| committer.name.to_string())
                    .unwrap_or_default();
                values.push(Box::new(TextValue { value: committer_name }));
                continue;
            }

            if column_name == "committer_email" {
                let committer_email = commit
                    .committer()
                    .map(|committer| committer.email.to_string())
                    .unwrap_or_default();
                values.push(Box::new(TextValue { value: committer_email }));
                continue;
            }

            if column_name == "title" {
                values.push(Box::new(TextValue {
                    value: commit.message().summary().to_string(),
                }));
                continue;
            }

            if column_name == "message" {
                values.push(Box::new(TextValue {
                    value: commit.message.to_string(),
                }));
                continue;
            }

            if column_name == "datetime" {
                let time_stamp = commit_info
                    .commit_time
                    .unwrap_or_else(|| commit.time().map(|time| time.seconds).unwrap_or(0));
                values.push(Box::new(DateTimeValue { value: time_stamp }));
                continue;
            }

            if column_name == "parents_count" {
                values.push(Box::new(IntValue {
                    value: commit.parents.len() as i64,
                }));
                continue;
            }

            if column_name == "repo" {
                values.push(Box::new(TextValue {
                    value: repo_path.clone(),
                }));
                continue;
            }

            values.push(Box::new(NullValue));
        }

        let row = Row { values };
        rows.push(row);
    }

    Ok(rows)
}

fn select_branches(
    repo: &gix::Repository,
    selected_columns: &[String],
) -> Result<Vec<Row>, String> {
    let mut rows: Vec<Row> = vec![];

    let repo_path = repo.path().to_string_lossy().into_owned();
    let platform = repo.references().map_err(|err| err.to_string())?;
    let local_branches = platform.local_branches().map_err(|err| err.to_string())?;
    let remote_branches = platform.remote_branches().map_err(|err| err.to_string())?;
    let local_and_remote_branches = local_branches.chain(remote_branches);
    let head_ref = match repo.head_ref().map_err(|err| err.to_string())? {
        Some(head_ref) => head_ref,
        None => return Ok(rows),
    };

    for mut branch in local_and_remote_branches.flatten() {
        let mut values: Vec<Box<dyn Value>> = Vec::with_capacity(selected_columns.len());

        for column_name in selected_columns {
            if column_name == "name" {
                let branch_name = branch.name().as_bstr().to_string();
                values.push(Box::new(TextValue { value: branch_name }));
                continue;
            }

            if column_name == "commit_count" {
                let commit_count = branch
                    .try_id()
                    .and_then(|id| id.ancestors().all().ok())
                    .map_or(-1, |revwalk| revwalk.count() as i64);

                values.push(Box::new(IntValue {
                    value: commit_count,
                }));
                continue;
            }

            if column_name == "updated" {
                if let Ok(id) = branch.peel_to_id() {
                    if let Ok(revwalk) = id.ancestors().all() {
                        if let Some(Ok(commit_info)) = revwalk.into_iter().next() {
                            if let Some(commit_timestamp) = commit_info.commit_time {
                                values.push(Box::new(DateTimeValue {
                                    value: commit_timestamp,
                                }));
                                continue;
                            }

                            if let Ok(commit) = commit_info.object() {
                                if let Ok(commit) = commit.decode() {
                                    let time_stamp = commit.time().map(|time| time.seconds).unwrap_or(0);
                                    values.push(Box::new(DateTimeValue { value: time_stamp }));
                                    continue;
                                }
                            }
                        }
                    }
                }

                values.push(Box::new(NullValue));
                continue;
            }

            if column_name == "is_head" {
                values.push(Box::new(BoolValue {
                    value: branch.inner == head_ref.inner,
                }));
                continue;
            }

            if column_name == "is_remote" {
                let is_remote = matches!(branch.name().category(), Some(Category::RemoteBranch));
                values.push(Box::new(BoolValue { value: is_remote }));
                continue;
            }

            if column_name == "repo" {
                values.push(Box::new(TextValue {
                    value: repo_path.clone(),
                }));
                continue;
            }

            values.push(Box::new(NullValue));
        }

        let row = Row { values };
        rows.push(row);
    }

    Ok(rows)
}

fn select_diffs(repo: &gix::Repository, selected_columns: &[String]) -> Result<Vec<Row>, String> {
    let mut repo = repo.clone();
    repo.object_cache_size_if_unset(4 * 1024 * 1024);

    let revwalk = repo.head_id().map_err(|err| err.to_string())?.ancestors().all().map_err(|err| err.to_string())?;
    let repo_path = repo.path().to_string_lossy().into_owned();

    let mut rewrite_cache = repo
        .diff_resource_cache(gix::diff::blob::pipeline::Mode::ToGit, Default::default())
        .map_err(|err| err.to_string())?;

    let mut diff_cache = rewrite_cache.clone();
    let mut rows: Vec<Row> = vec![];

    let select_insertions_or_deletions = selected_columns
        .iter()
        .any(|column| column == "insertions" || column == "deletions");

    for commit_info in revwalk {
        let commit_info = commit_info.map_err(|err| err.to_string())?;
        let commit = repo
            .find_object(commit_info.id)
            .map_err(|err| err.to_string())?
            .into_commit();
        let commit_ref = commit.decode().map_err(|err| err.to_string())?;
        let mut values: Vec<Box<dyn Value>> = Vec::with_capacity(selected_columns.len());

        for column_name in selected_columns {
            if column_name == "commit_id" {
                values.push(Box::new(TextValue {
                    value: commit_info.id.to_string(),
                }));
                continue;
            }

            if column_name == "name" {
                let author_name = commit_ref
                    .author()
                    .map(|author| author.name.to_string())
                    .unwrap_or_default();
                values.push(Box::new(TextValue { value: author_name }));
                continue;
            }

            if column_name == "email" {
                let author_email = commit_ref
                    .author()
                    .map(|author| author.email.to_string())
                    .unwrap_or_default();
                values.push(Box::new(TextValue { value: author_email }));
                continue;
            }

            if column_name == "datetime" {
                let time_stamp = commit_info
                    .commit_time
                    .unwrap_or_else(|| commit_ref.time().map(|time| time.seconds).unwrap_or(0));
                values.push(Box::new(DateTimeValue { value: time_stamp }));
                continue;
            }

            if column_name == "repo" {
                values.push(Box::new(TextValue {
                    value: repo_path.clone(),
                }));
                continue;
            }

            if column_name == "insertions"
                || column_name == "deletions"
                || column_name == "files_changed"
            {
                let current = commit.tree().map_err(|err| err.to_string())?;

                let previous = commit_info
                    .parent_ids()
                    .next()
                    .map(|id| {
                        repo.find_object(id)
                            .map_err(|err| err.to_string())
                            .and_then(|obj| obj.into_commit().tree().map_err(|err| err.to_string()))
                    })
                    .transpose()
                    .map_err(|err| err.to_string())?
                    .unwrap_or_else(|| repo.empty_tree());
                rewrite_cache.clear_resource_cache();
                diff_cache.clear_resource_cache();

                let (mut insertions, mut deletions, mut files_changed) = (0, 0, 0);
                previous
                    .changes()
                    .map_err(|err| err.to_string())?
                    .for_each_to_obtain_tree_with_cache(
                        &current,
                        &mut rewrite_cache,
                        |change| -> Result<_, Box<gix::object::blob::diff::init::Error>> {
                            files_changed += usize::from(change.entry_mode().is_no_tree());
                            if select_insertions_or_deletions {
                                if let Ok(mut platform) = change.diff(&mut diff_cache) {
                                    if let Ok(Some(counts)) = platform.line_counts() {
                                        deletions += counts.removals;
                                        insertions += counts.insertions;
                                    }
                                }
                            }
                            Ok(std::ops::ControlFlow::Continue(()))
                        },
                    )
                    .map_err(|err| err.to_string())?;

                if column_name == "insertions" {
                    values.push(Box::new(IntValue {
                        value: insertions as i64,
                    }));
                    continue;
                }

                if column_name == "deletions" {
                    values.push(Box::new(IntValue {
                        value: deletions as i64,
                    }));
                    continue;
                }

                if column_name == "files_changed" {
                    values.push(Box::new(IntValue {
                        value: files_changed as i64,
                    }));
                    continue;
                }
            }

            values.push(Box::new(NullValue));
        }

        let row = Row { values };
        rows.push(row);
    }

    Ok(rows)
}

fn select_tags(repo: &gix::Repository, selected_columns: &[String]) -> Result<Vec<Row>, String> {
    let platform = repo.references().map_err(|err| err.to_string())?;
    let tag_names = platform.tags().map_err(|err| err.to_string())?;
    let repo_path = repo.path().to_string_lossy().into_owned();
    let mut rows: Vec<Row> = vec![];
    for tag_ref in tag_names.flatten() {
        let mut values: Vec<Box<dyn Value>> = Vec::with_capacity(selected_columns.len());

        for column_name in selected_columns {
            if column_name == "name" {
                let tag_name = tag_ref
                    .name()
                    .category_and_short_name()
                    .map_or_else(String::default, |(_, short_name)| short_name.to_string());
                values.push(Box::new(TextValue {
                    value: tag_name.to_string(),
                }));
                continue;
            }

            if column_name == "repo" {
                values.push(Box::new(TextValue {
                    value: repo_path.clone(),
                }));
                continue;
            }

            values.push(Box::new(NullValue));
        }

        let row = Row { values };
        rows.push(row);
    }

    Ok(rows)
}
