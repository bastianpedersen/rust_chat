mod server;

fn main() {
    let config = server::Config { port: 6969 };

    server::start_server(config);
}
