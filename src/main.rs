use clap::{App, Arg, ArgAction};
use std::io::Result;
use std::path::{Path, PathBuf};
use std::time::Instant;
use std::{env, fs};

const WIDTH: usize = 50;

trait Visible {
    fn is_visible(&self) -> bool;
}

impl Visible for Path {
    fn is_visible(&self) -> bool {
        //this may be a bit much. but it works well.
        if self.file_name().unwrap().to_str().unwrap().chars().nth(0) == Some('.') {
            return false;
        }
        true
    }
}

//trait Ignore {
//    fn ignore(&self, gitignore: &Vec<String>) -> bool;
//}
//
//impl Ignore for Path {
//    fn ignore(&self, gitignore: &Vec<String>) -> bool {
//        for i in gitignore {
//            if self.file_name().unwrap().to_str().unwrap() == i {
//                return true;
//            }
//        }
//        false
//    }
//}
//
//
//fn detect_gitignore(path: &Path) -> Result<Vec<String>> {
//    let gitignore = path.join(".gitignore");
//    if !gitignore.exists() {
//        return Err(Error::new(ErrorKind::Other, ".gitignore not found"));
//    }
//
//    let contents: Vec<u8> = fs::read(gitignore).unwrap_or(Vec::new());
//    let mut list_to_ignore: Vec<String> = Vec::new();
//
//    for line in contents.lines() {
//        let mut value = line.unwrap_or_default();
//        if value.starts_with("/") {
//            value.remove(0);
//        }
//        list_to_ignore.push(value);
//    }
//
//    Ok(list_to_ignore)
//}

fn linecount(dir: Option<PathBuf>, byte_toggle: bool) -> Result<(u128, u128)> {
    let (mut total_lines, mut total_bytes) = (0, 0);

    let dir_path_binding = dir.unwrap_or(env::current_dir()?);
    let dir_path = dir_path_binding.as_path();

    let directory_entries = fs::read_dir(dir_path)?;
    for entry in directory_entries {
        let entry = entry?.path();
        let path = entry.as_path();

        if path.is_visible() {
            let metadata = fs::metadata(path)?.file_type();
            if metadata.is_file() {
                let content = String::from_utf8_lossy(&fs::read(&entry)?).into_owned();
                total_lines += content.lines().count() as u128;
                if byte_toggle {
                    total_bytes += content.as_bytes().len() as u128;
                }
                continue;
            }

            if metadata.is_dir() {
                let clone_entry = entry.clone();
                let _linecount_result = linecount(Some(entry).clone(), byte_toggle);
                let linecount = match _linecount_result {
                    Ok(success) => success,
                    Err(err) => {
                        eprintln!("{err}: skipping {:?}", Some(clone_entry));
                        continue;
                    }
                };
                total_lines += linecount.0;
                continue;
            };
        }
    }
    Ok((total_lines, total_bytes))
}

fn linecount_verbose(
    dir: Option<PathBuf>,
    byte_toggle: bool,
    mut indent_amount: Option<usize>,
) -> Result<(u128, u128)> {
    let (mut total_lines, mut total_bytes) = (0, 0);
    let dir_path_binding = dir.unwrap_or(env::current_dir()?);
    let dir_path = dir_path_binding.as_path();
    let mut file_indent_from_zero_size = indent_amount.unwrap_or_default();

    if indent_amount.is_none() {
        indent_amount = Some(0);
    } else if indent_amount.unwrap() > 0 {
        file_indent_from_zero_size += 1;
    }

    let (dir_indent, file_indent_from_dir, file_ident_from_zero) = (
        " ".repeat(indent_amount.unwrap_or_default()),
        "─".repeat(2),
        " ".repeat(file_indent_from_zero_size),
    );
    let dir_path_str = dir_path.file_name().unwrap().to_str().unwrap_or_default();

    println!("{dir_indent}{dir_path_str}/");

    let entries = fs::read_dir(dir_path)
        .expect("Failed to read directory")
        .map(|entry| entry.unwrap().path())
        .collect::<Vec<_>>();
    let (mut files, mut dirs) = (Vec::new(), Vec::new());

    for entry in entries {
        if entry.is_visible() {
            if entry.is_file() {
                files.push(entry);
            } else {
                dirs.push(entry);
            }
        }
    }
    files.sort();
    dirs.sort();
    let sorted_entries = files.iter().chain(dirs.iter());

    for (idx, entry) in sorted_entries.enumerate() {
        let mut connector = "├";
        let path = entry.as_path();
        let filetype = fs::metadata(path)?.file_type();
        let filename = entry.file_name().unwrap().to_str().unwrap_or("?");

        if path.is_visible() {
            if filetype.is_file() {
                let content = String::from_utf8_lossy(&fs::read(&path)?).into_owned();
                let file_linecount = content.lines().count() as u128;
                total_lines += file_linecount;

                let mut file_bytes: u128 = 0;
                if byte_toggle {
                    file_bytes = content.as_bytes().len() as u128;
                    total_bytes += file_bytes;
                }

                if idx == files.len() - 1 {
                    connector = "└";
                }

                let formatted_indent =
                    format!("{dir_indent}{file_ident_from_zero}{connector}{file_indent_from_dir}");
                let formatted_output = match byte_toggle {
                    true => format!(
                        "{:width$} ({}L, {}B)",
                        filename,
                        file_linecount,
                        file_bytes,
                        width = WIDTH
                    ),
                    false => format!(
                        "{:width$} (lines: {})",
                        filename,
                        file_linecount,
                        width = WIDTH
                    ),
                };

                println!("{formatted_indent}{formatted_output}");
            } else if filetype.is_dir() {
                let recursive_lc = linecount_verbose(
                    Some(PathBuf::from(&path)),
                    byte_toggle,
                    Some(indent_amount.unwrap() + 2),
                );
                if recursive_lc.is_err() {
                    continue;
                }
                total_lines += recursive_lc.as_ref().unwrap().0;
                if byte_toggle {
                    total_bytes += recursive_lc.as_ref().unwrap().1;
                }
            };
        }
    }
    Ok((total_lines, total_bytes))
}

fn main() -> std::io::Result<()> {
    let calls = App::new("lc")
        .version("1.1")
        .author("Ethan Water")
        .about("Line Counting Program")
        .arg(
            Arg::new("path")
                .short('p')
                .long("path")
                .action(ArgAction::Set)
                .value_name("PATH")
                .help("Provides a path to lc"),
        )
        .arg(Arg::new("verbose").short('v').long("verbose"))
        .arg(Arg::new("bytes").short('b').long("bytes"))
        .get_matches();

    let path = calls.get_one::<String>("path").map(PathBuf::from);

    if calls.is_present("verbose") && calls.is_present("bytes") {
        let start_time = Instant::now();
        let result = linecount_verbose(path, true, None)?;
        let end_time = Instant::now();
        let (lines, bytes) = result;
        println!(
            "[lines]       {lines}\n[bytes]       {bytes}\n[time]        {:?}",
            end_time - start_time
        );
    } else if calls.is_present("verbose") && !calls.is_present("bytes") {
        let start_time = Instant::now();
        let result = linecount_verbose(path, false, None)?;
        let end_time = Instant::now();
        let (lines, _bytes) = result;
        println!(
            "[lines]       {lines}\n[time]        {:?}",
            end_time - start_time
        );
    } else if calls.is_present("bytes") {
        let start_time = Instant::now();
        let result = linecount(path, true)?;
        let end_time = Instant::now();
        let (lines, bytes) = result;
        println!(
            "[lines]       {lines}\n[bytes]       {bytes}\n[time]        {:?}",
            end_time - start_time
        );
    } else {
        let start_time = Instant::now();
        let result = linecount(path, false)?;
        let end_time = Instant::now();
        let (lines, _bytes) = result;
        println!(
            "[lines]       {lines}\n[time]        {:?}",
            end_time - start_time
        );
    }
    Ok(())
}
