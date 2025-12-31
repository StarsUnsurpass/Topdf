use genpdf::fonts::FontData;
use std::fs;
use std::path::Path;
use anyhow::{Context, Result};
use genpdf::{elements, style, Element};
use pulldown_cmark::{Parser, Event, Tag, TagEnd, HeadingLevel};
use serde_json::Value;
use std::io::Read;
use zip::ZipArchive;
use std::sync::Arc;

#[derive(Debug, Clone, Copy)]
pub enum FileType {
    Markdown,
    Json,
    Xml,
    Txt,
    Docx,
    Html,
    Csv,
    Image,
    Unknown,
}

impl FileType {
    pub fn from_path(path: &Path) -> Self {
        match path.extension().and_then(|s| s.to_str()).map(|s| s.to_lowercase()).as_deref() {
            Some("md") | Some("markdown") => FileType::Markdown,
            Some("json") => FileType::Json,
            Some("xml") => FileType::Xml,
            Some("txt") | Some("rs") | Some("py") | Some("js") | Some("c") | Some("cpp") => FileType::Txt,
            Some("docx") => FileType::Docx,
            Some("html") | Some("htm") => FileType::Html,
            Some("csv") => FileType::Csv,
            Some("png") | Some("jpg") | Some("jpeg") | Some("bmp") => FileType::Image,
            _ => FileType::Unknown,
        }
    }
}

pub fn prepare_font(font_data: Arc<Vec<u8>>) -> Result<Arc<FontData>> {
    let font = FontData::new(font_data.as_ref().clone(), None)
        .context("Failed to parse font data")?;
    Ok(Arc::new(font))
}

pub fn convert(input: &Path, output: &Path, font: Arc<FontData>) -> Result<()> {
    log::info!("Starting conversion for: {:?}", input);
    let file_type = FileType::from_path(input);
    
    let content = match file_type {
        FileType::Image => String::new(), 
        FileType::Docx => read_docx(input)?,
        FileType::Csv => String::new(), 
        _ => fs::read_to_string(input).context("Failed to read file")?,
    };
    log::info!("File type identified as: {:?}. Content loaded.", file_type);

    let font_family = genpdf::fonts::FontFamily {
        regular: font.as_ref().clone(),
        bold: font.as_ref().clone(),
        italic: font.as_ref().clone(),
        bold_italic: font.as_ref().clone(),
    };

    log::debug!("Creating PDF document structure");
    let mut doc = genpdf::Document::new(font_family);
    doc.set_title("Converted Document");
    doc.set_minimal_conformance();
    doc.set_line_spacing(1.2);
    
    let mut decorator = genpdf::SimplePageDecorator::new();
    decorator.set_margins(10);
    doc.set_page_decorator(decorator);

    log::debug!("Rendering content to document");
    match file_type {
        FileType::Markdown => render_markdown(&content, &mut doc),
        FileType::Json => render_json(&content, &mut doc)?,
        FileType::Xml => render_xml(&content, &mut doc)?,
        FileType::Txt | FileType::Docx => render_text(&content, &mut doc),
        FileType::Html => render_html(&content, &mut doc),
        FileType::Csv => render_csv(input, &mut doc)?,
        FileType::Image => render_image(input, &mut doc)?,
        FileType::Unknown => {
            let msg = "Unknown file type";
            log::error!("{}", msg);
            return Err(anyhow::anyhow!(msg));
        }
    }

    log::info!("Rendering PDF to file {:?}", output);
    doc.render_to_file(output).context("Failed to render PDF")?;
    log::info!("Conversion complete for {:?}", input);
    Ok(())
}

fn read_docx(path: &Path) -> Result<String> {
    log::debug!("Reading DOCX file: {:?}", path);
    let file = fs::File::open(path)?;
    let mut archive = ZipArchive::new(file)?;
    let mut document_xml = archive.by_name("word/document.xml")?;
    let mut content = String::new();
    document_xml.read_to_string(&mut content)?;

    let mut text = String::new();
    let doc = roxmltree::Document::parse(&content)?;
    
    for node in doc.descendants() {
         if node.has_tag_name("p") {
             for child in node.descendants() {
                 if child.has_tag_name("t") {
                     if let Some(t) = child.text() {
                         text.push_str(t);
                     }
                 }
             }
             text.push('\n');
         }
    }
    Ok(text)
}

fn render_text(content: &str, doc: &mut genpdf::Document) {
    for line in content.lines() {
        doc.push(elements::Paragraph::new(line));
    }
}

fn render_json(content: &str, doc: &mut genpdf::Document) -> Result<()> {
    let v: Value = serde_json::from_str(content).unwrap_or(Value::Null);
    let pretty = if v.is_null() { content.to_string() } else { serde_json::to_string_pretty(&v)? };
    
    doc.push(elements::Paragraph::new("JSON Content:").styled(style::Style::new().bold()));
    doc.push(elements::Break::new(1.0));
    for line in pretty.lines() {
        doc.push(elements::Paragraph::new(line).styled(style::Style::new().with_font_size(10)));
    }
    Ok(())
}

fn render_xml(content: &str, doc: &mut genpdf::Document) -> Result<()> {
    doc.push(elements::Paragraph::new("XML Content:").styled(style::Style::new().bold()));
    doc.push(elements::Break::new(1.0));
     for line in content.lines() {
        doc.push(elements::Paragraph::new(line).styled(style::Style::new().with_font_size(10)));
    }
    Ok(())
}

fn render_html(content: &str, doc: &mut genpdf::Document) {
    doc.push(elements::Paragraph::new("HTML Content:").styled(style::Style::new().bold()));
    doc.push(elements::Break::new(1.0));
    if let Ok(text) = html2text::from_read(content.as_bytes(), 80) {
        render_text(&text, doc);
    } else {
        log::warn!("Failed to parse HTML content");
        doc.push(elements::Paragraph::new("Failed to parse HTML").styled(style::Style::new().with_color(style::Color::Rgb(255, 0, 0))));
    }
}

fn render_csv(path: &Path, doc: &mut genpdf::Document) -> Result<()> {
    let mut reader = csv::Reader::from_path(path)?;
    doc.push(elements::Paragraph::new("CSV Content:").styled(style::Style::new().bold()));
    doc.push(elements::Break::new(1.0));

    if let Ok(headers) = reader.headers() {
        let header_line = headers.iter().collect::<Vec<&str>>().join(" | ");
        doc.push(elements::Paragraph::new(header_line).styled(style::Style::new().bold()));
    }
    
    for result in reader.records() {
        if let Ok(record) = result {
             let line = record.iter().collect::<Vec<&str>>().join(" | ");
             doc.push(elements::Paragraph::new(line).styled(style::Style::new().with_font_size(10)));
        }
    }
    Ok(())
}

fn render_image(path: &Path, doc: &mut genpdf::Document) -> Result<()> {
    match elements::Image::from_path(path) {
        Ok(img) => {
             doc.push(img);
        },
        Err(e) => {
             log::error!("Error loading image {}: {}", path.display(), e);
             doc.push(elements::Paragraph::new(format!("Error loading image: {}", e)));
        }
    }
    Ok(())
}

fn render_markdown(content: &str, doc: &mut genpdf::Document) {
    let parser = Parser::new(content);
    
    let mut current_text = String::new();

    for event in parser {
        match event {
            Event::Text(text) => current_text.push_str(&text),
            Event::SoftBreak => current_text.push(' '),
            Event::HardBreak => current_text.push('\n'),
            Event::Start(Tag::Paragraph) => {
                current_text.clear();
            },
            Event::End(TagEnd::Paragraph) => {
                if !current_text.is_empty() {
                    doc.push(elements::Paragraph::new(&current_text));
                    doc.push(elements::Break::new(0.5));
                }
                current_text.clear();
            },
            Event::Start(Tag::Heading{..}) => {
                 current_text.clear();
            },
            Event::End(TagEnd::Heading(level)) => {
                 let size = match level {
                     HeadingLevel::H1 => 20,
                     HeadingLevel::H2 => 18,
                     _ => 14,
                 };
                 doc.push(elements::Paragraph::new(&current_text).styled(style::Style::new().with_font_size(size).bold()));
                 doc.push(elements::Break::new(0.5));
                 current_text.clear();
            },
            Event::Code(text) => {
                 current_text.push_str(&format!(" {} ", text));
            },
            Event::Start(Tag::CodeBlock(_)) => {
                current_text.clear();
            },
            Event::End(TagEnd::CodeBlock) => {
                 for line in current_text.lines() {
                    doc.push(elements::Paragraph::new(line).styled(style::Style::new().with_font_size(10)));
                 }
                 doc.push(elements::Break::new(0.5));
                 current_text.clear();
            }
             _ => {}
        }
    }
    if !current_text.is_empty() {
        doc.push(elements::Paragraph::new(&current_text));
    }
}
