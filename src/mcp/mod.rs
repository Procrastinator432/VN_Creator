pub mod protocol;
pub mod tools;
pub mod validation;

use protocol::{
    JsonRpcRequest, JsonRpcResponse, PromptArgument, PromptDefinition, ResourceContent,
    ResourceDefinition,
};
use serde_json::{json, Value};
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::Path;

pub fn run_mcp_server() -> io::Result<()> {
    eprintln!("[MCP] VN_Creator MCP-Server gestartet (stdio). Warten auf Anfragen...");
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                eprintln!("[MCP] Fehler beim Lesen von stdin: {}", e);
                break;
            }
        };

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let req: JsonRpcRequest = match serde_json::from_str(trimmed) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("[MCP] Fehlerhaftes JSON-RPC: {}", e);
                let err_resp = JsonRpcResponse::error(
                    None,
                    -32700,
                    format!("JSON-RPC Parse Error: {}", e),
                    None,
                );
                let out = serde_json::to_string(&err_resp)?;
                writeln!(stdout, "{}", out)?;
                stdout.flush()?;
                continue;
            }
        };

        // If request has no id, it is a notification -> do not respond!
        let is_notification = req.id.is_none();

        if let Some(resp) = handle_request(&req)
            && !is_notification {
                let out = serde_json::to_string(&resp)?;
                writeln!(stdout, "{}", out)?;
                stdout.flush()?;
            }
    }

    eprintln!("[MCP] VN_Creator MCP-Server beendet.");
    Ok(())
}

fn handle_request(req: &JsonRpcRequest) -> Option<JsonRpcResponse> {
    let method = req.method.as_str();
    let id = req.id.clone();
    let params = req.params.clone().unwrap_or(Value::Null);

    match method {
        "initialize" => {
            let result = json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {},
                    "resources": {},
                    "prompts": {}
                },
                "serverInfo": {
                    "name": "vn-creator",
                    "version": "1.1.0"
                }
            });
            Some(JsonRpcResponse::success(id, result))
        }

        "notifications/initialized" | "initialized" => {
            eprintln!("[MCP] Client hat Initialisierung bestätigt.");
            None
        }

        "ping" => Some(JsonRpcResponse::success(id, json!({}))),

        "tools/list" => {
            let tool_defs = tools::get_tool_definitions();
            Some(JsonRpcResponse::success(id, json!({ "tools": tool_defs })))
        }

        "tools/call" => {
            let name = match params.get("name").and_then(|v| v.as_str()) {
                Some(n) => n,
                None => {
                    return Some(JsonRpcResponse::error(
                        id,
                        -32602,
                        "Ungültige Parameter: 'name' erforderlich",
                        None,
                    ))
                }
            };

            let args = params.get("arguments").cloned().unwrap_or(Value::Null);
            let result = tools::handle_tool_call(name, &args);
            Some(JsonRpcResponse::success(
                id,
                serde_json::to_value(result).unwrap_or(Value::Null),
            ))
        }

        "resources/list" => {
            let resources = list_chapter_resources();
            Some(JsonRpcResponse::success(id, json!({ "resources": resources })))
        }

        "resources/read" => {
            let uri = match params.get("uri").and_then(|v| v.as_str()) {
                Some(u) => u,
                None => {
                    return Some(JsonRpcResponse::error(
                        id,
                        -32602,
                        "Ungültige Parameter: 'uri' erforderlich",
                        None,
                    ))
                }
            };

            match read_chapter_resource(uri) {
                Ok(content) => Some(JsonRpcResponse::success(id, json!({ "contents": [content] }))),
                Err(err_msg) => Some(JsonRpcResponse::error(id, -32000, err_msg, None)),
            }
        }

        "prompts/list" => {
            let prompts = get_prompt_definitions();
            Some(JsonRpcResponse::success(id, json!({ "prompts": prompts })))
        }

        "prompts/get" => {
            let name = match params.get("name").and_then(|v| v.as_str()) {
                Some(n) => n,
                None => {
                    return Some(JsonRpcResponse::error(
                        id,
                        -32602,
                        "Ungültige Parameter: 'name' erforderlich",
                        None,
                    ))
                }
            };

            match get_prompt_messages(name, &params.get("arguments").cloned().unwrap_or(Value::Null)) {
                Some(resp) => Some(JsonRpcResponse::success(id, resp)),
                None => Some(JsonRpcResponse::error(
                    id,
                    -32601,
                    format!("Unbekannter Prompt: '{}'", name),
                    None,
                )),
            }
        }

        unknown => {
            eprintln!("[MCP] Unbekannte Methode: {}", unknown);
            Some(JsonRpcResponse::error(
                id,
                -32601,
                format!("Methode nicht gefunden: '{}'", unknown),
                None,
            ))
        }
    }
}

fn list_chapter_resources() -> Vec<ResourceDefinition> {
    let mut resources = Vec::new();
    let current_dir = Path::new(".");

    if let Ok(entries) = fs::read_dir(current_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
                if file_name != "settings.json" {
                    resources.push(ResourceDefinition {
                        uri: format!("vn://chapter/{}", file_name),
                        name: file_name.to_string(),
                        description: Some(format!("Visual Novel Kapitel '{}'", file_name)),
                        mime_type: Some("application/json".to_string()),
                    });
                }
            }
        }
    }

    resources
}

fn read_chapter_resource(uri: &str) -> Result<ResourceContent, String> {
    let prefix = "vn://chapter/";
    if !uri.starts_with(prefix) {
        return Err(format!("Ungültiges URI-Schema. Erwartet 'vn://chapter/<dateiname>', erhalten: '{}'", uri));
    }

    let file_name = &uri[prefix.len()..];
    let path = Path::new(file_name);
    if !path.exists() {
        return Err(format!("Kapiteldatei '{}' nicht gefunden.", file_name));
    }

    match fs::read_to_string(path) {
        Ok(text) => Ok(ResourceContent {
            uri: uri.to_string(),
            mime_type: Some("application/json".to_string()),
            text,
        }),
        Err(e) => Err(format!("Fehler beim Lesen von '{}': {}", file_name, e)),
    }
}

fn get_prompt_definitions() -> Vec<PromptDefinition> {
    vec![
        PromptDefinition {
            name: "create_vn_scene".to_string(),
            description: Some("Leitfaden zum Erstellen einer kohärenten Visual Novel Szene mit passenden Dialogen, Hintergründen und Charakter-Sprites.".to_string()),
            arguments: Some(vec![
                PromptArgument {
                    name: "scene_idea".to_string(),
                    description: Some("Worum geht es in der Szene? (z.B. Geheimnisvolles Treffen nach der Schule)".to_string()),
                    required: true,
                },
                PromptArgument {
                    name: "characters".to_string(),
                    description: Some("Welche Charaktere kommen vor?".to_string()),
                    required: false,
                },
            ]),
        },
        PromptDefinition {
            name: "review_and_fix_chapter".to_string(),
            description: Some("Analysiert ein Kapitel mit validate_chapter, behebt nicht existierende Labels oder fehlende Jumps und optimiert das Story-Pacing.".to_string()),
            arguments: Some(vec![
                PromptArgument {
                    name: "chapter_path".to_string(),
                    description: Some("Pfad zur Kapiteldatei (z.B. 'test_chapter.json')".to_string()),
                    required: true,
                },
            ]),
        },
    ]
}

fn get_prompt_messages(name: &str, args: &Value) -> Option<Value> {
    match name {
        "create_vn_scene" => {
            let idea = args.get("scene_idea").and_then(|v| v.as_str()).unwrap_or("Eine neue Szene");
            let chars = args.get("characters").and_then(|v| v.as_str()).unwrap_or("Protagonist und ein Begleiter");
            Some(json!({
                "description": "Erstelle eine Visual Novel Szene",
                "messages": [
                    {
                        "role": "user",
                        "content": {
                            "type": "text",
                            "text": format!(
                                "Erstelle eine Visual Novel Szene für VN_Creator mit folgender Idee:\n\nIdee: {}\nCharaktere: {}\n\nBeachte:\n1. Verwende 'list_assets', um vorhandene Hintergründe und Sprites zu nutzen.\n2. Strukturiere die Szene mit SetBackground, ShowCharacter, Dialogue, und ggf. Choice.\n3. Stelle sicher, dass Sprungziele (Jumps) und Choices auf existierende Labels verweisen.\n4. Validiere das Ergebnis mit 'validate_chapter'.",
                                idea, chars
                            )
                        }
                    }
                ]
            }))
        }
        "review_and_fix_chapter" => {
            let path = args.get("chapter_path").and_then(|v| v.as_str()).unwrap_or("test_chapter.json");
            Some(json!({
                "description": "Kapitel analysieren und Fehler beheben",
                "messages": [
                    {
                        "role": "user",
                        "content": {
                            "type": "text",
                            "text": format!(
                                "Bitte überprüfe das Kapitel '{}':\n1. Rufe 'validate_chapter' für '{}' auf.\n2. Analysiere Fehler bei Sprüngen/Choices oder unbenutzten Labels.\n3. Korrigiere die Fehler und speichere das Kapitel mit 'save_chapter'.",
                                path, path
                            )
                        }
                    }
                ]
            }))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_initialize() {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            method: "initialize".to_string(),
            params: Some(json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {}
            })),
        };

        let resp = handle_request(&req).expect("Expected response");
        assert_eq!(resp.jsonrpc, "2.0");
        assert_eq!(resp.id, Some(json!(1)));
        assert!(resp.error.is_none());

        let res = resp.result.expect("Expected result");
        assert_eq!(res["serverInfo"]["name"], "vn-creator");
        assert!(res["capabilities"]["tools"].is_object());
    }

    #[test]
    fn test_mcp_tools_list() {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(2)),
            method: "tools/list".to_string(),
            params: None,
        };

        let resp = handle_request(&req).expect("Expected response");
        let res = resp.result.expect("Expected result");
        let tools = res["tools"].as_array().expect("Tools must be an array");
        assert_eq!(tools.len(), 10);

        let names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
        assert!(names.contains(&"list_chapters"));
        assert!(names.contains(&"read_chapter"));
        assert!(names.contains(&"create_chapter"));
        assert!(names.contains(&"save_chapter"));
        assert!(names.contains(&"add_actions"));
        assert!(names.contains(&"validate_chapter"));
        assert!(names.contains(&"list_assets"));
        assert!(names.contains(&"launch_player"));
        assert!(names.contains(&"launch_editor"));
        assert!(names.contains(&"get_settings"));
    }

    #[test]
    fn test_mcp_validate_existing_chapter() {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(3)),
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": "validate_chapter",
                "arguments": {
                    "path": "test_chapter.json"
                }
            })),
        };

        let resp = handle_request(&req).expect("Expected response");
        let res = resp.result.expect("Expected result");
        assert_eq!(res["isError"], false);
        let text = res["content"][0]["text"].as_str().expect("text content");
        let val: Value = serde_json::from_str(text).expect("Valid json text");
        assert_eq!(val["valid"], true);
        assert_eq!(val["errors_count"], 0);
    }

    #[test]
    fn test_mcp_read_chapter() {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(4)),
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": "read_chapter",
                "arguments": {
                    "path": "test_chapter.json",
                    "outline_only": true
                }
            })),
        };

        let resp = handle_request(&req).expect("Expected response");
        let res = resp.result.expect("Expected result");
        assert_eq!(res["isError"], false);
        let text = res["content"][0]["text"].as_str().expect("text content");
        let val: Value = serde_json::from_str(text).expect("Valid json text");
        assert_eq!(val["title"], "Kapitel 1: Das Treffen");
        assert!(val["outline"].as_array().is_some());
    }

    #[test]
    fn test_mcp_prompts_list() {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(5)),
            method: "prompts/list".to_string(),
            params: None,
        };

        let resp = handle_request(&req).expect("Expected response");
        let res = resp.result.expect("Expected result");
        let prompts = res["prompts"].as_array().expect("Prompts must be an array");
        assert_eq!(prompts.len(), 2);
    }
}
