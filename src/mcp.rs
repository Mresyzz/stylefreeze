use crate::engine::SurgicalEngine;
use crate::lang::SupportedLang;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};
use std::path::Path;

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    params: Option<Value>,
}

#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<Value>,
}

pub fn run_mcp_server() -> anyhow::Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let reader = stdin.lock();

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let req: JsonRpcRequest = match serde_json::from_str(trimmed) {
            Ok(r) => r,
            Err(e) => {
                let err_resp = json!({
                    "jsonrpc": "2.0",
                    "id": Value::Null,
                    "error": {
                        "code": -32700,
                        "message": format!("Parse error: {}", e)
                    }
                });
                writeln!(stdout, "{}", err_resp)?;
                stdout.flush()?;
                continue;
            }
        };

        // If it's a notification without id, don't reply
        let req_id = match req.id {
            Some(id) => id,
            None => continue,
        };

        let res = handle_request(&req.method, req.params.as_ref());
        let resp = match res {
            Ok(result) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: req_id,
                result: Some(result),
                error: None,
            },
            Err(err_msg) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: req_id,
                result: None,
                error: Some(json!({
                    "code": -32603,
                    "message": err_msg
                })),
            },
        };

        let out = serde_json::to_string(&resp)?;
        writeln!(stdout, "{}", out)?;
        stdout.flush()?;
    }

    Ok(())
}

fn handle_request(method: &str, params: Option<&Value>) -> Result<Value, String> {
    match method {
        "initialize" => Ok(json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {}
            },
            "serverInfo": {
                "name": "stylefreeze",
                "version": "0.1.0"
            }
        })),
        "tools/list" => Ok(json!({
            "tools": [
                {
                    "name": "stylefreeze_reconcile",
                    "description": "Applies AST-level surgical diff reconciliation between original code and candidate code, discarding non-semantic indentation, whitespace, and unsolicited formatting churn.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "path": {
                                "type": "string",
                                "description": "File path (used for syntax/language detection, e.g. 'src/app.ts')"
                            },
                            "original": {
                                "type": "string",
                                "description": "Original file contents before LLM modification"
                            },
                            "candidate": {
                                "type": "string",
                                "description": "Candidate code proposed by the LLM"
                            }
                        },
                        "required": ["path", "original", "candidate"]
                    }
                }
            ]
        })),
        "tools/call" => {
            let params = params.ok_or_else(|| "Missing params for tools/call".to_string())?;
            let tool_name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let arguments = params.get("arguments").ok_or_else(|| "Missing arguments".to_string())?;

            if tool_name == "stylefreeze_reconcile" {
                let path_str = arguments.get("path").and_then(|v| v.as_str()).unwrap_or("file.ts");
                let original = arguments.get("original").and_then(|v| v.as_str()).unwrap_or("");
                let candidate = arguments.get("candidate").and_then(|v| v.as_str()).unwrap_or("");

                let lang = SupportedLang::from_path(Path::new(path_str)).unwrap_or(SupportedLang::TypeScript);
                let engine = SurgicalEngine::new(lang);
                let (surgical_text, stats) = engine.reconcile(original, candidate);

                Ok(json!({
                    "content": [
                        {
                            "type": "text",
                            "text": surgical_text
                        }
                    ],
                    "metadata": {
                        "discarded_churn_lines": stats.discarded_churn_lines,
                        "tokens_saved": stats.estimated_tokens_saved,
                        "surgical_diff_lines": stats.surgical_added_lines + stats.surgical_removed_lines
                    }
                }))
            } else {
                Err(format!("Unknown tool: {}", tool_name))
            }
        }
        _ => Err(format!("Method not found: {}", method)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_initialize_and_tools_list() {
        let init_res = handle_request("initialize", None).unwrap();
        assert_eq!(init_res["serverInfo"]["name"], "stylefreeze");

        let tools_res = handle_request("tools/list", None).unwrap();
        let tools = tools_res["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0]["name"], "stylefreeze_reconcile");
    }

    #[test]
    fn test_mcp_tools_call_reconcile() {
        let params = json!({
            "name": "stylefreeze_reconcile",
            "arguments": {
                "path": "test.ts",
                "original": "function foo() {\n    return 1;\n}\n",
                "candidate": "function foo() {\n  return 2;\n}\n"
            }
        });

        let call_res = handle_request("tools/call", Some(&params)).unwrap();
        let text = call_res["content"][0]["text"].as_str().unwrap();
        println!("MCP Reconciled text: {}", text);
        assert!(text.contains("    return 2;"));
    }
}
