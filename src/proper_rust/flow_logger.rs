use std::{fmt, option, thread};
use std::convert::Infallible;

use chrono::{
    DateTime,
    format::{DelayedFormat, Fixed, Item}, Utc,
};
use log::{error, info, Record};
use log4rs::config::{Deserialize, Deserializers};
use log4rs::encode::{Encode, Write};
use log::Level;
use serde::ser::{self, Serialize};
use uuid::Uuid;
use warp::Filter;
use warp::http::HeaderMap;

use crate::proper_rust::settings::{LoggingMeta, Settings};

#[derive(Clone)]
pub struct FlowContext {
    pub flow_id: String,
}

pub trait FromFlowContext {
    fn from(self) -> FlowContext;
}

impl FromFlowContext for Option<String> {
    fn from(self) -> FlowContext {
        let flow_id = self.unwrap_or_else(|| { Uuid::new_v4().to_string() });
        FlowContext {
            flow_id
        }
    }
}

impl FromFlowContext for &str {
    fn from(self) -> FlowContext {
        FlowContext {
            flow_id: self.to_string()
        }
    }
}

impl FlowContext {
    pub fn new<A>(args: A) -> FlowContext
        where A: FromFlowContext
    {
        args.from()
    }

    pub fn extract_flow_context() -> impl Filter<Extract=(FlowContext, ), Error=Infallible> + Copy {
        warp::header::headers_cloned().map(move |headers: HeaderMap| {
            let flow_id_opt = headers.get("flow-id").map(|v| {
                match v.to_str() {
                    Ok(s) => Some(s.to_string()),
                    Err(_) => None,
                }
            }).flatten();
            FlowContext::new(flow_id_opt)
        })
    }
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

/// The JSON encoder's configuration
#[derive(Clone, Eq, PartialEq, Hash, Debug, Default, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JsonEncoderConfig {
    #[serde(skip_deserializing)]
    _p: (),
}

/// An `Encode`r which writes a JSON object.
#[derive(Clone, Debug)]
pub struct JsonEncoder {
    logging_meta: LoggingMeta,
}

impl JsonEncoder {
    fn new(logging_meta: LoggingMeta) -> JsonEncoder {
        JsonEncoder { logging_meta }
    }

    fn encode_inner(
        &self,
        w: &mut dyn Write,
        time: DateTime<Utc>,
        record: &Record,
    ) -> anyhow::Result<()> {
        let thread = thread::current();
        let flow_id_opt =
            log_mdc::get("flow-id", |s_opt| { s_opt.map(|s| { s.to_string() }) });

        let message = Message {
            time: time.format_with_items(Some(Item::Fixed(Fixed::RFC3339)).into_iter()),
            message: record.args(),
            level: record.level(),
            logger_name: record.target(),
            thread: thread.name(),
            thread_id: thread_id::get(),
            flow_id: flow_id_opt,
            app: self.logging_meta.name.as_str(),
            build_time: self.logging_meta.build_time.as_str(),
            version: self.logging_meta.version.as_str(),
        };
        message.serialize(&mut serde_json::Serializer::new(&mut *w))?;
        w.write_all("\n".as_bytes())?;
        Ok(())
    }
}

impl Encode for JsonEncoder {
    fn encode(&self, w: &mut dyn Write, record: &Record) -> anyhow::Result<()> {
        self.encode_inner(w, Utc::now(), record)
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
    app: &'a str,
    version: &'a str,
    build_time: &'a str,
}

fn ser_display<T, S>(v: &T, s: S) -> Result<S::Ok, S::Error>
    where
        T: fmt::Display,
        S: ser::Serializer,
{
    s.collect_str(v)
}

#[derive(Clone, Debug)]
pub struct CustomJsonEncoderDeserializer {
    logging_meta: LoggingMeta,
}

impl CustomJsonEncoderDeserializer {
    pub fn new(service_name: LoggingMeta) -> CustomJsonEncoderDeserializer {
        CustomJsonEncoderDeserializer { logging_meta: service_name }
    }
}

impl Deserialize for CustomJsonEncoderDeserializer {
    type Trait = dyn Encode;

    type Config = JsonEncoderConfig;

    fn deserialize(
        &self,
        _: JsonEncoderConfig,
        _: &Deserializers,
    ) -> anyhow::Result<Box<dyn Encode>> {
        Ok(Box::new(JsonEncoder::new(self.logging_meta.clone())))
    }
}

pub fn init_logging(config: &Settings) {
    let log_file = match &config.log_file {
        Some(a) => a.to_string(),
        None => "log4rs.yml".to_string()
    };
    let mut d: Deserializers = Default::default();
    d.insert("json", CustomJsonEncoderDeserializer::new(config.service.clone()));
    log4rs::init_file(log_file, d).unwrap();
}


#[cfg(test)]
mod test {
    use chrono::{DateTime, Utc};
    use log::{Level, Record};
    use log4rs::encode::writer::simple::SimpleWriter;

    use crate::proper_rust::flow_logger::JsonEncoder;
    use crate::proper_rust::settings::LoggingMeta;

    #[test]
    fn default() {
        let time = DateTime::parse_from_rfc3339("2016-03-20T14:22:20.644420340-08:00")
            .unwrap()
            .with_timezone(&Utc);
        let level = Level::Debug;
        let logger_name = "target";
        let module_path = "module_path";
        let file = "file";
        let line = 100;
        let message = "message";
        let thread = "proper_rust::flow_logger::test::default";
        let flow_id = "my-flow-id";
        log_mdc::insert("flow-id", flow_id);

        let encoder = JsonEncoder::new(LoggingMeta {
            build_time: "build".to_string(),
            name: "name".to_string(),
            version: "123".to_string(),
        });

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
             \"thread\":\"{}\",\"thread_id\":{},\"flow-id\":\"{}\",\
             \"app\":\"name\",\
             \"version\":\"123\",\
             \"build_time\":\"build\"\
             }}",
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