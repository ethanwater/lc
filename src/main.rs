use std::io::BufRead;
use std::path::Path;
use std::{fs, process};

macro_rules! ternary {
    ($test:expr => $true_expr:expr; $false_expr:expr) => {
        if $test {
            $true_expr
        } else {
            $false_expr
        }
    };
}

trait Visible {
    fn is_visible(&self) -> Option<()>;
}

impl Visible for Path {
    fn is_visible(&self) -> Option<()> {
        let filename = self.file_name()?.to_str()?;
        ternary!(filename.starts_with(".") => return None; return Some(()));
    }
}

const WIDTH: usize = 50;

fn ignore() -> Vec<String> {
    let contents = fs::read(".gitignore").unwrap();
    let mut ignored: Vec<String> = Vec::new();

    for line in contents.lines() {
        let mut ignore_value = Some(line).unwrap().unwrap();
        let prefix = ignore_value.chars().nth(0);
        if prefix.unwrap() == '/' {
            ignore_value.remove(0);
        }
        ignored.push(ignore_value);
    }

    ignored
}

fn linecount_abridged<P>(directory_path: P) -> std::io::Result<u128>
where
    P: AsRef<Path>,
{
    let mut total_lines: u128 = 0;

    for entry in fs::read_dir(directory_path)? {
        let entry = entry?.path();
        let path = entry.as_path();
        let metadata = fs::metadata(path)?.file_type();

        if metadata.is_file() && path.is_visible().is_some() {
            let content = String::from_utf8_lossy(&fs::read(&path)?).into_owned();
            for _ in content.lines() {
                total_lines += 1;
            }
        } else if metadata.is_dir() && path.is_visible().is_some() {
            let _linecount_result = linecount_abridged(Path::new(&path));
            let linecount = match _linecount_result {
                Ok(success) => success,
                Err(err) => panic!("shit!{err}"),
            };
            total_lines += linecount;
        };
    }
    Ok(total_lines)
}

fn linecount_verbose<P>(directory_path: P, mut indent_amount: Option<usize>) -> std::io::Result<u128>
where
    P: AsRef<Path>,
{
    let mut total_lines: u128 = 0;
    let path = directory_path.as_ref().as_os_str().to_str().unwrap();
    ternary!(indent_amount.is_none() => indent_amount = Some(2); indent_amount = indent_amount);

    let indent = " ".repeat(indent_amount.unwrap());
    let file_indent = " ".repeat(indent_amount.unwrap()+2);
    println!("{indent}{path}/");

    let entries = fs::read_dir(directory_path)
        .expect("Failed to read directory")
        .map(|entry| entry.unwrap().path())
        .collect::<Vec<_>>();
    let mut files = Vec::new();
    let mut dirs = Vec::new();

    for entry in entries {
        if entry.is_file() {
            files.push(entry);
        } else {
            dirs.push(entry);
        }
    }
    files.sort();
    dirs.sort();
    let sorted_entries = files.iter().chain(dirs.iter());

    for entry in sorted_entries {
        let path = entry.as_path();
        let metadata = fs::metadata(path)?;
        let filetype = metadata.file_type();
        //let bytesize = metadata.len();

        let mut target_total_lines: u128 = 0;
        let file_entry = entry.file_name().unwrap().to_str().unwrap_or("?");

        if filetype.is_file() && path.is_visible().is_some() {
            let content = String::from_utf8_lossy(&fs::read(&path)?).into_owned();
            for _ in content.lines() {
                total_lines += 1;
                target_total_lines += 1;
            }
            println!("{file_indent}{}", format!("{:width$} {}", file_entry, target_total_lines,  width=WIDTH));
        } else if filetype.is_dir() && path.is_visible().is_some() {
            let _linecount_result = linecount_verbose(Path::new(&path), Some(indent_amount.unwrap() + 2));
            let linecount = match _linecount_result {
                Ok(success) => success,
                Err(err) => panic!("shit!{err}"),
            };
            total_lines += linecount;
        };
    }
    Ok(total_lines)
}

fn linecount_verbose_ignore<P>(directory_path: P) -> std::io::Result<u128>
where
    P: AsRef<Path>,
{
    let mut total_lines: u128 = 0;
    let gitignore = ignore();

    for entry in fs::read_dir(directory_path)? {
        let entry = entry?.path();
        let path = entry.as_path();
        let metadata = fs::metadata(path)?.file_type();
        let mut target_total_lines: u128 = 0;

        if metadata.is_file() && path.is_visible().is_some() {
            let content = String::from_utf8_lossy(&fs::read(&path)?).into_owned();
            for _ in content.lines() {
                total_lines += 1;
                target_total_lines += 1;
            }
            let file_entry = entry.file_name().unwrap().to_str().unwrap_or("?");
            println!("{:<5}", format!("{:width$} {}", file_entry, target_total_lines, width=WIDTH));
        } else if metadata.is_dir() && path.is_visible().is_some() {
            if entry.file_name().unwrap().to_str().unwrap() == gitignore[0] {
                continue
            }
            let file_entry = entry.file_name().unwrap().to_str().unwrap();
            println!("\n{file_entry}/");
            let _linecount_result = linecount_verbose_ignore(Path::new(&path));
            let linecount = match _linecount_result {
                Ok(success) => success,
                Err(err) => panic!("shit!{err}"),
            };
            total_lines += linecount;
        };
    }
    Ok(total_lines)
}


fn fetch_directory() -> std::io::Result<String> {
    let output = process::Command::new("pwd").output()?;
    let mut current_dir = String::from_utf8_lossy(&output.stdout).into_owned();
    current_dir.pop();

    return Ok(current_dir);
}

fn main() -> std::io::Result<()> {
    let result = linecount_verbose(Path::new(&fetch_directory().unwrap()), None)?;
    println!("\ntotal: {result}");

    ignore();
    Ok(())
}
