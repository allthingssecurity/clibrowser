use std::path::Path;
use anyhow::Result;
use crate::cli::CreateArgs;
use crate::error::OfficeError;
use crate::format::detect_format;
use crate::models::WriteResult;
use crate::output::OutputConfig;

pub fn execute(args: CreateArgs, out: &OutputConfig) -> Result<i32> {
    let path = Path::new(&args.file);
    let kind = detect_format(path, None)?;

    match kind {
        crate::format::FormatKind::Docx => create_docx(path, &args, out),
        crate::format::FormatKind::Xlsx => create_xlsx(path, &args, out),
        crate::format::FormatKind::Pptx => create_pptx(path, &args, out),
        crate::format::FormatKind::Pdf => create_pdf(path, &args, out),
        _ => Err(OfficeError::NotSupported {
            op: "create".into(),
            format: kind.name().into(),
        }.into()),
    }
}

fn create_docx(path: &Path, args: &CreateArgs, out: &OutputConfig) -> Result<i32> {
    use docx_rs::*;

    let mut doc = Docx::new();

    if let Some(ref title) = args.title {
        doc = doc.add_paragraph(
            Paragraph::new().add_run(
                Run::new().add_text(title).bold()
            )
        );
    }

    let file = std::fs::File::create(path)?;
    doc.build().pack(file)?;

    let result = WriteResult {
        file: path.display().to_string(),
        format: "docx".into(),
        message: "Created empty document".into(),
    };
    if out.json { out.print_json(&result); } else { out.print_human(&result.message); }
    Ok(0)
}

fn create_xlsx(path: &Path, args: &CreateArgs, out: &OutputConfig) -> Result<i32> {
    let mut workbook = rust_xlsxwriter::Workbook::new();
    let _worksheet = workbook.add_worksheet();
    workbook.save(path)?;

    let result = WriteResult {
        file: path.display().to_string(),
        format: "xlsx".into(),
        message: "Created empty spreadsheet".into(),
    };
    let _ = args;
    if out.json { out.print_json(&result); } else { out.print_human(&result.message); }
    Ok(0)
}

fn create_pptx(path: &Path, args: &CreateArgs, out: &OutputConfig) -> Result<i32> {
    // Create a minimal PPTX using the pptx crate
    // The pptx crate might have limited creation support,
    // so we create a minimal valid PPTX by writing the ZIP structure
    create_minimal_pptx(path)?;

    let result = WriteResult {
        file: path.display().to_string(),
        format: "pptx".into(),
        message: "Created empty presentation".into(),
    };
    let _ = args;
    if out.json { out.print_json(&result); } else { out.print_human(&result.message); }
    Ok(0)
}

fn create_pdf(path: &Path, args: &CreateArgs, out: &OutputConfig) -> Result<i32> {
    use printpdf::*;

    let mut doc = PdfDocument::new(args.title.as_deref().unwrap_or("Untitled"));
    let page = PdfPage::new(Mm(210.0), Mm(297.0), vec![]);
    doc.pages.push(page);

    let mut warnings = Vec::new();
    let bytes = doc.save(&PdfSaveOptions::default(), &mut warnings);
    std::fs::write(path, bytes)?;

    let result = WriteResult {
        file: path.display().to_string(),
        format: "pdf".into(),
        message: "Created empty PDF".into(),
    };
    if out.json { out.print_json(&result); } else { out.print_human(&result.message); }
    Ok(0)
}

fn create_minimal_pptx(path: &Path) -> Result<()> {
    use std::io::Write;

    let file = std::fs::File::create(path)?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    // [Content_Types].xml
    zip.start_file("[Content_Types].xml", options)?;
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="xml" ContentType="application/xml"/>
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>
  <Override PartName="/ppt/slides/slide1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>
  <Override PartName="/ppt/slideLayouts/slideLayout1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slideLayout+xml"/>
  <Override PartName="/ppt/slideMasters/slideMaster1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slideMaster+xml"/>
</Types>"#)?;

    // _rels/.rels
    zip.start_file("_rels/.rels", options)?;
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/>
</Relationships>"#)?;

    // ppt/presentation.xml
    zip.start_file("ppt/presentation.xml", options)?;
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <p:sldMasterIdLst><p:sldMasterId id="2147483648" r:id="rId1"/></p:sldMasterIdLst>
  <p:sldIdLst><p:sldId id="256" r:id="rId2"/></p:sldIdLst>
  <p:sldSz cx="9144000" cy="6858000" type="screen4x3"/>
  <p:notesSz cx="6858000" cy="9144000"/>
</p:presentation>"#)?;

    // ppt/_rels/presentation.xml.rels
    zip.start_file("ppt/_rels/presentation.xml.rels", options)?;
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="slideMasters/slideMaster1.xml"/>
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/>
</Relationships>"#)?;

    // ppt/slides/slide1.xml
    zip.start_file("ppt/slides/slide1.xml", options)?;
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/></p:spTree></p:cSld>
</p:sld>"#)?;

    // ppt/slides/_rels/slide1.xml.rels
    zip.start_file("ppt/slides/_rels/slide1.xml.rels", options)?;
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout" Target="../slideLayouts/slideLayout1.xml"/>
</Relationships>"#)?;

    // ppt/slideLayouts/slideLayout1.xml
    zip.start_file("ppt/slideLayouts/slideLayout1.xml", options)?;
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldLayout xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" type="blank">
  <p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/></p:spTree></p:cSld>
</p:sldLayout>"#)?;

    // ppt/slideLayouts/_rels/slideLayout1.xml.rels
    zip.start_file("ppt/slideLayouts/_rels/slideLayout1.xml.rels", options)?;
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="../slideMasters/slideMaster1.xml"/>
</Relationships>"#)?;

    // ppt/slideMasters/slideMaster1.xml
    zip.start_file("ppt/slideMasters/slideMaster1.xml", options)?;
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/></p:spTree></p:cSld>
  <p:sldLayoutIdLst><p:sldLayoutId id="2147483649" r:id="rId1"/></p:sldLayoutIdLst>
</p:sldMaster>"#)?;

    // ppt/slideMasters/_rels/slideMaster1.xml.rels
    zip.start_file("ppt/slideMasters/_rels/slideMaster1.xml.rels", options)?;
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout" Target="../slideLayouts/slideLayout1.xml"/>
</Relationships>"#)?;

    zip.finish()?;
    Ok(())
}
