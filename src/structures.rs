pub mod structures {

    use serenity::model::id::GuildId;

    pub type ListId = u64;

    pub struct PingList {
        pub id: u64,
        pub guild_id: GuildId,
        pub description: String,
        pub cooldown: u64,
        pub restricted_join: bool,
        pub restricted_ping: bool,
        pub visible: bool,
    }
}
