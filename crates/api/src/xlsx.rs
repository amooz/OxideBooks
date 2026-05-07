// Minimal XLSX writer. Generates an uncompressed PKZIP archive containing the
// OOXML parts required by Excel / LibreOffice to open the file.

fn crc32(data: &[u8]) -> u32 {
    static TABLE: std::sync::OnceLock<[u32; 256]> = std::sync::OnceLock::new();
    let table = TABLE.get_or_init(|| {
        let mut t = [0u32; 256];
        for i in 0..256u32 {
            let mut c = i;
            for _ in 0..8 {
                c = if c & 1 != 0 {
                    0xedb88320 ^ (c >> 1)
                } else {
                    c >> 1
                };
            }
            t[i as usize] = c;
        }
        t
    });
    let mut crc = 0xffff_ffffu32;
    for &byte in data {
        crc = table[((crc ^ u32::from(byte)) & 0xff) as usize] ^ (crc >> 8);
    }
    crc ^ 0xffff_ffff
}

struct ZipEntry<'a> {
    name: &'a str,
    data: Vec<u8>,
}

fn build_zip(entries: &[ZipEntry]) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    let mut central: Vec<u8> = Vec::new();
    let mut offsets: Vec<u32> = Vec::new();

    for entry in entries {
        let offset = out.len() as u32;
        offsets.push(offset);
        let crc = crc32(&entry.data);
        let sz = entry.data.len() as u32;
        let name = entry.name.as_bytes();
        let nl = name.len() as u16;

        // local file header
        out.extend_from_slice(&0x04034b50u32.to_le_bytes());
        out.extend_from_slice(&20u16.to_le_bytes()); // version needed
        out.extend_from_slice(&0u16.to_le_bytes()); // flags
        out.extend_from_slice(&0u16.to_le_bytes()); // stored
        out.extend_from_slice(&0u16.to_le_bytes()); // mod time
        out.extend_from_slice(&0u16.to_le_bytes()); // mod date
        out.extend_from_slice(&crc.to_le_bytes());
        out.extend_from_slice(&sz.to_le_bytes());
        out.extend_from_slice(&sz.to_le_bytes());
        out.extend_from_slice(&nl.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes()); // extra len
        out.extend_from_slice(name);
        out.extend_from_slice(&entry.data);

        // central directory record
        central.extend_from_slice(&0x02014b50u32.to_le_bytes());
        central.extend_from_slice(&20u16.to_le_bytes()); // version made by
        central.extend_from_slice(&20u16.to_le_bytes()); // version needed
        central.extend_from_slice(&0u16.to_le_bytes()); // flags
        central.extend_from_slice(&0u16.to_le_bytes()); // stored
        central.extend_from_slice(&0u16.to_le_bytes()); // mod time
        central.extend_from_slice(&0u16.to_le_bytes()); // mod date
        central.extend_from_slice(&crc.to_le_bytes());
        central.extend_from_slice(&sz.to_le_bytes());
        central.extend_from_slice(&sz.to_le_bytes());
        central.extend_from_slice(&nl.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes()); // extra len
        central.extend_from_slice(&0u16.to_le_bytes()); // comment len
        central.extend_from_slice(&0u16.to_le_bytes()); // disk start
        central.extend_from_slice(&0u16.to_le_bytes()); // internal attrs
        central.extend_from_slice(&0u32.to_le_bytes()); // external attrs
        central.extend_from_slice(&offset.to_le_bytes()); // local hdr offset
        central.extend_from_slice(name);
    }

    let cd_offset = out.len() as u32;
    let cd_size = central.len() as u32;
    let n = entries.len() as u16;
    out.extend_from_slice(&central);

    // end-of-central-directory
    out.extend_from_slice(&0x06054b50u32.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes()); // disk num
    out.extend_from_slice(&0u16.to_le_bytes()); // cd start disk
    out.extend_from_slice(&n.to_le_bytes()); // entries this disk
    out.extend_from_slice(&n.to_le_bytes()); // total entries
    out.extend_from_slice(&cd_size.to_le_bytes());
    out.extend_from_slice(&cd_offset.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes()); // comment len
    out
}

fn xe(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn col_letter(mut col: usize) -> String {
    let mut s = String::new();
    loop {
        s.insert(0, (b'A' + (col % 26) as u8) as char);
        if col < 26 {
            break;
        }
        col = col / 26 - 1;
    }
    s
}

/// Build XLSX bytes from a sheet name and rows of (header → cell value).
/// `headers` controls column order. Values can be strings or numbers (JSON).
pub fn build_xlsx(sheet_name: &str, headers: &[&str], rows: &[Vec<String>]) -> Vec<u8> {
    let mut sheet = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<worksheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\">\
<sheetData>",
    );

    // header row
    sheet.push_str("<row r=\"1\">");
    for (c, h) in headers.iter().enumerate() {
        let cr = format!("{}1", col_letter(c));
        sheet.push_str(&format!(
            "<c r=\"{cr}\" t=\"inlineStr\"><is><t>{}</t></is></c>",
            xe(h)
        ));
    }
    sheet.push_str("</row>");

    // data rows
    for (ri, row) in rows.iter().enumerate() {
        let rn = ri + 2;
        sheet.push_str(&format!("<row r=\"{rn}\">"));
        for (c, val) in row.iter().enumerate() {
            let cr = format!("{}{rn}", col_letter(c));
            // Try to emit as a number if it parses cleanly
            if val.parse::<f64>().is_ok() {
                sheet.push_str(&format!("<c r=\"{cr}\"><v>{}</v></c>", xe(val)));
            } else {
                sheet.push_str(&format!(
                    "<c r=\"{cr}\" t=\"inlineStr\"><is><t>{}</t></is></c>",
                    xe(val)
                ));
            }
        }
        sheet.push_str("</row>");
    }
    sheet.push_str("</sheetData></worksheet>");

    let sn = xe(sheet_name);
    let entries = vec![
        ZipEntry {
            name: "[Content_Types].xml",
            data: b"<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\">\
<Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/>\
<Default Extension=\"xml\" ContentType=\"application/xml\"/>\
<Override PartName=\"/xl/workbook.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml\"/>\
<Override PartName=\"/xl/worksheets/sheet1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml\"/>\
</Types>".to_vec(),
        },
        ZipEntry {
            name: "_rels/.rels",
            data: b"<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">\
<Relationship Id=\"rId1\" \
 Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" \
 Target=\"xl/workbook.xml\"/>\
</Relationships>".to_vec(),
        },
        ZipEntry {
            name: "xl/workbook.xml",
            data: format!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<workbook xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\" \
 xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\">\
<sheets><sheet name=\"{sn}\" sheetId=\"1\" r:id=\"rId1\"/></sheets></workbook>"
            ).into_bytes(),
        },
        ZipEntry {
            name: "xl/_rels/workbook.xml.rels",
            data: b"<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">\
<Relationship Id=\"rId1\" \
 Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet\" \
 Target=\"worksheets/sheet1.xml\"/></Relationships>".to_vec(),
        },
        ZipEntry {
            name: "xl/worksheets/sheet1.xml",
            data: sheet.into_bytes(),
        },
    ];

    build_zip(&entries)
}

pub const XLSX_CONTENT_TYPE: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet";
