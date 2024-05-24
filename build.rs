fn main() {
    tonic_build::compile_protos("proto/filter.proto").unwrap();
    tonic_build::compile_protos("proto/light_push.proto").unwrap();
    tonic_build::compile_protos("proto/peer_exchange.proto").unwrap();
    tonic_build::configure()
        .build_server(false)
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
