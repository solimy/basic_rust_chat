mod client;
mod model;
mod protocol;
mod server;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
#[command(arg_required_else_help = true)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Serve the chat server")]
    #[command(arg_required_else_help = true)]
    Serve {
        #[arg(help = "Host to bind the server")]
        host: String,
        #[arg(help = "Port to bind the server")]
        port: u16,
    },
    #[command(about = "Join a conversation")]
    #[command(arg_required_else_help = true)]
    Join {
        #[arg(help = "Host of the server to connect to")]
        host: String,
        #[arg(help = "Port of the server to connect to")]
        port: u16,
        #[arg(help = "Conversation ID to join")]
        conversation_id: String,
    },
    #[command(about = "List conversations")]
    #[command(arg_required_else_help = true)]
    List {
        #[arg(help = "Host of the server to connect to")]
        host: String,
        #[arg(help = "Port of the server to connect to")]
        port: u16,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Serve { host, port }) => {
            server::serve(host, port);
        }
        Some(Commands::Join {
            host,
            port,
            conversation_id,
        }) => {
            client::join(host, port, conversation_id);
        }
        Some(Commands::List { host, port }) => {
            client::list(host, port);
        }
        None => {}
    }
}
