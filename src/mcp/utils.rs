use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::context::{Context, ProjectContext};
use anyhow::Result;
use lsp_types::Position;
use mcp_core::types::{CallToolRequest, CallToolResponse, ToolResponseContent};

pub fn error_response(message: &str) -> CallToolResponse {
    CallToolResponse {
        content: vec![ToolResponseContent::Text {
            text: message.to_string(),
        }],
        is_error: Some(true),
        meta: None,
    }
}

pub(super) trait RequestExtension {
    fn get_line(&self) -> Result<u64, CallToolResponse>;
    fn get_symbol(&self) -> Result<String, CallToolResponse>;
    fn get_file(&self) -> Result<String, CallToolResponse>;
}

impl RequestExtension for CallToolRequest {
    fn get_line(&self) -> Result<u64, CallToolResponse> {
        let number = self
            .arguments
            .as_ref()
            .and_then(|args| args.get("line"))
            .and_then(|v| v.as_u64())
            .ok_or_else(|| error_response("Line is required"))?;
        
        
        Ok(number)
        
        
        
        
        
        
    }

    fn get_symbol(&self) -> Result<String, CallToolResponse> {
        self.arguments
            .as_ref()
            .and_then(|args| args.get("symbol"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| error_response("Symbol is required"))
            .map(|s| s.to_string())
    }

    fn get_file(&self) -> Result<String, CallToolResponse> {
        self.arguments
            .as_ref()
            .and_then(|args| args.get("file"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| error_response("File is required"))
            .map(|s| s.to_string())
    }
}

pub async fn get_info_from_request(
    context: &Context,
    request: &CallToolRequest,
) -> Result<(Arc<ProjectContext>, String, PathBuf), CallToolResponse> {
    let file = request.get_file()?;
    let absolute_path = PathBuf::from(file.clone());
    let Some(project) = context.get_project_by_path(&absolute_path).await else {
        return Err(error_response("No project found for file {file}"));
    };

    let relative_path = project
        .project
        .relative_path(&file)
        .map_err(|e| error_response(&e))?;

    Ok((project, relative_path, absolute_path))
}

pub async fn find_symbol_position_in_file(
    project: &Arc<ProjectContext>,
    relative_file: &str,
    symbol: &str,
    line: u64,
) -> Result<Position, String> {
    let symbols = match project.lsp.document_symbols(relative_file).await {
        Ok(Some(symbols)) => symbols,
        Ok(None) => return Err("No symbols found".to_string()),
        Err(e) => return Err(e.to_string()),
    };
    for symbol in symbols {
        if symbol.location.range.start.line == line as u32 {
            return Ok(symbol.location.range.start);
        }
    }
    Err(format!("Symbol {symbol} not found in file {relative_file}"))
}

pub fn get_file_lines(
    file_path: impl AsRef<Path>,
    start_line: u32,
    end_line: u32,
    prefix: u8,
    suffix: u8,
) -> std::io::Result<Option<String>> {
    let content = std::fs::read_to_string(file_path)?;
    let lines: Vec<&str> = content.lines().collect();

    
    let start = start_line.saturating_sub(prefix as u32);
    let mut end = end_line.saturating_add(suffix as u32);

    if end > lines.len() as u32 {
        end = lines.len() as u32;
    }

    
    if start > end || end >= lines.len() as u32 {
        return Ok(None);
    }

    
    let selected_lines = lines[start as usize..=end as usize].join("\n");
    Ok(Some(selected_lines))
}
