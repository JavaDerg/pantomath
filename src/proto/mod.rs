macro_rules! include_proto {
    ($name:literal as $mod:ident) => {
        pub mod $mod {
            include!(concat!(env!("OUT_DIR"), "/", $name, ".rs"));
        }
    };
}

include_proto!("proto.init" as init);
include_proto!("proto.client" as client);
include_proto!("proto.server" as server);
include_proto!("proto.protocol" as protocol);

// pub mod init {
//     include!(concat!(env!("OUT_DIR"), "/proto.init.rs"));
// }
//
// pub mod client {
//     include!(concat!(env!("OUT_DIR"), "/proto.client.rs"));
// }
//
// pub mod server {
//     include!(concat!(env!("OUT_DIR"), "/proto.server.rs"));
// }
//
// pub(crate) mod protocol {
//     include!(concat!(env!("OUT_DIR"), "/proto.protocol.rs"));
// }
