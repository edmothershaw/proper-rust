use std::{fmt, option, thread};

use chrono::{
    DateTime,
    format::{DelayedFormat, Fixed, Item}, Local,
};
use log::{error, info, Record};
use log4rs::config::{Deserialize, Deserializers};
use log4rs::encode;
use log4rs::encode::{Encode, Write};
use log::Level;
use serde::ser::{self, Serialize, SerializeMap};

#[derive(Clone)]
pub struct FlowContext {
    pub flow_id: String,
}

pub struct FlowLogger {
    name: String,
}

impl FlowLogger {
    pub fn new(name: &str) -> FlowLogger {
        FlowLogger { name: name.to_string() }
    }

    pub fn info(&self, fc: &FlowContext, message: &str) {
        FlowLogger::mdc_flow_context(fc);
        info!(target: self.name.as_str(), "{}", message)
    }

    pub fn error(&self, fc: &FlowContext, message: &str) {
        FlowLogger::mdc_flow_context(fc);
        error!(target: self.name.as_str(), "{}", message)
    }

    fn mdc_flow_context(fc: &FlowContext) {
        log_mdc::insert("flow-id", &fc.flow_id);
    }
}

//
/// The JSON encoder's configuration
#[derive(Clone, Eq, PartialEq, Hash, Debug, Default, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JsonEncoderConfig {
    #[serde(skip_deserializing)]
    _p: (),
}

/// An `Encode`r which writes a JSON object.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, Default)]
pub struct JsonEncoder(());

impl JsonEncoder {
    /// Returns a new `JsonEncoder` with a default configuration.
    pub fn new() -> Self {
        Self::default()
    }
}

const NEWLINE: &str = "\n";

impl JsonEncoder {
    fn encode_inner(
        &self,
        w: &mut dyn Write,
        time: DateTime<Local>,
        record: &Record,
    ) -> anyhow::Result<()> {
        let thread = thread::current();
        let mdc = Mdc;
        let flow_id: Option<String> = log_mdc::get("flow-id", |s_opt| {
            match s_opt {
                Some(s) => {
                    Some(s.to_string())
                }
                None => None
            }
        });
        let message = Message {
            time: time.format_with_items(Some(Item::Fixed(Fixed::RFC3339)).into_iter()),
            message: record.args(),
            level: record.level(),
            logger_name: record.target(),
            thread: thread.name(),
            thread_id: thread_id::get(),
            flow_id: flow_id,
        };
        message.serialize(&mut serde_json::Serializer::new(&mut *w))?;
        w.write_all(NEWLINE.as_bytes())?;
        Ok(())
    }
}

impl Encode for JsonEncoder {
    fn encode(&self, w: &mut dyn Write, record: &Record) -> anyhow::Result<()> {
        self.encode_inner(w, Local::now(), record)
    }
}

#[derive(serde::Serialize)]
struct Message<'a> {
    #[serde(serialize_with = "ser_display")]
    time: DelayedFormat<option::IntoIter<Item<'a>>>,
    #[serde(serialize_with = "ser_display")]
    message: &'a fmt::Arguments<'a>,
    level: Level,
    logger_name: &'a str,
    thread: Option<&'a str>,
    thread_id: usize,
    #[serde(rename = "flow-id", skip_serializing_if = "Option::is_none")]
    flow_id: Option<String>,
}

fn ser_display<T, S>(v: &T, s: S) -> Result<S::Ok, S::Error>
    where
        T: fmt::Display,
        S: ser::Serializer,
{
    s.collect_str(v)
}

struct Mdc;

impl ser::Serialize for Mdc {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: ser::Serializer,
    {
        let mut map = serializer.serialize_map(None)?;

        let mut err = Ok(());
        log_mdc::iter(|k, v| {
            if let Ok(()) = err {
                err = map.serialize_key(k).and_then(|()| map.serialize_value(v));
            }
        });
        err?;

        map.end()
    }
}


#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, Default)]
pub struct CustomJsonEncoderDeserializer;

impl Deserialize for CustomJsonEncoderDeserializer {
    type Trait = dyn Encode;

    type Config = JsonEncoderConfig;

    fn deserialize(
        &self,
        _: JsonEncoderConfig,
        _: &Deserializers,
    ) -> anyhow::Result<Box<dyn Encode>> {
        Ok(Box::new(JsonEncoder::default()))
    }
}

pub fn init_logging(log_config_file: &str) {
    let mut d: Deserializers = Default::default();
    d.insert("json", CustomJsonEncoderDeserializer);
    log4rs::init_file(log_config_file, d).unwrap();
}


#[cfg(test)]
mod test {
    use chrono::{DateTime, Local};
    use log::{Level, Record};
    use log4rs::*;
    use log4rs::encode::writer::simple::SimpleWriter;

    use crate::proper_rust::flow_logger::JsonEncoder;

    #[test]
    fn default() {
        let time = DateTime::parse_from_rfc3339("2016-03-20T14:22:20.644420340-08:00")
            .unwrap()
            .with_timezone(&Local);
        let level = Level::Debug;
        let logger_name = "target";
        let module_path = "module_path";
        let file = "file";
        let line = 100;
        let message = "message";
        let thread = "proper_rust::flow_logger::test::default";
        let flow_id = "my-flow-id";
        log_mdc::insert("flow-id", flow_id);

        let encoder = JsonEncoder::new();

        let mut buf = vec![];
        encoder
            .encode_inner(
                &mut SimpleWriter(&mut buf),
                time,
                &Record::builder()
                    .level(level)
                    .target(logger_name)
                    .module_path(Some(module_path))
                    .file(Some(file))
                    .line(Some(line))
                    .args(format_args!("{}", message))
                    .build(),
            )
            .unwrap();

        let expected = format!(
            "{{\"time\":\"{}\",\"message\":\"{}\",\
             \"level\":\"{}\",\"logger_name\":\"{}\",\
             \"thread\":\"{}\",\"thread_id\":{},\"flow-id\":\"{}\"}}",
            time.to_rfc3339(),
            message,
            level,
            logger_name,
            thread,
            thread_id::get(),
            flow_id,
        );
        assert_eq!(expected, String::from_utf8(buf).unwrap().trim());
    }
}