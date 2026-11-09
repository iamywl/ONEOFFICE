//! KS X 6101 HWPX container parsing, serialization, and safe entry extraction.

use std::collections::BTreeMap;
use std::io::{Cursor, Read, Write};

use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::error::HwpxError;

/// Mandatory KS X 6101 HWPX MIME type.
pub const HWPX_MIMETYPE: &str = "application/hwp+zip";

/// Maximum size allowed for a single uncompressed package entry (128 MiB).
pub const MAX_SINGLE_ENTRY_BYTES: u64 = 128 * 1024 * 1024;

/// Maximum cumulative decompressed payload allowed across the package (512 MiB).
pub const MAX_TOTAL_DECOMPRESSED_BYTES: u64 = 512 * 1024 * 1024;

/// In-memory representation of an HWPX package with lossless passthrough preservation.
#[derive(Clone, Debug, PartialEq)]
pub struct HwpxPackage {
    /// In-memory map of normalized entry paths to uncompressed byte contents.
    entries: BTreeMap<String, Vec<u8>>,
}

impl HwpxPackage {
    /// Parses an HWPX package from a byte slice with strict KS X 6101 verification and zip-bomb guards.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, HwpxError> {
        let cursor = Cursor::new(bytes);
        let mut archive = ZipArchive::new(cursor)
            .map_err(|e| HwpxError::Zip(format!("Failed to open ZIP archive: {e}")))?;

        let mut entries = BTreeMap::new();
        let mut total_decompressed: u64 = 0;

        for i in 0..archive.len() {
            let mut file = archive
                .by_index(i)
                .map_err(|e| HwpxError::Zip(format!("Failed to read ZIP entry #{i}: {e}")))?;

            let raw_name = file.name().to_string();
            // Security: Path traversal checks
            if raw_name.contains("..") || raw_name.starts_with('/') || raw_name.starts_with('\\') {
                return Err(HwpxError::ZipBomb(format!(
                    "Path traversal attempt in entry: '{raw_name}'"
                )));
            }

            if file.is_dir() {
                continue;
            }

            let uncompressed_size = file.size();
            if uncompressed_size > MAX_SINGLE_ENTRY_BYTES {
                return Err(HwpxError::ZipBomb(format!(
                    "Entry '{raw_name}' uncompressed size {uncompressed_size} exceeds 128MiB limit"
                )));
            }

            total_decompressed = total_decompressed.saturating_add(uncompressed_size);
            if total_decompressed > MAX_TOTAL_DECOMPRESSED_BYTES {
                return Err(HwpxError::ZipBomb(
                    "Total package decompressed size exceeds 512MiB limit".to_string(),
                ));
            }

            let mut buffer = Vec::with_capacity(uncompressed_size as usize);
            file.read_to_end(&mut buffer)
                .map_err(|e| HwpxError::Io(format!("Failed to decompress '{raw_name}': {e}")))?;

            entries.insert(raw_name, buffer);
        }

        // Verify mimetype entry
        if let Some(mimetype_bytes) = entries.get("mimetype") {
            let mt_str = String::from_utf8_lossy(mimetype_bytes);
            if mt_str.trim() != HWPX_MIMETYPE {
                return Err(HwpxError::MalformedPackage(format!(
                    "Invalid mimetype: expected '{HWPX_MIMETYPE}', found '{mt_str}'"
                )));
            }
        }

        Ok(Self { entries })
    }

    /// Serializes the package conforming to KS X 6101:
    /// `mimetype` is entry #0 uncompressed (`Stored`), all other entries `Deflated`.
    pub fn to_bytes(&self) -> Result<Vec<u8>, HwpxError> {
        let mut buffer = Vec::new();
        let mut writer = ZipWriter::new(Cursor::new(&mut buffer));

        let stored_options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Stored)
            .unix_permissions(0o644);

        let deflated_options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Deflated)
            .unix_permissions(0o644);

        // 1. Mandatory KS X 6101: mimetype MUST be entry #0 and uncompressed
        let mimetype_content = self
            .entries
            .get("mimetype")
            .map(|v| v.as_slice())
            .unwrap_or(HWPX_MIMETYPE.as_bytes());

        writer
            .start_file("mimetype", stored_options)
            .map_err(|e| HwpxError::Zip(format!("Failed to write mimetype entry: {e}")))?;
        writer
            .write_all(mimetype_content)
            .map_err(|e| HwpxError::Io(e.to_string()))?;

        // 2. Write all other entries deterministically (sorted order)
        for (name, data) in &self.entries {
            if name == "mimetype" {
                continue;
            }
            writer
                .start_file(name, deflated_options)
                .map_err(|e| HwpxError::Zip(format!("Failed to start ZIP entry '{name}': {e}")))?;
            writer
                .write_all(data)
                .map_err(|e| HwpxError::Io(format!("Failed to write '{name}': {e}")))?;
        }

        writer
            .finish()
            .map_err(|e| HwpxError::Zip(format!("Failed to finalize ZIP archive: {e}")))?;

        Ok(buffer)
    }

    /// Total number of section XML files (`Contents/section*.xml`) in the package.
    #[must_use]
    pub fn section_count(&self) -> usize {
        let mut count = 0;
        while self
            .entries
            .contains_key(&format!("Contents/section{count}.xml"))
        {
            count += 1;
        }
        count
    }

    /// Reads the XML text of the requested section (`0`-indexed).
    pub fn section_xml(&self, section_idx: usize) -> Result<String, HwpxError> {
        let path = format!("Contents/section{section_idx}.xml");
        let bytes = self
            .entries
            .get(&path)
            .ok_or(HwpxError::SectionNotFound(section_idx))?;
        String::from_utf8(bytes.clone())
            .map_err(|e| HwpxError::Xml(format!("Section {section_idx} is not valid UTF-8: {e}")))
    }

    /// Updates or replaces the XML text of the requested section (`0`-indexed).
    pub fn set_section_xml(&mut self, section_idx: usize, xml: &str) -> Result<(), HwpxError> {
        let path = format!("Contents/section{section_idx}.xml");
        self.entries.insert(path, xml.as_bytes().to_vec());
        Ok(())
    }

    /// Reads the package manifest (`Contents/content.hpf`), if present.
    #[must_use]
    pub fn content_hpf(&self) -> Option<String> {
        self.entries
            .get("Contents/content.hpf")
            .and_then(|b| String::from_utf8(b.clone()).ok())
    }

    /// Updates the package manifest (`Contents/content.hpf`).
    pub fn set_content_hpf(&mut self, hpf: &str) {
        self.entries
            .insert("Contents/content.hpf".to_string(), hpf.as_bytes().to_vec());
    }

    /// Stores or replaces a binary asset (e.g. `BinData/image_1.png`, `BinData/table_1.xlsx`).
    pub fn add_binary_attachment(&mut self, internal_path: &str, data: Vec<u8>) {
        self.entries.insert(internal_path.to_string(), data);
    }

    /// Retrieves an internal file or attachment by its path.
    #[must_use]
    pub fn get_attachment(&self, internal_path: &str) -> Option<&[u8]> {
        self.entries.get(internal_path).map(Vec::as_slice)
    }

    /// In-memory view of all package entries.
    #[must_use]
    pub const fn entries(&self) -> &BTreeMap<String, Vec<u8>> {
        &self.entries
    }

    /// Constructs a valid, clean-room minimal KS X 6101 HWPX package in memory.
    pub fn create_minimal_hwpx(
        initial_title: &str,
        initial_body_text: &str,
    ) -> Result<Self, HwpxError> {
        let mut entries = BTreeMap::new();

        // 1. mimetype
        entries.insert("mimetype".to_string(), HWPX_MIMETYPE.as_bytes().to_vec());

        // 2. META-INF/container.xml
        let container_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<ocf:container xmlns:ocf="urn:oasis:names:tc:opendocument:xmlns:container">
  <ocf:rootfiles>
    <ocf:rootfile full-path="Contents/content.hpf" media-type="application/hwp+zip"/>
  </ocf:rootfiles>
</ocf:container>"#;
        entries.insert(
            "META-INF/container.xml".to_string(),
            container_xml.as_bytes().to_vec(),
        );

        // 3. Contents/content.hpf
        let content_hpf = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<opf:package xmlns:opf="http://www.idpf.org/2007/opf" version="2.0" unique-identifier="uid">
  <opf:metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:title>{initial_title}</dc:title>
    <dc:language>ko</dc:language>
    <dc:creator>ONEOFFICE Pure-Rust Engine</dc:creator>
  </opf:metadata>
  <opf:manifest>
    <opf:item id="header" href="header.xml" media-type="application/xml"/>
    <opf:item id="section0" href="section0.xml" media-type="application/xml"/>
  </opf:manifest>
  <opf:spine>
    <opf:itemref idref="section0"/>
  </opf:spine>
</opf:package>"#
        );
        entries.insert(
            "Contents/content.hpf".to_string(),
            content_hpf.as_bytes().to_vec(),
        );

        // 4. Contents/header.xml (minimal style definitions)
        let header_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<hh:head xmlns:hh="http://www.hancom.co.kr/hwpml/2011/head" version="1.0">
  <hh:beginNum page="1" footnote="1" endnote="1" pic="1" tbl="1" equation="1"/>
  <hh:refList>
    <hh:fontfaces itemCnt="1">
      <hh:fontface lang="hangul" fontCnt="1">
        <hh:font id="0" face="맑은 고딕" type="ttf" isEmbedded="0"/>
      </hh:fontface>
    </hh:fontfaces>
    <hh:charProperties itemCnt="1">
      <hh:charPr id="0" height="1000" textColor="000000" shadeColor="none" useFontSpace="0" symMark="none" borderFillIDRef="1"/>
    </hh:charProperties>
    <hh:paraProperties itemCnt="1">
      <hh:paraPr id="0" align="left" headingType="none" breakType="none"/>
    </hh:paraProperties>
  </hh:refList>
</hh:head>"#;
        entries.insert(
            "Contents/header.xml".to_string(),
            header_xml.as_bytes().to_vec(),
        );

        // 5. Contents/section0.xml (initial body paragraphs)
        let section0_xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<hs:sec xmlns:hs="http://www.hancom.co.kr/hwpml/2011/section" xmlns:hp="http://www.hancom.co.kr/hwpml/2011/paragraph" xmlns:hc="http://www.hancom.co.kr/hwpml/2011/core">
  <hp:p id="0" paraPrIDRef="0" styleIDRef="0" pageBreak="0" columnBreak="0">
    <hp:run charPrIDRef="0">
      <hp:t>{initial_title}</hp:t>
    </hp:run>
  </hp:p>
  <hp:p id="1" paraPrIDRef="0" styleIDRef="0" pageBreak="0" columnBreak="0">
    <hp:run charPrIDRef="0">
      <hp:t>{initial_body_text}</hp:t>
    </hp:run>
  </hp:p>
</hs:sec>"#
        );
        entries.insert(
            "Contents/section0.xml".to_string(),
            section0_xml.as_bytes().to_vec(),
        );

        Ok(Self { entries })
    }
}
