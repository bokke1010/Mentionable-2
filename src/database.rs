
/* SQL setup
 *
 * TABLE memberships
 *      id integer primary key
 *      userID integer not null
 *      listID integer not null references lists(id)
 * 
 * TABLE lists
 *      id integer primary key
 *      guild_id integer references guilds(id)
 *      name varchar(255) not null
 *      description varchar(2048)
 *      size integer not null
 *      cooldown integer default -1
 *      restricted_join integer default 0
 *      restricted_ping integer default 0
 *      visible         integer default 1
 * 
 * TABLE guilds
 *      id integer primary key
 *      general_cooldown integer default 1
 *      general_canping integer default 1
 *      pingcooldown integer default 1800
 *      
 * 
 *      general_propose integer default 1
 *      propose_threshold integer default 8
 *      propose_timeout integer default 86400
 * 
 * 
 * TABLE role_settings
 *      id integer primary key
 *      guild_id integer not null references guilds(id)
 *      roleID integer not null
 *      override_propose integer default -1
 *      override_canping integer default -1
 *      override_cooldown integer default -1 (If not set to -1, this value will be the cooldown for this role??)
 * 
 * TABLE channel_settings
 *      channelID integer primary key
 *      public_commands integer default 0
 *      override_information         ? maybe not, if silent
 *      override_membership          ? maybe not, if silent
 *      override_mentioning integer default -1
 *      override_proposals integer default -1
 *
 * TABLE log_role
 *      id integer primary key
 *      roleID integer not null
 *      type integer
 *      channelID integer
 *      message text
 *
 * TABLE log_role_condition
 *      id integer primary key
 *      rolelogID integer not null references rolelog(id)
 *      invert integer default 0 (when set to 1, the situation instead must NOT conform to the condition)
 *      type integer (added by, removed by, added to, removed from)
 *      acomp_id integer
 * 
 * TABLE log_message
 *      id integer
 *      type integer (channel, role, user)
 *      acomp_id integer
 *      destination_channel integer
 * 
 * TABLE proposals
 *      messageID integer primary key
 *      guild_id integer not null references guilds(id)
 *      channelID integer not null
 *      timestamp integer not null
 *      listID integer not null references lists(id)
 */


pub mod data_access {
    use rusqlite::{Connection, Result, Error, params};

    pub struct Database {
        db: Connection,

    }
    
    impl Database {
        
        pub fn new(database_path: String) -> Result<Database, Error> {
            let conn = Connection::open(database_path)?;
            
            Ok(Database {
                db: conn
            })
        }

        fn add_guild(&mut self, id: i32) -> Result<(), Error> {
            self.db.execute("INSERT INTO guilds (id) VALUES (?)", [id])?;
            Ok(())
        }

        fn add_list(&mut self, guild_id: i32, name: String, description: String) -> Result<(),Error> {
            self.db.execute("INSERT INTO lists (guild_id, name, description) VALUES (?1, ?2, ?3)", params![guild_id, name, description])?;
            Ok(())
        }

        fn add_member(&mut self, member_id: i32, list_id: i32) -> Result<(),Error> {
            self.db.execute("INSERT INTO memberships (userID, listID) VALUES (?1, ?2)", params![member_id, list_id])?;
            Ok(())
        }

        fn get_list_id_by_name(&mut self, list_name: String, guild_id: i32) -> Result<i32,Error> {
            self.db.query_row("SELECT id FROM lists WHERE name=?1 AND guild_id=?2", params![list_name, guild_id], |row| row.get(0))
        }
    }
}