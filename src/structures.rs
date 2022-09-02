pub mod structures {

    use serenity::model::id::{ChannelId, GuildId, RoleId};
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

    #[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
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

    #[derive(Clone, Copy)]
    pub enum LOGTRIGGER {
        RoleAdd(RoleId),
        RoleRemove(RoleId),
        JoinServer(),
        LeaveServer(),
    }

    impl LOGTRIGGER {
        pub fn toint(self) -> u64 {
            match self {
                LOGTRIGGER::RoleAdd(_) => 0,
                LOGTRIGGER::RoleRemove(_) => 1,
                LOGTRIGGER::JoinServer() => 2,
                LOGTRIGGER::LeaveServer() => 3,
            }
        }
    }

    #[derive(Clone, Copy)]
    pub enum LOGCONDITION {
        HasRole(RoleId),
    }

    impl LOGCONDITION {
        pub fn toint(self) -> u64 {
            match self {
                LOGCONDITION::HasRole(_) => 0,
            }
        }

        pub fn fromint(cond_type: u64, ref_id: u64) -> LOGCONDITION {
            match cond_type {
                0 => LOGCONDITION::HasRole(RoleId::from(ref_id)),
                _ => panic!("invalid int"),
            }
        }
    }
}
