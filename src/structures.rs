pub mod structures {


    pub struct PingList {
        pub guild_id: u64,
        pub name: String,
        pub description: String,
        pub cooldown: u64,
        pub restricted_join: bool,
        pub restricted_ping: bool,
        pub visible: bool
    }
}