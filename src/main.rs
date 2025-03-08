use clap::{App, Arg, ArgAction};
use std::io::Result;
use std::path::{Path, PathBuf};
use std::time::Instant;
use std::{env, fs};

const WIDTH: usize = 20;
const FILENAME_RENDER_LIMIT: usize = 60;

enum ContentType {
    CODE,       //LIGHT BLUE
    MEDIA,      //IMAGE, AUDIO, VIDEO PURPLE
    EXECUTABLE, //RED
    NORMAL,
    TEXT,
    LICENSE,
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

trait Content {
    fn content_type(&self) -> ContentType;
}

impl Content for Path {
    fn content_type(&self) -> ContentType {
        let code_extensions = vec![
            // General programming languages
            "c",
            "h",
            "cpp",
            "hpp",
            "cc",
            "cxx",
            "hh",
            "hxx", // C/C++
            "cs",
            "java",
            "class",
            "jar",
            "kt",
            "kts", // C#, Java, Kotlin
            "js",
            "jsx",
            "mjs",
            "cjs",
            "ts",
            "tsx", // JavaScript, TypeScript
            "py",
            "pyc",
            "pyd",
            "pyo",
            "rb",
            "erb", // Python, Ruby
            "php",
            "phar",
            "go",
            "rs",
            "rlib",
            "swift",
            "dart", // PHP, Go, Rust, Swift, Dart
            "scala",
            "lua",
            "r",
            "pl",
            "pm",
            "sql", // Scala, Lua, R, Perl, SQL
            // Scripting & Shell
            "sh",
            "bash",
            "zsh",
            "bat",
            "cmd", // Shell & Batch
            "ps1",
            "psm1",
            "psd1", // PowerShell
            // Web & Stylesheets
            "html",
            "htm",
            "xhtml",
            "xml",
            "css",
            "scss",
            "sass",
            // Config & Data
            "json",
            "yaml",
            "yml",
            "toml",
            "env",
            "ini",
            "cfg",
            // Documentation & Build
            "md",
            "rst",
            "makefile",
            "cmake",
            "mk",
            // DevOps & Version Control
            "dockerfile",
            "dockerignore",
            "gitignore",
            "gitattributes",
        ];

        let media_extensions = vec![
            // Image extensions
            "png", "jpg", "jpeg", "gif", "bmp", "tiff", "tif", "webp", "svg", "ico", "heic", "avif",
            // Audio extensions
            "mp3", "wav", "flac", "aac", "ogg", "opus", "m4a", "wma", "aiff", "alac", "amr",
            // Video extensions
            "mp4", "mkv", "avi", "mov", "wmv", "flv", "webm", "m4v", "mpeg", "mpg", "3gp", "ogv",
        ];

        let executable_extensions = vec![
            // Windows executables
            "exe", "bat", "cmd", "msi", // Unix/Linux/macOS binary executables
            "run", "out", "bin", "app", // Java executables
            "jar",
        ];

        let text_extensions = vec![
            // Plain text formats
            "txt", "md", "rtf", "csv", "log", // Documentation formats (including PDF)
            "pdf", "doc", "docx", "odt", "tex", "pages",
        ];

        if code_extensions.contains(&self.extension().unwrap_or_default().to_str().unwrap()) {
            return ContentType::CODE;
        } else if media_extensions.contains(&self.extension().unwrap_or_default().to_str().unwrap())
        {
            return ContentType::MEDIA;
        } else if executable_extensions
            .contains(&self.extension().unwrap_or_default().to_str().unwrap())
        {
            return ContentType::EXECUTABLE;
        } else if text_extensions.contains(&self.extension().unwrap_or_default().to_str().unwrap())
        {
            return ContentType::TEXT;
        } else if self.file_name().unwrap() == "LICENSE" {
            return ContentType::LICENSE;
        } else {
            return ContentType::NORMAL;
        }
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
                total_bytes += linecount.1;
                continue;
            };
        }
    }
    Ok((total_lines, total_bytes))
}

use colored::Colorize;

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

                let formatted_indent = match indent_amount {
                    Some(0) => format!("{file_ident_from_zero}{connector}{file_indent_from_dir}"),
                    _ => format!("|{file_ident_from_zero}{connector}{file_indent_from_dir}"),
                };

                let formatted_output = match byte_toggle {
                    true => format!(
                        "{:width$} ({}L, {}B)",
                        {
                            match path.content_type() {
                                ContentType::MEDIA => filename.magenta().to_string(),
                                ContentType::CODE => filename.cyan().to_string(),
                                ContentType::EXECUTABLE => filename.green().to_string(),
                                ContentType::TEXT => filename.yellow().to_string(),
                                ContentType::LICENSE => filename.red().to_string(),
                                _ => filename.to_string(),
                            }
                        },
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
        let f_bytes = format_byte_count(bytes);
        println!(
            "[lines]       {lines}\n[bytes]       {f_bytes}\n[time]        {:?}",
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
        let f_bytes = format_byte_count(bytes);
        println!(
            "[lines]       {lines}L\n[bytes]       {f_bytes}\n[time]        {:?}",
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
