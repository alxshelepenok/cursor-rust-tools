mod cargo_remote;
mod context;
mod docs;
mod lsp;
mod mcp;
mod project;
mod ui;

use std::env::args;

use anyhow::Result;
use context::Context as ContextType;
use mcp::run_server;
use tokio::signal;
use tracing::{error, info};
use tracing_subscriber::{
    EnvFilter, Layer, fmt::format::PrettyFields, layer::SubscriberExt, util::SubscriberInitExt,
};
use ui::run_ui;

#[tokio::main]
async fn main() -> Result<()> {
    let log_layer = tracing_subscriber::fmt::layer()
        .event_format(tracing_subscriber::fmt::format().compact())
        .fmt_fields(PrettyFields::new())
        .boxed();

    tracing_subscriber::registry()
        .with(
            (EnvFilter::builder().try_from_env())
                .unwrap_or(EnvFilter::new("cursor_rust_tools=info")),
        )
        .with(log_layer)
        .init();

    let no_ui = args().any(|arg| arg == "--no-ui");

    let (sender, receiver) = flume::unbounded();
    let context = ContextType::new(4000, sender).await;
    context.load_config().await?;

    let final_context = context.clone();

    let cloned_context = context.clone();
    let server_handle = tokio::spawn(async move {
        run_server(cloned_context).await.unwrap();
    });

    let main_loop_fut = async {
        if no_ui {
            info!(
                "Running in CLI mode on port {}:{}",
                context.address_information().0,
                context.address_information().1
            );
            info!("Configuration file: {}", context.configuration_file());
            if context.projects().await.is_empty() {
                error!(
                    "No projects found, please run without `--no-ui` or edit configuration file"
                );
                return Ok(());
            }
            info!(
                "Cursor mcp json (project/.cursor.mcp.json):\n```json\n{}\n```",
                context.mcp_configuration()
            );

            loop {
                while let Ok(notification) = receiver.try_recv() {
                    info!("  {}", notification.description());
                }

                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        } else {
            let projects = context.projects().await;
            let settings = context.settings().await;

            run_ui(context, receiver, projects, settings)
        }
    };

    tokio::select! {
        res = main_loop_fut => {
            if let Err(e) = res {
                error!("Main loop finished with error: {}", e);
            } else {
                info!("Main loop finished normally.");
            }
        },
        _ = signal::ctrl_c() => {
            info!("Ctrl+C received, shutting down...");
        }
        _ = server_handle => {
             info!("Server task finished unexpectedly.");
        }
    }

    if no_ui {
        final_context.shutdown_all().await;
    }

    Ok(())
}
