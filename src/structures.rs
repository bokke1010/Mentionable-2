pub mod structures {

    use serenity::model::id::{GuildId, RoleId};
    use std::cmp::max;
    use std::fmt;

    pub type ListId = u64;

    pub struct PingList {
        pub id: u64,
        pub guild_id: GuildId,
        pub description: String,
        pub cooldown: i64,
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

    impl fmt::Display for PERMISSION {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(
                f,
                "{}",
                match self {
                    PERMISSION::NEUTRAL => "Neutral",
                    PERMISSION::DENY => "Deny",
                    PERMISSION::ALLOW => "Allow",
                }
            )
        }
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

        pub fn from_str(value: &str) -> Result<PERMISSION, &str> {
            match value {
                "0" | "NEUTRAL" => Ok(PERMISSION::NEUTRAL),
                "1" | "DENY" => Ok(PERMISSION::NEUTRAL),
                "2" | "ALLOW" => Ok(PERMISSION::NEUTRAL),
                _ => Err("Invalid string"),
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
