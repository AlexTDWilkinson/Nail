//! MCP module stdlib registry entries. mcp_serve takes an array of tool
//! structs and dispatches to the program's handle_tool function, so it is
//! written out in full.

use super::*;

pub(super) fn register(m: &mut HashMap<&'static str, StdlibFunction>) {
    m.insert("mcp_serve", StdlibFunction {
        rust_path: "std_lib::mcp::serve".to_string(),
        crate_deps: vec![CrateDependency::Tokio, CrateDependency::SerdeJson, CrateDependency::Serde],
        struct_derives: vec![],
        custom_type_imports: vec![("MCP_Tool", "nail::std_lib::mcp")],
        module: StdlibModule::Mcp,
        parameters: vec![
            nail_param!(name: s),
            nail_param!(version: s),
            StdlibParameter {
                name: "tools".to_string(),
                param_type: NailDataTypeDescriptor::Array(Box::new(NailDataTypeDescriptor::Struct("MCP_Tool".to_string()))),
                pass_by_reference: false,
            },
        ],
        return_type: nail_type!((v!e)),
        diverging: false,
        description: "Serves the declared tools as an MCP server over stdin and stdout, the protocol AI assistants use to call outside tools. Each call is passed to the program's handle_tool(name:s, arguments_json:s):s!e function, whose Ok text becomes the tool result and whose error becomes a tool error the model can read. Stdout belongs to the protocol while serving, so anything for a person goes through the log functions, which write to stderr. Blocks until the client hangs up. The error case is a tool list that is empty or carries a schema that is not JSON.",
        example: "danger(mcp_serve(`weather`, `1.0.0`, tools));",
    });
}
