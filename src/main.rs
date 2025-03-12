#![allow(dead_code)]

use clap::{Arg, ArgAction, Command};
use colored::Colorize;
use std::collections::HashSet;
use std::io::Result;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use std::{env, fs};

const WIDTH: usize = 20;
const FILENAME_RENDER_LIMIT: usize = 60;

enum ContentType {
    CODE,
    MEDIA,
    EXECUTABLE,
    NORMAL,
    TEXT,
    LICENSE,
    MAKEFILE,
}

lazy_static::lazy_static! {
    static ref CODE_EXTENSIONS: HashSet<&'static str> = [
        "c", "h", "cpp", "hpp", "cc", "cxx", "hh", "hxx", "cs", "java", "class", "jar", "kt", "kts",
        "js", "jsx", "mjs", "cjs", "ts", "tsx", "py", "pyc", "pyd", "pyo", "rb", "erb",
        "php", "phar", "go", "rs", "rlib", "swift", "dart", "scala", "lua", "r", "pl", "pm", "sql",
        "html", "htm", "xhtml", "xml", "css", "scss", "sass", "json", "yaml", "yml", "toml",
        "env", "ini", "cfg", "md", "rst", "cmake", "mk", "dockerfile", "dockerignore",
        "gitignore", "gitattributes"
    ].iter().copied().collect();

    static ref MEDIA_EXTENSIONS: HashSet<&'static str> = [
        "png", "jpg", "jpeg", "gif", "bmp", "tiff", "tif", "webp", "svg", "ico", "heic", "avif",
        "mp3", "wav", "flac", "aac", "ogg", "opus", "m4a", "wma", "aiff", "alac", "amr",
        "mp4", "mkv", "avi", "mov", "wmv", "flv", "webm", "m4v", "mpeg", "mpg", "3gp", "ogv"
    ].iter().copied().collect();

    static ref EXECUTABLE_EXTENSIONS: HashSet<&'static str> = [
        "exe", "bat", "cmd", "msi", "run", "out", "bin", "app", "jar", "sh", "bash", "zsh",
        "ps1", "psm1", "psd1"
    ].iter().copied().collect();

    static ref TEXT_EXTENSIONS: HashSet<&'static str> = [
        "txt", "md", "rtf", "csv", "log", "pdf", "doc", "docx", "odt", "tex", "pages"
    ].iter().copied().collect();
}

trait Content {
    fn content_type(&self) -> ContentType;
}

impl Content for Path {
    fn content_type(&self) -> ContentType {
        if let Some(ext) = self.extension().and_then(|s| s.to_str()) {
            if CODE_EXTENSIONS.contains(ext) {
                return ContentType::CODE;
            }
            if MEDIA_EXTENSIONS.contains(ext) {
                return ContentType::MEDIA;
            }
            if EXECUTABLE_EXTENSIONS.contains(ext)
                || (self.is_unix_executable().unwrap_or(false) && !TEXT_EXTENSIONS.contains(ext))
            {
                return ContentType::EXECUTABLE;
            }
            if TEXT_EXTENSIONS.contains(ext) {
                return ContentType::TEXT;
            }
        }

        match self.file_name().and_then(|s| s.to_str()) {
            Some("LICENSE") => ContentType::LICENSE,
            Some("Makefile") => ContentType::MAKEFILE,
            _ => ContentType::NORMAL,
        }
    }
}

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

trait UnixExecutable {
    fn is_unix_executable(&self) -> Result<bool>;
}

impl UnixExecutable for Path {
    fn is_unix_executable(&self) -> Result<bool> {
        let metadata = self.metadata()?;

        Ok(metadata.permissions().mode() & 0o111 != 0)
    }
}

fn fetch_gitignore(path: &Path) -> Result<Vec<String>> {
    let gitignore = path.join(".gitignore");
    if !gitignore.exists() {
        return Ok(Vec::new());
    }

    let contents = fs::read_to_string(gitignore)?;
    let mut list_to_ignore: Vec<String> = Vec::new();

    for line in contents.lines() {
        let mut value = line.to_string();
        if value.starts_with("/") {
            value.remove(0);
        }
        list_to_ignore.push(value);
    }

    Ok(list_to_ignore)
}

fn linecount_async(dir: Option<PathBuf>) -> Result<(u128, u128)> {
    let total_lines = Arc::new(Mutex::new(0));
    let total_bytes = Arc::new(Mutex::new(0));
    let dir_path_binding = dir.unwrap_or(env::current_dir()?);
    let dir_path = dir_path_binding.as_path();
    //let ignore_vec = fetch_gitignore(&dir_path)?;
    let mut handles = Vec::new();

    let entries = fs::read_dir(dir_path)
        .expect("Failed to read directory")
        .map(|entry| entry.unwrap().path())
        .collect::<Vec<_>>();

    for entry in entries {
        let path = entry.as_path();
        let filetype = fs::metadata(path)?.file_type();

        if filetype.is_file() {
            let content = fs::read(&path)?; // Read the raw bytes
            let content_len = content.len() as u128;
            let content_str = std::str::from_utf8(&content).unwrap_or("");

            //let content = String::from_utf8_lossy(&fs::read(&path)?).into_owned();
            let file_linecount = content_str.lines().count() as u128;
            let file_bytes = content_len;

            *total_lines.lock().unwrap() += file_linecount;
            *total_bytes.lock().unwrap() += file_bytes;
        } else if filetype.is_dir() {
            let handle = {
                let total_lines = Arc::clone(&total_lines);
                let total_bytes = Arc::clone(&total_bytes);
                let path = PathBuf::from(path);

                thread::spawn(move || {
                    let recursive_lc = linecount_async(Some(path));

                    if let Ok((lines, bytes)) = recursive_lc {
                        *total_lines.lock().unwrap() += lines;
                        *total_bytes.lock().unwrap() += bytes;
                    }
                })
            };
            handles.push(handle);
        }
    }
    for handle in handles {
        handle.join().unwrap();
    }

    Ok(get_totals(total_lines, total_bytes))
}

fn linecount_display(
    dir: Option<PathBuf>,
    mut indent_amount: Option<usize>,
) -> Result<(u128, u128)> {
    let (mut total_lines, mut total_bytes) = (0, 0);
    let dir_path_binding = dir.unwrap_or(env::current_dir()?);
    let dir_path = dir_path_binding.as_path();
    let mut file_indent_from_zero_size = indent_amount.unwrap_or_default();
    //let ignore_vec = fetch_gitignore(&dir_path)?;

    if indent_amount.is_none() {
        indent_amount = Some(0);
    } else if indent_amount.unwrap() > 0 {
        file_indent_from_zero_size += 1;
    }

    let (dir_indent, file_indent_from_dir, file_ident_from_zero) = (
        "─".repeat(indent_amount.unwrap_or_default()),
        "─".repeat(2),
        " ".repeat(file_indent_from_zero_size),
    );
    let dir_path_str = dir_path
        .file_name()
        .unwrap()
        .to_str()
        .unwrap_or_default()
        .blue()
        .bold();

    match indent_amount {
        Some(0) => println!("{dir_indent}{dir_path_str}/"),
        _ => println!("├{dir_indent}{dir_path_str}/"),
    }

    let entries = fs::read_dir(dir_path)
        .expect("Failed to read directory")
        .map(|entry| entry.unwrap().path())
        .collect::<Vec<_>>();
    let (mut files, mut dirs) = (Vec::new(), Vec::new());

    for entry in entries {
        //if ignore_toggle {
        //    if ignore_vec.contains(&entry.file_name().unwrap().to_string_lossy().to_string()) {
        //        continue;
        //    }
        //}

        if entry.is_file() {
            files.push(entry);
        } else {
            dirs.push(entry);
        }
    }
    files.sort();
    dirs.sort();
    let sorted_entries = files.iter().chain(dirs.iter());

    for (idx, entry) in sorted_entries.enumerate() {
        let mut connector = "├";
        let path = entry.as_path();
        let filetype = fs::metadata(path)?.file_type();

        if filetype.is_file() {
            let content = String::from_utf8_lossy(&fs::read(&path)?).into_owned();
            let file_linecount = content.lines().count() as u128;
            let file_bytes = content.as_bytes().len() as u128;

            total_lines += file_linecount;
            total_bytes += file_bytes;

            let filename = entry
                .file_name()
                .unwrap()
                .to_str()
                .unwrap_or("?")
                .to_string();

            let filename = if filename.len() > FILENAME_RENDER_LIMIT {
                format!("{}...", &filename[..FILENAME_RENDER_LIMIT])
            } else {
                filename
            };

            //if last file in head/sub-directory
            if idx == files.len() - 1 {
                connector = "└";
            }

            let formatted_indent: String = match indent_amount {
                Some(0) => format!("{file_ident_from_zero}{connector}{file_indent_from_dir}"),
                _ => format!("|{file_ident_from_zero}{connector}{file_indent_from_dir}"),
            };

            let formatted_output = format!(
                "{:width$} ({}L, {}B)",
                {
                    match path.content_type() {
                        ContentType::MEDIA => filename.bright_magenta().to_string(),
                        ContentType::CODE => filename.cyan().to_string(),
                        ContentType::EXECUTABLE => filename.green().to_string(),
                        ContentType::TEXT => filename.truecolor(217, 50, 122).to_string(),
                        ContentType::LICENSE => filename.truecolor(0, 0, 255).to_string(),
                        ContentType::MAKEFILE => filename.red().to_string(),
                        _ => filename.to_string(),
                    }
                },
                file_linecount,
                file_bytes,
                width = WIDTH
            );
            println!("{formatted_indent}{formatted_output}");
        } else if filetype.is_dir() {
            if let Ok((lines, bytes)) = linecount_display(
                Some(PathBuf::from(&path)),
                Some(indent_amount.unwrap_or_default() + 2),
            ) {
                total_lines += lines;
                total_bytes += bytes;
            }
        };
    }
    Ok((total_lines, total_bytes))
}

//EXPERIMENTAL: runs linecount_display via paralellization. has significant increase in speed.
//   -BUGS: since the function operates in parrell, printing the treemap is unreliable since order is not guaranteed.
//          because of this the output looks scattered and disorganized.
//
//
fn linecount_display_async(
    dir: Option<PathBuf>,
    mut indent_amount: Option<usize>,
) -> Result<(u128, u128)> {
    let total_lines = Arc::new(Mutex::new(0));
    let total_bytes = Arc::new(Mutex::new(0));
    let dir_path_binding = dir.unwrap_or(env::current_dir()?);
    let dir_path = dir_path_binding.as_path();
    let mut file_indent_from_zero_size = indent_amount.unwrap_or_default();
    //let ignore_vec = fetch_gitignore(&dir_path)?;
    let mut handles = Vec::new();

    if indent_amount.is_none() {
        indent_amount = Some(0);
    } else if indent_amount.unwrap() > 0 {
        file_indent_from_zero_size += 1;
    }

    let (dir_indent, file_indent_from_dir, file_ident_from_zero) = (
        "─".repeat(indent_amount.unwrap_or_default()),
        "─".repeat(2),
        " ".repeat(file_indent_from_zero_size),
    );
    let dir_path_str = dir_path
        .file_name()
        .unwrap()
        .to_str()
        .unwrap_or_default()
        .blue()
        .bold();

    match indent_amount {
        Some(0) => println!("{dir_indent}{dir_path_str}/"),
        _ => println!("├{dir_indent}{dir_path_str}/"),
    }

    let entries = fs::read_dir(dir_path)
        .expect("Failed to read directory")
        .map(|entry| entry.unwrap().path())
        .collect::<Vec<_>>();
    let (mut files, mut dirs) = (Vec::new(), Vec::new());

    for entry in entries {
        //if ignore_toggle {
        //    if ignore_vec.contains(&entry.file_name().unwrap().to_string_lossy().to_string()) {
        //        continue;
        //    }
        //}

        if entry.is_file() {
            files.push(entry);
        } else {
            dirs.push(entry);
        }
    }
    files.sort();
    dirs.sort();
    let sorted_entries = files.iter().chain(dirs.iter());

    for (idx, entry) in sorted_entries.enumerate() {
        let mut connector = "├";
        let path = entry.as_path();
        let filetype = fs::metadata(path)?.file_type();

        if filetype.is_file() {
            let content = String::from_utf8_lossy(&fs::read(&path)?).into_owned();
            let file_linecount = content.lines().count() as u128;
            let file_bytes = content.as_bytes().len() as u128;

            *total_lines.lock().unwrap() += file_linecount;
            *total_bytes.lock().unwrap() += file_bytes;

            let filename = entry
                .file_name()
                .unwrap()
                .to_str()
                .unwrap_or("?")
                .to_string();

            let filename = if filename.len() > FILENAME_RENDER_LIMIT {
                format!("{}...", &filename[..FILENAME_RENDER_LIMIT])
            } else {
                filename
            };
            if idx == files.len() - 1 {
                connector = "└";
            }

            let formatted_indent = match indent_amount {
                Some(0) => format!("{file_ident_from_zero}{connector}{file_indent_from_dir}"),
                _ => format!("│{file_ident_from_zero}{connector}{file_indent_from_dir}"),
            };

            let formatted_output = format!(
                "{:width$} ({}L, {}B)",
                {
                    match path.content_type() {
                        ContentType::MEDIA => filename.bright_magenta().to_string(),
                        ContentType::CODE => filename.cyan().to_string(),
                        ContentType::EXECUTABLE => filename.green().to_string(),
                        ContentType::TEXT => filename.truecolor(217, 50, 122).to_string(),
                        ContentType::LICENSE => filename.truecolor(0, 0, 255).to_string(),
                        ContentType::MAKEFILE => filename.red().to_string(),
                        _ => filename.to_string(),
                    }
                },
                file_linecount,
                file_bytes,
                width = WIDTH
            );
            println!("{formatted_indent}{formatted_output}");
        } else if filetype.is_dir() {
            let handle = {
                let total_lines = Arc::clone(&total_lines);
                let total_bytes = Arc::clone(&total_bytes);
                let path = PathBuf::from(path);

                thread::spawn(move || {
                    let recursive_lc =
                        linecount_display_async(Some(path), Some(indent_amount.unwrap() + 2));

                    if let Ok((lines, bytes)) = recursive_lc {
                        *total_lines.lock().unwrap() += lines;
                        *total_bytes.lock().unwrap() += bytes;
                    }
                })
            };
            handles.push(handle);
        }
    }
    for handle in handles {
        handle.join().unwrap();
    }

    Ok(get_totals(total_lines, total_bytes))
}

fn get_totals(total_lines: Arc<Mutex<u128>>, total_bytes: Arc<Mutex<u128>>) -> (u128, u128) {
    let lines = total_lines.lock().unwrap();
    let bytes = total_bytes.lock().unwrap();
    (*lines, *bytes)
}

fn format_byte_count(byte_count: u128) -> String {
    if byte_count / 1_000_000_000 > 1 {
        return format!("{} GB", byte_count as f64 / 1_000_000_000.);
    } else if byte_count / 1_000_000 > 1 {
        return format!("{} MB", byte_count as f64 / 1_000_000.);
    } else if byte_count / 1_000 > 1 {
        return format!("{} KB", byte_count as f64 / 1_000.);
    } else {
        return format!("{} B", byte_count);
    }
}

fn format_and_print_results(lines: u128, bytes: u128, time: Duration) {
    let f_bytes = format_byte_count(bytes);
    println!("╭───────────────────────────────────────────────────╮");
    println!(
        "│{:<51}│\n│{:<51}│\n│{:<51}│",
        format!("Lines       :{lines}"),
        format!("Bytes       :{f_bytes}"),
        format!("Time Taken  :{:.5} Seconds", time.as_secs_f64())
    );
    println!("╰───────────────────────────────────────────────────╯")
}

fn main() -> std::io::Result<()> {
    let calls = Command::new("lc")
        .version("1.2")
        .author("Ethan Water")
        .about("Line Counting Program")
        .args([
            Arg::new("path")
                .short('p')
                .long("path")
                .action(ArgAction::Set)
                .value_name("PATH")
                .help("Provides a path to lc"),
            Arg::new("display")
                .short('d')
                .long("display")
                .action(ArgAction::SetTrue)
                .help("Displays the filetree search"),
        ])
        .get_matches();

    let path = calls.get_one::<String>("path").map(PathBuf::from);

    if *calls.get_one::<bool>("display").unwrap_or(&false) {
        let start_time = Instant::now();
        let (lines, bytes) = linecount_display(path, None)?;
        let end_time = Instant::now();
        format_and_print_results(lines, bytes, end_time - start_time);
    } else {
        let start_time = Instant::now();
        let (lines, bytes) = linecount_async(path)?;
        let end_time = Instant::now();
        format_and_print_results(lines, bytes, end_time - start_time);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::linecount_async;
    use std::time::Instant;

    const TEST_ITERATIONS: u128 = 1000;

    #[test]
    fn get_average_execution_time() {
        let mut total_execution_time: f64 = 0.;
        let mut iteration = 0;
        let mut t_bytes = 0;

        while iteration < TEST_ITERATIONS {
            let start_time = Instant::now();
            let (_lines, bytes) = linecount_async(None).unwrap();
            let end_time = Instant::now();

            t_bytes += bytes;
            total_execution_time += (end_time - start_time).as_secs_f64();
            iteration += 1;
        }

        let avg_bytes = t_bytes / TEST_ITERATIONS;
        let avg_execution_time = total_execution_time / TEST_ITERATIONS as f64;
        let mbs = avg_bytes as f64 / (avg_execution_time * 1_048_576.);

        println!("Average MB/S:                         {:.7}mb/s", mbs);
        println!(
            "Average Execution Time Per Iteration: {}",
            avg_execution_time
        );
        println!(
            "Total Execution Time:                 {}",
            total_execution_time
        );
    }
}
