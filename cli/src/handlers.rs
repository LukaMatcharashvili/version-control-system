use crate::utils::*;
use std::fs;
use std::fs::File;
use std::io::Write;

pub fn init() -> std::io::Result<()> {
    let history_path = ".history".to_owned();
    let git_path = ".git".to_owned();
    if fs::read_dir(git_path.clone()).is_ok() || fs::read_dir(history_path.clone()).is_ok() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            "Already initialized",
        ));
    }
    fs::create_dir(&history_path)?;
    File::create(history_path + "/commits.json")?.write_all(b"[]")?;

    Ok(())
}

pub fn commit(description: &str) -> std::io::Result<()> {
    check_if_initialized()?;

    let history_path = ".history".to_owned();
    let commit_id: String = generate_commit_id();
    let file_paths = traverse_directory(None);
    let formatted_date = get_current_formatted_date();

    add_commit(
        &(history_path.clone() + "/commits.json"),
        Commit {
            date: formatted_date.clone(),
            description: description.to_owned(),
            commit_id: commit_id.clone(),
        },
    )?;

    for file_path in file_paths {
        let file_name_for_history = file_path.clone().to_str().unwrap().replace("/", "_");
        let last_committed_file_path = history_path.clone() + &"/" + file_name_for_history.as_str();
        let file_contents = fs::read_to_string(file_path.clone())?;

        let file_metadata_string_result: Result<Vec<CommitMetadata>, std::io::Error> =
            file_metadata(&last_committed_file_path);
        let mut file_metadata = match file_metadata_string_result {
            Ok(res) => res,
            Err(_) => {
                fs::create_dir(&last_committed_file_path)?;
                write_to_metadata_file(
                    &last_committed_file_path,
                    vec![CommitMetadata {
                        date: formatted_date.clone(),
                        description: description.to_owned(),
                        commit_id: commit_id.clone(),
                        pointer_to_data: 0,
                        size: file_contents.len() as i32,
                    }],
                )?;

                write_to_data_file(&last_committed_file_path, &file_contents, false)?;
                continue;
            }
        };
        let last_committed_metadata = file_metadata.last().unwrap();
        let last_committed_file_pointer = last_committed_metadata.pointer_to_data;
        let last_committed_file_size = last_committed_metadata.size;

        let last_committed_file_contents = read_part_of_file(
            &(last_committed_file_path.clone() + "/data.bin"),
            last_committed_file_pointer as u64,
            last_committed_file_size as usize,
        )?;

        if !compare_strings(&last_committed_file_contents, &file_contents) {
            file_metadata.push(CommitMetadata {
                date: formatted_date.clone(),
                description: description.to_owned(),
                commit_id: commit_id.clone(),
                pointer_to_data: last_committed_file_pointer + last_committed_file_size,
                size: file_contents.len() as i32,
            });

            write_to_metadata_file(&last_committed_file_path, file_metadata)?;
            write_to_data_file(&last_committed_file_path, &file_contents, true)?;
        }
    }

    Ok(())
}

pub fn commits() -> std::io::Result<()> {
    check_if_initialized()?;
    let history_path = ".history".to_owned();
    let commits_string_result = fs::read_to_string(history_path + "/commits.json")?;
    let commits = serde_json::from_str::<Vec<Commit>>(&commits_string_result).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Failed to parse metadata: {}", e),
        )
    })?;

    for commit in commits {
        println!(
            "{} {} {}",
            commit.date, commit.commit_id, commit.description
        );
    }

    Ok(())
}

pub fn view(branch_id: &str) -> std::io::Result<()> {
    check_if_initialized()?;
    delete_contents_of_directory(".")?;
    let history_path = ".history".to_owned();

    if let Ok(entries) = fs::read_dir(history_path.clone()) {
        for entry in entries {
            if let Ok(entry) = entry {
                let entry_path = entry.path();
                if entry_path.is_file() {
                    continue;
                }
                let file_name = entry_path.file_name().unwrap().to_str().unwrap().to_owned();
                let file_name_for_history = file_name.replace("_", "/");
                let last_committed_file_path: String = history_path.clone() + &"/" + &file_name;

                let file_metadata = file_metadata(&last_committed_file_path)?;
                let target_commit_metadata_result =
                    find_metadata_by_commit_id(&file_metadata, branch_id);
                let target_commit_metadata = match target_commit_metadata_result {
                    None => file_metadata.last().unwrap().clone(),
                    _ => target_commit_metadata_result.unwrap(),
                };

                let last_committed_file_pointer = target_commit_metadata.pointer_to_data;
                let last_committed_file_size = target_commit_metadata.size;

                let last_committed_file_contents = read_part_of_file(
                    &(last_committed_file_path.clone() + "/data.bin"),
                    last_committed_file_pointer as u64,
                    last_committed_file_size as usize,
                )?;

                let mut file = File::create(file_name_for_history)?;
                file.write_all(last_committed_file_contents.as_bytes())?;
            }
        }
    }

    Ok(())
}