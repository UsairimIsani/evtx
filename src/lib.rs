#![deny(unused_must_use)]
#![cfg_attr(backtraces, feature(backtrace))]
#![forbid(unsafe_code)]
// Don't allow dbg! prints in release.
#![cfg_attr(not(debug_assertions), deny(clippy::dbg_macro))]
// This needs to come first!
#[macro_use]
mod macros;

pub use evtx_chunk::{EvtxChunk, EvtxChunkData, EvtxChunkHeader, IterChunkRecords};
pub use evtx_parser::{EvtxParser, IntoIterChunks, IterChunks, ParserSettings};
pub use evtx_record::{EvtxRecord, EvtxRecordHeader, SerializedEvtxRecord};
pub use json_output::JsonOutput;
pub use xml_output::{BinXmlOutput, XmlOutput};

pub mod binxml;
pub mod err;
pub mod model;

mod evtx_chunk;
mod evtx_file_header;
mod evtx_parser;
mod evtx_record;
mod string_cache;
mod template_cache;
mod utils;

mod json_output;
mod xml_output;

pub type ChunkOffset = u32;
pub type FileOffset = u64;

use crate::binxml::name::BinXmlName;
use crate::binxml::value_variant::BinXmlValue;
use crate::model::xml::{XmlAttribute, XmlElement};
use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;
use std::borrow::Cow;

fn bytes_to_string(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).expect("UTF8 Input")
}

fn dummy_event() -> XmlElement<'static> {
    XmlElement {
        name: Cow::Owned(BinXmlName::from_str("Dummy")),
        attributes: vec![],
    }
}

fn event_to_element(event: BytesStart) -> XmlElement {
    let mut attrs = vec![];

    for attr in event.attributes() {
        let attr = attr.expect("Failed to read attribute.");
        attrs.push(XmlAttribute {
            name: Cow::Owned(BinXmlName::from_string(bytes_to_string(attr.key))),
            // We have to compromise here and assume all values are strings.
            value: Cow::Owned(BinXmlValue::StringType(bytes_to_string(&attr.value))),
        });
    }

    XmlElement {
        name: Cow::Owned(BinXmlName::from_string(bytes_to_string(event.name()))),
        attributes: attrs,
    }
}
pub fn xml_to_json(xml: &str) -> String {
    let settings = ParserSettings::new()
        .num_threads(1)
        .separate_json_attributes(true);

    let mut reader = Reader::from_str(xml);
    reader.trim_text(true);

    let mut output = JsonOutput::new(&settings);
    output.visit_start_of_stream().expect("Start of stream");

    let mut buf = vec![];

    loop {
        match reader.read_event(&mut buf) {
            Ok(event) => match event {
                Event::Start(start) => {
                    output
                        .visit_open_start_element(&event_to_element(start))
                        .expect("Open start element");
                }
                Event::End(_) => output
                    .visit_close_element(&dummy_event())
                    .expect("Close element"),
                Event::Empty(empty) => {
                    output
                        .visit_open_start_element(&event_to_element(empty))
                        .expect("Empty Open start element");

                    output
                        .visit_close_element(&dummy_event())
                        .expect("Empty Close");
                }
                Event::Text(text) => output
                    .visit_characters(&BinXmlValue::StringType(bytes_to_string(text.as_ref())))
                    .expect("Text element"),
                Event::Comment(_) => {}
                Event::CData(_) => unimplemented!(),
                Event::Decl(_) => {}
                Event::PI(_) => unimplemented!(),
                Event::DocType(_) => {}
                Event::Eof => {
                    output.visit_end_of_stream().expect("End of stream");
                    break;
                }
            },
            Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
        }
    }

    serde_json::to_string_pretty(&output.into_value().expect("Output")).expect("To serialize")
}

// For tests, we only initialize logging once.
#[cfg(test)]
use std::sync::Once;

#[cfg(test)]
static LOGGER_INIT: Once = Once::new();

// Rust runs the tests concurrently, so unless we synchronize logging access
// it will crash when attempting to run `cargo test` with some logging facilities.
#[cfg(test)]
pub fn ensure_env_logger_initialized() {
    use std::io::Write;

    LOGGER_INIT.call_once(|| {
        let mut builder = env_logger::Builder::from_default_env();
        builder
            .format(|buf, record| writeln!(buf, "[{}] - {}", record.level(), record.args()))
            .init();
    });
}

// Cannot use `cfg(test)` here since `rustdoc` won't look at it.
#[cfg(debug_assertions)]
mod test_readme {
    macro_rules! calculated_doc {
        ($doc:expr, $id:ident) => {
            #[doc = $doc]
            enum $id {}
        }
    }

    calculated_doc!(include_str!("../README.md"), _DoctestReadme);
}
