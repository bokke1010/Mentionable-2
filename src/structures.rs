pub mod structures {

    use serenity::model::id::GuildId;
    use std::cmp::max;

    pub type ListId = u64;

    pub struct PingList {
        pub id: u64,
        pub guild_id: GuildId,
        pub description: String,
        pub cooldown: u64,
        pub join_permission: PERMISSION,
        pub ping_permission: PERMISSION,
        pub visible: bool,
    }

    #[derive(PartialEq, Eq, PartialOrd, Ord)]
    pub enum PERMISSION {
        NEUTRAL = 0,
        DENY = 1,
        ALLOW = 2,
    }

    impl PERMISSION {
        pub fn combine(self, other: PERMISSION) -> PERMISSION {
            max(self, other)
        }

        pub fn fromint(value: u64) -> PERMISSION {
            match value {
                0 => PERMISSION::NEUTRAL,
                1 => PERMISSION::DENY,
                2 => PERMISSION::ALLOW,
                _ => panic!("Invalid permission value"),
            }
        }
    }
}
