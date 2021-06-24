pub mod init {
    include!(concat!(env!("OUT_DIR"), "/proto.init.rs"));
}

pub mod client {
    include!(concat!(env!("OUT_DIR"), "/proto.client.rs"));
}

pub mod server {
    include!(concat!(env!("OUT_DIR"), "/proto.server.rs"));
}

pub(crate) mod protocol {
    include!(concat!(env!("OUT_DIR"), "/proto.protocol.rs"));
}
