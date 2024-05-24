fn main() {
    tonic_build::configure()
        .build_server(false)
        .protoc_arg("--experimental_allow_proto3_optional")
        .compile(
            &[
                "proto/message.proto",
                "proto/peer_exchange.proto",
                "proto/light_push.proto",
                "proto/filter.proto",
            ],
            &["proto/"],
        )
        .unwrap();
}
