//! Explicitly opted-in local converters. No shell, inherited credentials or network provider.
use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const MAX_TOOL_BYTES: u64 = 32 * 1024 * 1024;
const MAX_OCR_PAGES: usize = 20;

pub fn convert(
    path: &Path,
    format: &str,
    attempts: &mut Vec<String>,
) -> Result<(String, &'static str), String> {
    let path = path.canonicalize().map_err(|error| error.to_string())?;
    let deadline = Instant::now() + Duration::from_secs(90);
    match format {
        "doc" => {
            attempts.push("antiword".to_string());
            run("antiword", &[path.as_os_str()], deadline).map(|text| (text, "antiword"))
        }
        "pdf" => {
            attempts.push("pdftotext".to_string());
            let text = run(
                "pdftotext",
                &[
                    OsStr::new("-layout"),
                    OsStr::new("-enc"),
                    OsStr::new("UTF-8"),
                    path.as_os_str(),
                    OsStr::new("-"),
                ],
                deadline,
            )
            .map(|text| text.replace('\u{c}', "\n\n"));
            if let Ok(text) = &text {
                let mut report = super::ExtractionReport::new("pdf");
                if !text.chars().any(super::quality::suspicious_character)
                    && super::quality::assess_text(text, &mut report).is_ok()
                {
                    return Ok((text.clone(), "pdftotext"));
                }
            }
            ocr_pdf(&path, deadline, attempts).map(|text| (text, "pdftoppm+tesseract"))
        }
        "png" | "jpg" | "jpeg" | "tif" | "tiff" | "bmp" | "webp" => {
            ocr_image(&path, deadline, attempts).map(|text| (text, "tesseract"))
        }
        _ => Err("No local converter is available for this format.".to_string()),
    }
}

fn ocr_image(path: &Path, deadline: Instant, attempts: &mut Vec<String>) -> Result<String, String> {
    attempts.push("tesseract".to_string());
    run(
        "tesseract",
        &[path.as_os_str(), OsStr::new("stdout")],
        deadline,
    )
}

fn ocr_pdf(path: &Path, deadline: Instant, attempts: &mut Vec<String>) -> Result<String, String> {
    attempts.push("pdfinfo".to_string());
    let info = run("pdfinfo", &[path.as_os_str()], deadline)?;
    let pages = info
        .lines()
        .find_map(|line| line.strip_prefix("Pages:")?.trim().parse::<usize>().ok())
        .ok_or_else(|| "Cannot determine the PDF page count for bounded OCR.".to_string())?;
    if pages == 0 || pages > MAX_OCR_PAGES {
        return Err(format!("Local PDF OCR supports 1–{MAX_OCR_PAGES} pages per file; this file has {pages}. Split or export it first."));
    }
    let directory = tempfile::tempdir().map_err(|error| error.to_string())?;
    let prefix = directory.path().join("page");
    let mut sections = Vec::new();
    let mut bytes = 0;
    for page in 1..=pages {
        let number = OsString::from(page.to_string());
        attempts.push("pdftoppm".to_string());
        run(
            "pdftoppm",
            &[
                OsStr::new("-f"),
                &number,
                OsStr::new("-l"),
                &number,
                OsStr::new("-singlefile"),
                OsStr::new("-scale-to"),
                OsStr::new("2000"),
                OsStr::new("-png"),
                path.as_os_str(),
                prefix.as_os_str(),
            ],
            deadline,
        )?;
        let text = ocr_image(&prefix.with_extension("png"), deadline, attempts)?;
        if text.trim().is_empty() {
            return Err(format!(
                "OCR produced no text for page {page}; no partial PDF was imported."
            ));
        }
        bytes += text.len();
        if bytes > MAX_TOOL_BYTES as usize {
            return Err("OCR output exceeds the extraction limit.".to_string());
        }
        sections.push(format!("## Page {page}\n\n{}", text.trim()));
    }
    Ok(sections.join("\n\n"))
}

fn run(program: &str, args: &[&OsStr], deadline: Instant) -> Result<String, String> {
    let output = tempfile::NamedTempFile::new().map_err(|error| error.to_string())?;
    let mut child = Command::new(program)
        .args(args)
        .env_clear()
        .env("PATH", "/usr/local/bin:/usr/bin:/bin:/opt/homebrew/bin")
        .env("LC_ALL", "C")
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .stdout(output.reopen().map_err(|error| error.to_string())?)
        .spawn()
        .map_err(|_| {
            format!("Cannot start {program}. Install the local converter or import a text export.")
        })?;
    let status = loop {
        let oversized = output
            .as_file()
            .metadata()
            .map(|m| m.len() > MAX_TOOL_BYTES)
            .unwrap_or(true);
        if Instant::now() >= deadline || oversized {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!(
                "{program} exceeded the local extraction time or output limit."
            ));
        }
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("Cannot wait for {program}: {error}"));
            }
        }
    };
    if !status.success() {
        return Err(format!("{program} could not extract this file. Check that the original is readable and the converter supports its format."));
    }
    let mut text = String::new();
    File::open(output.path())
        .map_err(|error| error.to_string())?
        .take(MAX_TOOL_BYTES + 1)
        .read_to_string(&mut text)
        .map_err(|error| format!("{program} did not return UTF-8 text: {error}"))?;
    if text.len() > MAX_TOOL_BYTES as usize {
        return Err(format!("{program} output exceeds the extraction limit."));
    }
    Ok(text)
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn local_converter_timeout_is_bounded_and_missing_tools_are_actionable() {
        let error = run(
            "sleep",
            &[OsStr::new("10")],
            Instant::now() + Duration::from_millis(30),
        )
        .unwrap_err();
        assert!(error.contains("limit"));
        let error = run(
            "cowiki-nonexistent-converter",
            &[],
            Instant::now() + Duration::from_secs(1),
        )
        .unwrap_err();
        assert!(error.contains("Install"));
    }

    #[test]
    fn converter_arguments_are_not_interpreted_by_a_shell() {
        let text = run(
            "printf",
            &[
                OsStr::new("%s"),
                OsStr::new("literal; $(touch never-created)"),
            ],
            Instant::now() + Duration::from_secs(1),
        )
        .unwrap();
        assert_eq!(text, "literal; $(touch never-created)");
    }
}
