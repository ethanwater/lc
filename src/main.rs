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

fn linecount<P>(directory_path: P) -> std::io::Result<u128>
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
            let _linecount_result = linecount(Path::new(&path));
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
    let result = linecount(Path::new(&fetch_directory().unwrap()))?;
    println!("linecount: {result}");
    Ok(())
}
