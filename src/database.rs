use rusqlite::{Connection, Result};
use rusqlite::NO_PARAMS;

/* SQL setup
 *
 * TABLE memberships
 *      id integer primary key
 *      userID integer not null
 *      listID integer not null references lists(id)
 * 
 * TABLE lists
 *      id integer primary key
 *      guildID integer references guilds(id)
 *      name varchar(255) not null
 *      description varchar(2048)
 *      size integer not null
 *      cooldown integer
 *      restricted_join integer default 0
 *      restricted_ping integer default 0
 *      visible         integer default 1
 * 
 * TABLE guilds
 *      id integer primary key
 *      pingcooldown integer
 *      proposalThreshold integer default 8
 *      proposalTimeout integer default 86400
 * 
 * TABLE roleExceptions
 *      id integer primary key
 *      guildID integer not null references guilds(id)
 *      roleID integer not null
 *      canPropose integer default 1
 *      canPing integer default 1
 *      bypassCooldown integer default 0
 * 
 * TABLE channelRestrictions
 *      channelID integer primary key
 *      information         ? maybe not, if silent
 *      membership          ? maybe not, if silent
 *      mentioning integer default 0
 *      proposals integer default 0
 * 
 * TABLE rolelog
 *      id integer primary key
 *      roleID integer not null
 *      type integer
 *      channelID integer
 *      message text
 *      
 * TABLE rolelogrestrictions
 *      id integer primary key
 *      rolelogID integer not null references rolelog(id)
 *      type integer
 *      acomp_id integer
 * 
 * TABLE proposals
 *      messageID integer primary key
 *      guildID integer not null references guilds(id)
 *      channelID integer not null
 *      timestamp integer not null
 *      listID integer not null references lists(id)
 */


pub mod data_access {

    pub struct Database {
        abc: i32,
    }
    
    impl Database {
        
        pub fn new(database_path: String) -> Database {
            Database {
                abc: 1
            }
        }

        fn add_list(&mut self, name: String) {

            println!("Hello!");
        }
    }
}