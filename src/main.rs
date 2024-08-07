use clap::{App, Arg};
use std::io::BufRead;
use std::path::Path;
use std::time::Instant;
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
    fn is_visible(&self) -> bool;
}

impl Visible for Path {
    fn is_visible(&self) -> bool {
        let filename = self.file_name().unwrap_or_default().to_str().unwrap_or_default();
        ternary!(filename.starts_with(".") => return false; return true);
    }
}

trait Ignore {
    fn ignore(&self, gitignore: Vec<String>) -> bool;
}

impl Ignore for Path {
    fn ignore(&self, gitignore: Vec<String>) -> bool {
        for i in gitignore {
            if self.file_name().unwrap().to_str().unwrap() == i {
                return true;
            }
        }
        false
    }
}

const WIDTH: usize = 50;

fn fetch_directory() -> std::io::Result<String> {
    let output = process::Command::new("pwd").output()?;
    let mut current_dir = String::from_utf8_lossy(&output.stdout).into_owned();
    current_dir.pop();

    return Ok(current_dir);
}

fn detect_gitignore(pathstr: &str) -> Vec<String> {
    let mut path = pathstr.to_string();
    path.push_str("/.gitignore");
    let contents: Vec<u8> = fs::read(path).unwrap_or(Vec::new());

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
    let mut total_linecount: u128 = 0;

    for entry in fs::read_dir(directory_path)? {
        let entry = entry?.path();
        let path = entry.as_path();
        let metadata = fs::metadata(path)?.file_type();

        if metadata.is_file() && path.is_visible() {
            let content = String::from_utf8_lossy(&fs::read(&path)?).into_owned();
            total_linecount = content.lines().count() as u128;
        } else if metadata.is_dir() && path.is_visible() {
            let _linecount_result = linecount_abridged(Path::new(&path));
            let linecount = match _linecount_result {
                Ok(success) => success,
                Err(err) => panic!("shit!{err}"),
            };
            total_linecount += linecount;
        };
    }
    Ok(total_linecount)
}

fn linecount_abridged_ignore(directory_path: &Path) -> std::io::Result<u128> {
    let mut total_linecount: u128 = 0;
    let dir = directory_path.as_os_str().to_str().unwrap();
    let gitignore = detect_gitignore(dir);

    for entry in fs::read_dir(directory_path)? {
        let entry = entry?.path();
        let path = entry.as_path();
        let metadata = fs::metadata(path)?.file_type();

        if path.ignore(gitignore.clone()) {
            continue;
        } else if metadata.is_file() && path.is_visible() {
            let content = String::from_utf8_lossy(&fs::read(&path)?).into_owned();
            for _ in content.lines() {
                total_linecount += 1;
            }
        } else if metadata.is_dir() && path.is_visible() {
            let _linecount_result = linecount_abridged_ignore(Path::new(&path));
            let linecount = match _linecount_result {
                Ok(success) => success,
                Err(err) => panic!("shit!{err}"),
            };
            total_linecount += linecount;
        };
    }
    Ok(total_linecount)
}

//these functions should also return the bytes or something
fn linecount_verbose<P>(
    directory_path: P,
    mut indent_amount: Option<usize>,
) -> std::io::Result<u128>
where
    P: AsRef<Path>,
{
    let dir_path = directory_path
        .as_ref()
        .as_os_str()
        .to_str()
        .unwrap_or("???");

    ternary!(indent_amount.is_none() => indent_amount = Some(0); indent_amount = indent_amount);
    let (dir_indent, file_indent) = (
        " ".repeat(indent_amount.unwrap_or_default()),
        " ".repeat(indent_amount.unwrap_or_default() + 2),
    );
    println!("{dir_indent}{dir_path}/");

    let entries = fs::read_dir(directory_path)
        .expect("Failed to read directory")
        .map(|entry| entry.unwrap().path())
        .collect::<Vec<_>>();
    let (mut files, mut dirs) = (Vec::new(), Vec::new());

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

    let mut total_linecount: u128 = 0;
    for entry in sorted_entries {
        let path = entry.as_path();
        let filetype = fs::metadata(path)?.file_type();
        let filename = entry.file_name().unwrap().to_str().unwrap_or("?");

        let mut file_linecount: u128 = 0;

        if filetype.is_file() && path.is_visible() {
            let content = String::from_utf8_lossy(&fs::read(&path)?).into_owned();
            total_linecount += content.lines().count() as u128;
            file_linecount += content.lines().count() as u128;
            println!(
                "{file_indent}{}",
                format!("{:width$} {}", filename, file_linecount, width = WIDTH)
            );
        } else if filetype.is_dir() && path.is_visible() {
            total_linecount +=
                linecount_verbose(Path::new(&path), Some(indent_amount.unwrap() + 2))
                    .expect("shit!");
        };
    }
    Ok(total_linecount)
}

fn linecount_verbose_ignore<P>(
    directory_path: P,
    mut indent_amount: Option<usize>,
) -> std::io::Result<u128>
where
    P: AsRef<Path>,
{
    let dir_path = directory_path
        .as_ref()
        .as_os_str()
        .to_str()
        .unwrap_or("???");

    ternary!(indent_amount.is_none() => indent_amount = Some(0); indent_amount = indent_amount);
    let (dir_indent, file_indent) = (
        " ".repeat(indent_amount.unwrap_or_default()),
        " ".repeat(indent_amount.unwrap_or_default() + 2),
    );
    println!("{dir_indent}{dir_path}/");
    let gitignore = detect_gitignore(dir_path);

    let entries = fs::read_dir(directory_path)
        .expect("Failed to read directory")
        .map(|entry| entry.unwrap().path())
        .collect::<Vec<_>>();
    let (mut files, mut dirs) = (Vec::new(), Vec::new());

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

    let mut total_linecount: u128 = 0;
    for entry in sorted_entries {
        let path = entry.as_path();
        let filetype = fs::metadata(path)?.file_type();
        let filename = entry.file_name().unwrap().to_str().unwrap_or("?");

        let mut file_linecount: u128 = 0;

        if path.ignore(gitignore.clone()){
            continue;
        } else if filetype.is_file() && path.is_visible() {
            let content = String::from_utf8_lossy(&fs::read(&path)?).into_owned();
            total_linecount += content.lines().count() as u128;
            file_linecount += content.lines().count() as u128;
            println!(
                "{file_indent}{}",
                format!("{:width$} {}", filename, file_linecount, width = WIDTH)
            );
        } else if filetype.is_dir() && path.is_visible() {
            total_linecount +=
                linecount_verbose(Path::new(&path), Some(indent_amount.unwrap() + 2))
                    .expect("shit!");
        };
    }
    Ok(total_linecount)
}

fn main() -> std::io::Result<()> {
    //let calls = App::new("lc")
    //    .version("1.0")
    //    .author("Ethan Water")
    //    .about("Line counting program")
    //    .arg(Arg::new("verbose").short('v').long("verbose"))
    //    .arg(Arg::new("ignore").short('i').long("ignore"))
    //    .get_matches();

    //if calls.is_present("verbose") && calls.is_present("ignore") {
    //    println!("[tree]");
    //    let start_execution = Instant::now();
    //    let result = linecount_verbose_ignore(Path::new(&fetch_directory().unwrap()), None)?;
    //    let end_execution = Instant::now();
    //    println!("\n[sum]   {result}");
    //    println!("[execution]   {:?}", end_execution - start_execution);
    //} else if calls.is_present("verbose") {
    //    println!("[tree]");
    //    let start_execution = Instant::now();
    //    let result = linecount_verbose(Path::new(&fetch_directory().unwrap()), None)?;
    //    let end_execution = Instant::now();
    //    println!("\n[sum]   {result}");
    //    println!("[execution]   {:?}", end_execution - start_execution);
    //} else if calls.is_present("ignore") {
    //    let result = linecount_abridged_ignore(Path::new(&fetch_directory().unwrap()))?;
    //    println!("{result}");
    //} else {
    //    let result = linecount_abridged(Path::new(&fetch_directory().unwrap()))?;
    //    println!("{result}");
    //}

    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verbose() {
        println!("[tree]");
        let start_execution = Instant::now();
        let result = linecount_verbose(Path::new(&fetch_directory().unwrap()), None).unwrap();
        let end_execution = Instant::now();
        println!("\n[sum]   {result}");
        println!("[execution]   {:?}", end_execution - start_execution);
    }
    #[test]
    fn verbose_ignore() -> std::io::Result<()> {
        println!("[tree]");
        let start_execution = Instant::now();
        let result = linecount_verbose_ignore(Path::new(&fetch_directory().unwrap()), None)?;
        let end_execution = Instant::now();
        println!("\n[sum]   {result}");
        println!("[execution]   {:?}", end_execution - start_execution);
        Ok(())
    }
    #[test]
    fn abridged_ignore() -> std::io::Result<()> {
        let result = linecount_abridged_ignore(Path::new(&fetch_directory().unwrap()))?;
        println!("{result}");
        Ok(())
    }
    #[test]
    fn abridged() -> std::io::Result<()> {
        let result = linecount_abridged(Path::new(&fetch_directory().unwrap()))?;
        println!("{result}");
        Ok(())
    }
}

