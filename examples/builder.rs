mod server {
    use optarg2chain::optarg_impl;

    #[derive(Debug)]
    pub struct Server {
        hostname: String,
        port: u16,
        service_name: String,
        enabled: bool,
    }

    #[optarg_impl]
    impl Server {
        #[optarg_method(ServerBuilder, build)]
        pub fn new<'a, 'b>(
            hostname: &'a str,
            port: u16,
            #[optarg_default] service_name: &'b str,
            #[optarg(false)] enabled: bool,
        ) -> Server {
            // Some processes to open the server...
            Server {
                hostname: hostname.to_owned(),
                port,
                service_name: service_name.to_owned(),
                enabled,
            }
        }
    }
}

fn main() {
    let server = server::Server::new("example.com", 10000)
        .service_name("my-super-service")
        .build();
    println!("{:#?}", server);
}
