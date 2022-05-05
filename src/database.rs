
/* SQL setup
 *
 * TABLE memberships
 *      id integer primary key
 *      user_id integer not null
 *      list_id integer not null references lists(id)
 * 
 * TABLE lists
 *      id integer primary key
 *      guild_id integer references guilds(id)
 *      name varchar(255) not null
 *      description varchar(2048)
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
 *      list_id integer not null references lists(id)
 */


pub mod data_access {
    use rusqlite::{Connection, Result, Error, params};

    pub struct Database {
        db: Connection,
    }
    
    impl Database {
        
        pub fn new(database_path: String) -> Result<Database, Error> {
            let conn = Connection::open(database_path)?;
            
            let mut database = Database {
                db: conn
            };
            
            database.init_tables()?;

            Ok(database)
        }

        fn init_tables(&mut self) -> Result<(), Error> {
            let statement = "PRAGMA foreign_keys = ON;\n\
            CREATE TABLE IF NOT EXISTS guilds ( \
                id                  INTEGER PRIMARY KEY NOT NULL, \
                general_cooldown    INTEGER DEFAULT 1 CHECK( general_cooldown = 0 OR general_cooldown = 1 ), \
                general_canping     INTEGER DEFAULT 1 CHECK( general_canping = 0 OR general_canping = 1 ) , \
                pingcooldown        INTEGER DEFAULT 1800 CHECK( pingcooldown > 0 ), \
                general_propose     INTEGER DEFAULT 1 CHECK( general_propose = 0 OR general_propose = 1 ), \
                propose_threshold   INTEGER DEFAULT 8 CHECK( propose_threshold > 0 ), \
                propose_timeout     INTEGER DEFAULT 86400 CHECK( propose_timeout > 2 ));\n\
            CREATE TABLE IF NOT EXISTS memberships ( \
                id                  INTEGER PRIMARY KEY ASC, \
                user_id             INTEGER NOT NULL, \
                list_id             INTEGER NOT NULL REFERENCES lists(id));\n\
            CREATE TABLE IF NOT EXISTS lists ( \
                id                  INTEGER PRIMARY KEY ASC, \
                guild_id            INTEGER REFERENCES guilds(id), \
                name                TEXT NOT NULL, \
                description         TEXT, \
                cooldown            INTEGER DEFAULT -1 CHECK( cooldown >= -1 AND cooldown <= 1 ), \
                restricted_join     INTEGER DEFAULT 0 CHECK( restricted_join >= -1 AND restricted_join <= 1 ), \
                restricted_ping     INTEGER DEFAULT 0 CHECK( restricted_ping >= -1 AND restricted_ping <= 1 ), \
                visible             INTEGER DEFAULT 1 CHECK( visible = 0 OR visible = 1), \
                UNIQUE (name, guild_id) );";
            self.db.execute_batch(statement)
        }

        fn has_guild(&self, guild_id: u64) -> bool {
            self.db.query_row(
                "SELECT EXISTS (SELECT id FROM guilds WHERE id=?1)",
                params![guild_id],
                |row| match row.get(0).expect("No value in row from guild exist query") {
                    1 => Ok(true),
                    _ => Ok(false),
                }
            ).expect("Unexpected database error when checking guild existance")
        }

        pub fn add_guild(&mut self, id: u64) -> Result<(), Error> {
            if self.has_guild(id) {
                return Ok(());
            }
            self.db.execute("INSERT INTO guilds (id) VALUES (?)", [id])?;
            Ok(())
        }

        pub fn add_list(&mut self, guild_id: u64, name: String, description: String) -> Result<(),Error> {
            self.db.execute("INSERT INTO lists (guild_id, name, description) VALUES (?1, ?2, ?3)", params![guild_id, name, description])?;
            Ok(())
        }

        pub fn list_exists(&mut self, guild_id: u64, name: &str) -> bool {
            self.db.query_row(
                "SELECT EXISTS (SELECT id FROM lists WHERE name=?1 AND guild_id=?2)",
                params![name, guild_id],
                |row| match row.get(0).expect("No value in row from membership exist query") {
                    1 => Ok(true),
                    _ => Ok(false),
                }
            ).expect("Unexpected database error when checking membership existance")
        }

        pub fn has_member(&mut self, member_id: u64, list_id: u64) -> bool {
            self.db.query_row(
                "SELECT EXISTS (SELECT id FROM memberships WHERE user_id=?1 AND list_id=?2)",
                params![member_id, list_id],
                |row| match row.get(0).expect("No value in row from membership exist query") {
                    1 => Ok(true),
                    _ => Ok(false),
                }
            ).expect("Unexpected database error when checking membership existance")
        }

        pub fn get_lists_with_member(&mut self, guild_id: u64, member_id: u64) -> Result<Vec<u64>, Error> {
            let mut stmt = self.db.prepare("SELECT lists.id FROM lists, memberships WHERE lists.id=memberships.list_id AND memberships.user_id=? AND lists.guild_id=?")?;
            let mut rows = stmt.query(params![member_id, guild_id])?;
            println!("running get query");
            let mut lists = Vec::new();
            while let Some(row) = rows.next()? {
                println!("parsing get result {}", row.get::<usize, u64>(0)?);
                lists.push(row.get(0)?);
            }
            Ok(lists)
        }

        pub fn get_members_in_list(&mut self, guild_id: u64, list_id: u64) -> Result<Vec<u64>, Error> {
            let mut stmt = self.db.prepare("SELECT memberships.user_id FROM lists, memberships WHERE lists.id=memberships.list_id AND memberships.list_id=? AND lists.guild_id=?")?;
            let mut rows = stmt.query(params![list_id, guild_id])?;
            let mut lists = Vec::new();
            while let Some(row) = rows.next()? {
                lists.push(row.get(0)?);
            }
            Ok(lists)
        }

        pub fn add_member(&mut self, member_id: u64, list_id: u64) -> Result<(),Error> {
            self.db.execute("INSERT INTO memberships (user_id, list_id) VALUES (?1, ?2)", params![member_id, list_id])?;
            Ok(())
        }

        pub fn remove_member(&mut self, member_id: u64, list_id: u64) -> Result<(),Error> {
            self.db.execute("DELETE FROM memberships WHERE user_id = ?1 AND list_id = ?2", params![member_id, list_id])?;
            Ok(())
        }

        pub fn get_list_id_by_name(&mut self, list_name: &str, guild_id: u64) -> Result<u64,Error> {
            self.db.query_row("SELECT id FROM lists WHERE name=?1 AND guild_id=?2", params![list_name, guild_id], |row| row.get(0))
        }

        pub fn get_list_name_by_id(&mut self, list_id: u64, guild_id: u64) -> Result<String, Error> {
            self.db.query_row("SELECT name FROM lists WHERE id=?1 AND guild_id=?2", params![list_id, guild_id], |row| row.get::<usize, String>(0))
        }
    }
}