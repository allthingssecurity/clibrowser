use std::path::Path;
use anyhow::Result;
use crate::cli::WriteArgs;
use crate::error::OfficeError;
use crate::format::detect_format;
use crate::models::WriteResult;
use crate::output::OutputConfig;

pub fn execute(args: WriteArgs, out: &OutputConfig) -> Result<i32> {
    let path = Path::new(&args.file);
    let source_path = Path::new(&args.from);

    if !source_path.exists() {
        return Err(OfficeError::FileNotFound(args.from.clone()).into());
    }

    let kind = detect_format(path, None)?;
    let source_content = std::fs::read_to_string(source_path)?;

    match kind {
        crate::format::FormatKind::Docx => write_docx(path, &source_content, &args, out),
        crate::format::FormatKind::Xlsx => write_xlsx(path, &source_content, &args, out),
        crate::format::FormatKind::Pdf => write_pdf(path, &source_content, &args, out),
        _ => Err(OfficeError::NotSupported {
            op: "write".into(),
            format: kind.name().into(),
        }.into()),
    }
}

fn write_docx(path: &Path, content: &str, args: &WriteArgs, out: &OutputConfig) -> Result<i32> {
    use docx_rs::*;

    let mut doc = Docx::new();

    // Add title if provided
    if let Some(ref title) = args.title {
        doc = doc.add_paragraph(
            Paragraph::new()
                .add_run(Run::new().add_text(title).bold().size(48))
        );
    }

    // Convert content to paragraphs
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            doc = doc.add_paragraph(Paragraph::new());
        } else if trimmed.starts_with("# ") {
            doc = doc.add_paragraph(
                Paragraph::new()
                    .add_run(Run::new().add_text(&trimmed[2..]).bold().size(36))
            );
        } else if trimmed.starts_with("## ") {
            doc = doc.add_paragraph(
                Paragraph::new()
                    .add_run(Run::new().add_text(&trimmed[3..]).bold().size(28))
            );
        } else if trimmed.starts_with("### ") {
            doc = doc.add_paragraph(
                Paragraph::new()
                    .add_run(Run::new().add_text(&trimmed[4..]).bold().size(24))
            );
        } else if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            doc = doc.add_paragraph(
                Paragraph::new()
                    .add_run(Run::new().add_text(&format!("  \u{2022} {}", &trimmed[2..])))
            );
        } else {
            doc = doc.add_paragraph(
                Paragraph::new().add_run(Run::new().add_text(trimmed))
            );
        }
    }

    let file = std::fs::File::create(path)?;
    doc.build().pack(file)?;

    let result = WriteResult {
        file: path.display().to_string(),
        format: "docx".into(),
        message: format!("Created document from {}", args.from),
    };
    if out.json { out.print_json(&result); } else { out.print_human(&result.message); }
    Ok(0)
}

fn write_xlsx(path: &Path, content: &str, args: &WriteArgs, out: &OutputConfig) -> Result<i32> {
    let mut workbook = rust_xlsxwriter::Workbook::new();
    let worksheet = workbook.add_worksheet();

    // Check if source is CSV
    if args.from.ends_with(".csv") {
        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(false)
            .from_reader(content.as_bytes());

        for (row_idx, result) in rdr.records().enumerate() {
            let record = result?;
            for (col_idx, field) in record.iter().enumerate() {
                // Try to parse as number first
                if let Ok(num) = field.parse::<f64>() {
                    worksheet.write_number(row_idx as u32, col_idx as u16, num)?;
                } else {
                    worksheet.write_string(row_idx as u32, col_idx as u16, field)?;
                }
            }
        }
    } else {
        // Plain text: one line per row
        for (row_idx, line) in content.lines().enumerate() {
            worksheet.write_string(row_idx as u32, 0, line)?;
        }
    }

    workbook.save(path)?;

    let result = WriteResult {
        file: path.display().to_string(),
        format: "xlsx".into(),
        message: format!("Created spreadsheet from {}", args.from),
    };
    if out.json { out.print_json(&result); } else { out.print_human(&result.message); }
    Ok(0)
}

fn write_pdf(path: &Path, content: &str, args: &WriteArgs, out: &OutputConfig) -> Result<i32> {
    use printpdf::*;

    let title = args.title.as_deref().unwrap_or("Document");
    let mut doc = PdfDocument::new(title);

    let font = PdfFontHandle::Builtin(BuiltinFont::Helvetica);

    let mut ops = Vec::new();
    ops.push(Op::StartTextSection);
    ops.push(Op::SetFont { font, size: Pt(11.0) });
    ops.push(Op::SetLineHeight { lh: Pt(14.0) });
    ops.push(Op::SetTextCursor { pos: Point { x: Mm(20.0).into(), y: Mm(280.0).into() } });

    for line in content.lines() {
        ops.push(Op::ShowText { items: vec![TextItem::Text(line.to_string())] });
        ops.push(Op::AddLineBreak);
    }

    ops.push(Op::EndTextSection);

    let page = PdfPage::new(Mm(210.0), Mm(297.0), ops);
    doc.pages.push(page);

    let mut warnings = Vec::new();
    let bytes = doc.save(&PdfSaveOptions::default(), &mut warnings);
    std::fs::write(path, bytes)?;

    let result = WriteResult {
        file: path.display().to_string(),
        format: "pdf".into(),
        message: format!("Created PDF from {}", args.from),
    };
    if out.json { out.print_json(&result); } else { out.print_human(&result.message); }
    Ok(0)
}
