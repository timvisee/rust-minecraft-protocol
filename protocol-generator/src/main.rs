use std::fs::File;
use std::io::Write;

use crate::mappings::CodeMappings;
use crate::transformer::transform_protocol;
use handlebars::*;
use heck::SnakeCase;
use serde::Serialize;
use serde_json::json;
use structopt::StructOpt;

pub mod backend;
pub mod frontend;
pub mod mappings;
pub mod transformer;

#[derive(StructOpt)]
#[structopt(name = "protocol-generator")]
struct Opt {
    #[structopt(short, long, default_value = "1.14.4")]
    protocol_version: String,
}

pub fn main() {
    let opt: Opt = Opt::from_args();
    let template_engine = create_template_engine();

    let protocol_data_file_name = format!(
        "protocol-generator/minecraft-data/data/pc/{}/protocol.json",
        opt.protocol_version
    );

    let protocol_data_file =
        File::open(protocol_data_file_name).expect("Failed to open protocol data file");

    let protocol_input: backend::ProtocolHandler =
        serde_json::from_reader(protocol_data_file).expect("Failed to parse protocol data");

    let mappings = CodeMappings {};

    let protocols = vec![
        (
            transform_protocol(
                &mappings,
                frontend::State::Handshake,
                &protocol_input.handshaking,
            ),
            frontend::State::Handshake,
        ),
        (
            transform_protocol(&mappings, frontend::State::Status, &protocol_input.status),
            frontend::State::Status,
        ),
        (
            transform_protocol(&mappings, frontend::State::Login, &protocol_input.login),
            frontend::State::Login,
        ),
        (
            transform_protocol(&mappings, frontend::State::Game, &protocol_input.game),
            frontend::State::Game,
        ),
    ];

    for (protocol, state) in protocols {
        let file_name = format!(
            "protocol/src/packet/{}.rs",
            state.to_string().to_lowercase()
        );
        let file = File::create(file_name).expect("Failed to create file");

        generate_rust_file(&protocol, &template_engine, &file)
            .expect("Failed to generate rust file");
    }
}

fn create_template_engine() -> Handlebars<'static> {
    let mut template_engine = Handlebars::new();

    template_engine.register_helper("snake_case", Box::new(format_snake_case));
    template_engine.register_helper("packet_id", Box::new(format_packet_id));
    template_engine.register_escape_fn(|s| s.to_owned());

    template_engine
        .register_template_file(
            "packet_imports",
            "protocol-generator/templates/packet_imports.hbs",
        )
        .expect("Failed to register template");

    template_engine
        .register_template_file(
            "packet_enum",
            "protocol-generator/templates/packet_enum.hbs",
        )
        .expect("Failed to register template");

    template_engine
        .register_template_file(
            "packet_structs",
            "protocol-generator/templates/packet_structs.hbs",
        )
        .expect("Failed to register template");

    template_engine
}

#[derive(Serialize)]
struct GenerateContext<'a> {
    packet_enum_name: String,
    packets: &'a Vec<frontend::Packet>,
}

fn generate_rust_file<W: Write>(
    protocol: &frontend::Protocol,
    template_engine: &Handlebars,
    mut writer: W,
) -> Result<(), TemplateRenderError> {
    let server_bound_ctx = GenerateContext {
        packet_enum_name: format!("{}{}BoundPacket", &protocol.state, frontend::Bound::Server),
        packets: &protocol.server_bound_packets,
    };

    let client_bound_ctx = GenerateContext {
        packet_enum_name: format!("{}{}BoundPacket", &protocol.state, frontend::Bound::Client),
        packets: &protocol.client_bound_packets,
    };

    let mut imports = vec![
        "crate::DecodeError",
        "crate::Decoder",
        "std::io::Read",
        "minecraft_protocol_derive::Packet",
    ];

    imports.extend(protocol.data_type_imports().iter());

    template_engine.render_to_write(
        "packet_imports",
        &json!({ "imports": imports }),
        &mut writer,
    )?;

    template_engine.render_to_write("packet_enum", &server_bound_ctx, &mut writer)?;
    template_engine.render_to_write("packet_enum", &client_bound_ctx, &mut writer)?;

    template_engine.render_to_write("packet_structs", &server_bound_ctx, &mut writer)?;
    template_engine.render_to_write("packet_structs", &client_bound_ctx, &mut writer)?;

    Ok(())
}

fn format_snake_case(
    h: &Helper,
    _: &Handlebars,
    _: &Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> Result<(), RenderError> {
    let str = h
        .param(0)
        .and_then(|v| v.value().as_str())
        .ok_or(RenderError::new(
            "Param 0 with str type is required for snake case helper.",
        ))? as &str;

    let snake_case_str = str.to_snake_case();

    out.write(snake_case_str.as_ref())?;
    Ok(())
}

fn format_packet_id(
    h: &Helper,
    _: &Handlebars,
    _: &Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> Result<(), RenderError> {
    let id = h
        .param(0)
        .and_then(|v| v.value().as_u64())
        .ok_or(RenderError::new(
            "Param 0 with u64 type is required for packet id helper.",
        ))? as u64;

    let packet_id_str = format!("{:#04X}", id);

    out.write(packet_id_str.as_ref())?;
    Ok(())
}
