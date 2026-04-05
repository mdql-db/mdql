use std::path::PathBuf;

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();

    let db_path = if args.len() > 1 {
        PathBuf::from(&args[1])
    } else {
        PathBuf::from(".")
    };

    let port: u16 = if args.len() > 2 {
        args[2].parse().unwrap_or(3000)
    } else {
        3000
    };

    mdql_web::run_server(db_path, port).await;
}
