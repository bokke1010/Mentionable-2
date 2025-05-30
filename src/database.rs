use crate::structures::{
    JoinResult, ListId, PingList, ProposalStatus, LOGCONDITION, LOGTRIGGER, PERMISSION,
};
use rusqlite::{named_params, params, Connection, Error, OptionalExtension, Result};
use serenity::model::id::*;

pub struct Database {
    db: Connection,
}

impl Database {
    // ANCHOR Initialization
    pub fn new(database_path: &str) -> Database {
        let conn = Connection::open(database_path).expect("Invalid path or SQL open failure");
        let mut database = Database { db: conn };

        match database
            .db
            .query_row("PRAGMA user_version", [], |row| row.get(0))
        {
            Ok(2) => println!("The database was loaded succesfully"),
            Ok(1) => {
                database
                    .db
                    .execute_batch(
                        "PRAGMA user_version = 2; \n\
                        ALTER TABLE proposals ADD channel_id INTEGER NOT NULL DEFAULT 0; \n\
                        ALTER TABLE proposals ADD message_id INTEGER NOT NULL DEFAULT 0;",
                    )
                    .expect("Issue updating database");
                println!("Updating database")
            }
            Ok(0) => {
                database.init_tables();
                println!("Created new database");
            }
            Ok(v) => {
                println!(
                    "Unknown database version {}, likely from a future release, aborting",
                    v
                );
                panic!("Unsupported (likely future) DB version")
            }
            Err(e) => Err(e).unwrap(),
        }

        database
    }

    fn init_tables(&mut self) -> () {
        let statement = "PRAGMA user_version = 2; \n\
            PRAGMA foreign_keys = ON;\n\
            CREATE TABLE IF NOT EXISTS guilds ( \
                id                  INTEGER PRIMARY KEY NOT NULL, \
                general_canping     INTEGER DEFAULT 1 CHECK( general_canping = 0 OR general_canping = 1 ) , \
                general_cooldown    INTEGER DEFAULT 0 CHECK( general_cooldown >= 0 ), \
                pingcooldown        INTEGER DEFAULT 1800 CHECK( pingcooldown >= 0 ), \
                general_propose     INTEGER DEFAULT 1 CHECK( general_propose = 0 OR general_propose = 1 ), \
                propose_threshold   INTEGER DEFAULT 8 CHECK( propose_threshold > 0 ), \
                propose_timeout     INTEGER DEFAULT 86400 CHECK( propose_timeout > 2 ), \
                log_channel         INTEGER DEFAULT 0 );\n\
            CREATE TABLE IF NOT EXISTS alias ( \
                id                  INTEGER PRIMARY KEY ASC, \
                list_id             INTEGER REFERENCES lists(id), \
                name                TEXT NOT NULL, \
                UNIQUE(list_id, name) );\n\
            CREATE TABLE IF NOT EXISTS memberships ( \
                id                  INTEGER PRIMARY KEY ASC, \
                user_id             INTEGER NOT NULL, \
                list_id             INTEGER NOT NULL REFERENCES lists(id), \
                UNIQUE(user_id, list_id) );\n\
            CREATE TABLE IF NOT EXISTS lists ( \
                id                  INTEGER PRIMARY KEY ASC, \
                guild_id            INTEGER REFERENCES guilds(id), \
                description         TEXT    DEFAULT '', \
                cooldown            INTEGER DEFAULT -1 CHECK( cooldown >= -1 ), \
                join_permission     INTEGER DEFAULT 0 CHECK( join_permission >= 0 AND join_permission <= 2 ), \
                ping_permission     INTEGER DEFAULT 0 CHECK( ping_permission >= 0 AND ping_permission <= 2 ), \
                visible             INTEGER DEFAULT 1 CHECK( visible = 0 OR visible = 1));\n\
            CREATE TABLE IF NOT EXISTS role_settings ( \
                id                  INTEGER PRIMARY KEY ASC, \
                guild_id            INTEGER NOT NULL REFERENCES guilds(id), \
                role_id             INTEGER UNIQUE NOT NULL, \
                propose_permission  INTEGER DEFAULT 0 CHECK( propose_permission >= 0 AND propose_permission <= 2), \
                ping_permission     INTEGER DEFAULT 0 CHECK( ping_permission >= 0 AND ping_permission <= 2), \
                ignore_gbcooldown   INTEGER DEFAULT 0 CHECK( ignore_gbcooldown = 0 OR ignore_gbcooldown = 1 ) );\n\
            CREATE TABLE IF NOT EXISTS user_settings ( \
                id                  INTEGER PRIMARY KEY ASC, \
                guild_id            INTEGER NOT NULL REFERENCES guilds(id), \
                user_id             INTEGER UNIQUE NOT NULL, \
                propose_permission  INTEGER DEFAULT 0 CHECK( propose_permission >= 0 AND propose_permission <= 2), \
                ping_permission     INTEGER DEFAULT 0 CHECK( ping_permission >= 0 AND ping_permission <= 2), \
                ignore_gbcooldown   INTEGER DEFAULT 0 CHECK( ignore_gbcooldown = 0 OR ignore_gbcooldown = 1 ) );\n\
            CREATE TABLE IF NOT EXISTS channel_settings ( \
                channel_id          INTEGER PRIMARY KEY, \
                public_commands     INTEGER DEFAULT 0, \
                override_mentioning INTEGER DEFAULT 0, \
                propose_permission  INTEGER DEFAULT 0 );\n\
            CREATE TABLE IF NOT EXISTS action_response ( \
                id                  INTEGER PRIMARY KEY ASC, \
                guild_id            INTEGER REFERENCES guilds(id), \
                trigger             INTEGER NOT NULL, \
                trigger_id          INTEGER NOT NULL DEFAULT 0, \
                response_channel    INTEGER NOT NULL, \
                response_message    TEXT NOT NULL);\n\
            CREATE TABLE IF NOT EXISTS action_response_condition ( \
                id                  INTEGER PRIMARY KEY ASC, \
                rolelogID           INTEGER NOT NULL REFERENCES action_response(id), \
                type                INTEGER DEFAULT 0, \
                acomp_id            INTEGER, \
                invert              INTEGER DEFAULT 0);\n\
            CREATE TABLE IF NOT EXISTS proposals ( \
                list_id             INTEGER PRIMARY KEY REFERENCES lists(id), \
                timestamp           INTEGER NOT NULL, \
                channel_id          INTEGER NOT NULL DEFAULT 0, \
                message_id          INTEGER NOT NULL DEFAULT 0 );";
        self.db.execute_batch(statement).expect("Malformed SQL")
    }

    //ANCHOR Guild setup
    fn has_guild(&self, guild_id: GuildId) -> bool {
        self.db
            .query_row(
                "SELECT EXISTS (SELECT id FROM guilds WHERE id=?1)",
                params![guild_id.get()],
                |row| row.get::<usize, bool>(0),
            )
            .expect("Unexpected database error when checking guild existance")
    }

    pub fn add_guild(&mut self, id: GuildId) -> Result<(), Error> {
        if self.has_guild(id) {
            return Ok(());
        }
        self.db
            .execute("INSERT INTO guilds (id) VALUES (?)", [id.get()])?;
        Ok(())
    }

    pub fn get_guild_ping_data(&self, guild_id: GuildId) -> (u64, bool, u64) {
        self.db
            .query_row(
                "SELECT general_cooldown, general_canping, pingcooldown FROM guilds WHERE id=?1",
                params![guild_id.get()],
                |row| {
                    Ok((
                        row.get::<usize, u64>(0)?,
                        row.get::<usize, bool>(1)?,
                        row.get::<usize, u64>(2)?,
                    ))
                },
            )
            .unwrap()
    }

    pub fn set_guild_canping(&mut self, guild_id: GuildId, value: bool) -> Result<(), Error> {
        self.db.execute(
            "UPDATE guilds SET general_canping = ?1 WHERE id = ?2",
            params![value, guild_id.get()],
        )?;
        Ok(())
    }

    pub fn set_guild_general_cooldown(
        &mut self,
        guild_id: GuildId,
        value: u64,
    ) -> Result<(), Error> {
        self.db.execute(
            "UPDATE guilds SET general_cooldown = ?1 WHERE id = ?2",
            params![value, guild_id.get()],
        )?;
        Ok(())
    }

    pub fn set_guild_ping_cooldown(&mut self, guild_id: GuildId, value: u64) -> Result<(), Error> {
        self.db.execute(
            "UPDATE guilds SET pingcooldown = ?1 WHERE id = ?2",
            params![value, guild_id.get()],
        )?;
        Ok(())
    }

    //ANCHOR List functions
    pub fn add_list(&mut self, guild_id: GuildId, name: &str) -> Option<ListId> {
        let tx = self.db.transaction().unwrap();
        let suc = tx
            .execute(
                "INSERT INTO lists (guild_id) VALUES (?1);",
                params![guild_id.get()],
            )
            .unwrap();
        if suc == 0 {
            tx.finish().unwrap();
            return None;
        }
        let list_id = tx.last_insert_rowid() as u64;
        Database::add_alias(&tx, list_id, name);
        tx.commit().unwrap();
        Some(list_id)
    }

    pub fn remove_list(&mut self, list_id: ListId) -> Result<bool, Error> {
        self.remove_all_alias(list_id)?;
        self.remove_all_members(list_id)?;
        self.remove_proposal(list_id)?;
        Ok(self
            .db
            .execute("DELETE FROM lists WHERE id = ?1", params![list_id])?
            > 0)
    }

    //List config
    pub fn set_pingable(&mut self, list_id: ListId, pingable: PERMISSION) -> bool {
        self.db
            .execute(
                "UPDATE lists SET ping_permission = ?1 WHERE id = ?2",
                params![pingable as u64, list_id],
            )
            .unwrap()
            > 0
    }

    pub fn set_joinable(&mut self, list_id: ListId, joinable: PERMISSION) -> bool {
        self.db
            .execute(
                "UPDATE lists SET join_permission = ?1 WHERE id = ?2",
                params![joinable as u64, list_id],
            )
            .unwrap()
            > 0
    }

    pub fn set_visible(&mut self, list_id: ListId, visible: bool) -> bool {
        self.db
            .execute(
                "UPDATE lists SET visible = ?1 WHERE id = ?2",
                params![visible, list_id],
            )
            .unwrap()
            > 0
    }

    pub fn set_description(&mut self, list_id: ListId, value: &str) -> bool {
        self.db
            .execute(
                "UPDATE lists SET description = ?1 WHERE id = ?2",
                params![value, list_id],
            )
            .unwrap()
            > 0
    }
    pub fn set_cooldown(&mut self, list_id: ListId, value: i64) -> bool {
        self.db
            .execute(
                "UPDATE lists SET cooldown = ?1 WHERE id = ?2",
                params![value, list_id],
            )
            .unwrap()
            > 0
    }

    pub fn add_alias_inline(&self, list_id: ListId, name: &str) -> bool {
        Database::add_alias(&self.db, list_id, name)
    }

    pub fn add_alias(db: &Connection, list_id: ListId, name: &str) -> bool {
        match db.execute(
            "INSERT INTO alias (list_id, name) VALUES (?1, ?2)",
            params![list_id, name],
        ) {
            Err(Error::SqliteFailure(
                rusqlite::ffi::Error {
                    code: _,
                    extended_code: 2067,
                },
                _,
            )) => false, // Unique constraint violation, alias already exists for this list
            Ok(_) => true,
            Err(a) => Err(a).unwrap(),
        }
    }

    pub fn remove_alias(
        &mut self,
        db: Option<&Connection>,
        list_id: ListId,
        name: &str,
    ) -> Result<(), Error> {
        let db = db.unwrap_or(&self.db);
        db.execute(
            "DELETE FROM alias WHERE list_id = ?1 AND name = ?2",
            params![list_id, name],
        )?;
        Ok(())
    }

    fn remove_all_alias(&mut self, list_id: ListId) -> Result<(), Error> {
        self.db
            .execute("DELETE FROM alias WHERE list_id = ?1", params![list_id])?;
        Ok(())
    }

    fn remove_all_members(&mut self, list_id: ListId) -> Result<(), Error> {
        self.db.execute(
            "DELETE FROM memberships WHERE list_id = ?1",
            params![list_id],
        )?;
        Ok(())
    }

    //Getters

    pub fn get_list_permissions(&self, list_id: ListId) -> (i64, PERMISSION, PERMISSION) {
        self.db
            .query_row(
                "SELECT cooldown, join_permission, ping_permission FROM lists WHERE id=?1",
                params![list_id],
                |row| {
                    Ok((
                        row.get::<usize, i64>(0)?,
                        PERMISSION::fromint(row.get::<usize, u64>(1)?),
                        PERMISSION::fromint(row.get::<usize, u64>(2)?),
                    ))
                },
            )
            .unwrap()
    }

    pub fn get_list_id_by_name(&mut self, list_name: &str, guild_id: GuildId) -> Option<ListId> {
        self.db.query_row(
                "SELECT lists.id FROM lists, alias WHERE alias.name=?1 AND alias.list_id = lists.id AND lists.guild_id=?2",
                params![list_name, guild_id.get()], |row| row.get::<usize, u64>(0)
            ).optional().unwrap()
    }

    pub fn get_list_exists(&mut self, list_id: ListId) -> bool {
        self.db
            .query_row(
                "SELECT 1 FROM lists WHERE lists.id=?1",
                params![list_id],
                |row| row.get::<usize, u64>(0),
            )
            .optional()
            .expect("Malformed SQL statement")
            != None
    }

    pub fn get_list_names(&mut self, list_id: ListId) -> Vec<String> {
        let mut stmt = self
            .db
            .prepare(
                "SELECT alias.name FROM lists, alias WHERE lists.id=?1 AND alias.list_id=lists.id",
            )
            .expect("Malformed sql");
        let rows = stmt
            .query_map(params![list_id], |row| row.get::<usize, String>(0))
            .expect("Error binding parameters");

        rows.collect::<Result<Vec<String>, _>>()
            .expect("Element is not string or index is incorrect")
    }

    pub fn get_lists_by_search(
        &mut self,
        guild_id: GuildId,
        filter: &str,
        show_hidden: bool,
    ) -> Result<Vec<PingList>, Error> {
        let lists_query = "SELECT lists.id, lists.description, lists.visible, lists.join_permission, lists.ping_permission, lists.cooldown \
                FROM lists, alias \
                WHERE lists.guild_id=:guid \
                AND alias.name LIKE '%' || :filter || '%' \
                AND alias.list_id = lists.id \
                AND (lists.visible = 1 OR :show_hidden) \
                ORDER BY alias.name ASC";
        let mut stmt = self.db.prepare(lists_query)?;
        let mut rows = stmt.query(
                named_params! { ":guid": guild_id.get(), ":filter": filter, ":show_hidden": show_hidden },
            )?;

        let mut lists = Vec::new();
        while let Some(row) = rows.next()? {
            lists.push(PingList {
                id: row.get::<usize, u64>(0)?,
                guild_id: guild_id,
                description: row.get::<usize, String>(1)?,
                visible: row.get::<usize, bool>(2)?,
                cooldown: row.get::<usize, i64>(5)?,
                join_permission: PERMISSION::fromint(row.get::<usize, u64>(3)?),
                ping_permission: PERMISSION::fromint(row.get::<usize, u64>(4)?),
            });
        }
        Ok(lists)
    }

    pub fn count_lists_by_search(
        &mut self,
        guild_id: GuildId,
        filter: &str,
        show_hidden: bool,
    ) -> usize {
        self.db
                .query_row(
                    "SELECT COUNT(DISTINCT lists.id) \
                    FROM lists, alias \
                    WHERE lists.guild_id=:guid \
                    AND alias.name LIKE '%' || :filter || '%' \
                    AND alias.list_id = lists.id \
                    AND (lists.visible = 1 OR :show_hidden)",
                    named_params! { ":guid": guild_id.get(), ":filter": filter, ":show_hidden": show_hidden},
                    |row| row.get::<usize, usize>(0),
                )
                .unwrap_or(0)
    }

    pub fn get_list_aliases_by_search(
        &mut self,
        guild_id: GuildId,
        start: usize,
        amount: usize,
        filter: &str,
        show_all: bool,
    ) -> Vec<String> {
        let lists_query = "SELECT alias.name \
                FROM lists, alias \
                WHERE lists.guild_id=:guid \
                AND alias.name LIKE '%' || :filter || '%' \
                AND alias.list_id = lists.id \
                AND (NOT lists.ping_permission = :permissiondeny OR :show_all)
                ORDER BY alias.name ASC \
                LIMIT :start, :amt";
        let mut stmt = self.db.prepare(lists_query).unwrap(); // Sql should be correct
        let rows = stmt.query_map(
                named_params! { ":guid": guild_id.get(), ":filter": filter, ":amt": amount, ":start": start, ":show_all": show_all, ":permissiondeny": PERMISSION::DENY as u64 },
                |row| row.get::<usize, String>(0)
            ).unwrap(); // fails if parameters don't bind (sql wrong)

        rows.collect::<Result<Vec<String>, _>>()
            .expect("Element is not string or index is incorrect")
    }

    pub fn get_list_membership_by_search(
        &mut self,
        guild_id: GuildId,
        user_id: UserId,
        amount: usize,
        filter: &str,
        show_all: bool,
    ) -> Vec<String> {
        let lists_query = "SELECT alias.name \
                FROM lists, alias \
                WHERE lists.guild_id=:guid \
                AND alias.name LIKE '%' || :filter || '%' \
                AND alias.list_id = lists.id \
                AND (NOT lists.ping_permission = :permissiondeny OR :show_all)
                AND EXISTS ( \
                    SELECT id FROM memberships WHERE \
                    memberships.user_id = :user \
                    AND memberships.list_id = lists.id) \
                ORDER BY alias.name ASC \
                LIMIT 0, :amt";
        let mut stmt = self
            .db
            .prepare(lists_query)
            .expect("Sql statement malformed");
        let rows = stmt.query_map(
                named_params! { ":guid": guild_id.get(), ":filter": filter, ":amt": amount, ":show_all": show_all, ":permissiondeny": PERMISSION::DENY as u64, ":user": user_id.get() },
                |row| row.get::<usize, String>(0)
            ).expect("Error binding parameters");

        rows.collect::<Result<Vec<String>, _>>()
            .expect("Element is not string or index is incorrect")
    }

    pub fn get_list_joinable_by_search(
        &mut self,
        guild_id: GuildId,
        user_id: UserId,
        amount: usize,
        filter: &str,
        show_all: bool,
    ) -> Vec<String> {
        let lists_query = "SELECT alias.name \
                FROM lists, alias \
                WHERE lists.guild_id=:guid \
                AND alias.name LIKE '%' || :filter || '%' \
                AND alias.list_id = lists.id \
                AND (NOT lists.join_permission = :permissiondeny OR :show_all)
                AND NOT EXISTS ( \
                    SELECT id FROM memberships WHERE \
                    memberships.user_id = :user \
                    AND memberships.list_id = lists.id) \
                ORDER BY alias.name ASC \
                LIMIT 0, :amt";
        let mut stmt = self.db.prepare(lists_query).expect("Sql query malformed");
        let rows = stmt.query_map(
                named_params! { ":guid": guild_id.get(), ":filter": filter, ":amt": amount, ":show_all": show_all, ":permissiondeny": PERMISSION::DENY as u64, ":user": user_id.get() },
                |row| row.get::<usize, String>(0)
            ).expect("Unable to bind parameters to query");

        rows.collect::<Result<Vec<String>, _>>()
            .expect("Element is not string or index is incorrect")
    }
    // List memberships

    pub fn get_lists_with_member(
        &mut self,
        guild_id: GuildId,
        member_id: UserId,
    ) -> Result<Vec<u64>, Error> {
        let mut stmt = self.db.prepare("SELECT lists.id FROM lists, memberships WHERE lists.id=memberships.list_id AND memberships.user_id=? AND lists.guild_id=?")?;
        let mut rows = stmt.query(params![member_id.get(), guild_id.get()])?;
        let mut lists = Vec::new();
        while let Some(row) = rows.next()? {
            lists.push(row.get(0)?);
        }
        Ok(lists)
    }

    pub fn get_members_in_list(&mut self, list_id: ListId) -> Vec<UserId> {
        let mut stmt = self.db.prepare("SELECT memberships.user_id FROM lists, memberships WHERE lists.id=memberships.list_id AND memberships.list_id=?").unwrap();
        let rows = stmt
            .query_map(params![list_id], |row| {
                row.get::<usize, u64>(0).map(|id| UserId::new(id))
            })
            .unwrap();
        rows.collect::<Result<Vec<UserId>, _>>().unwrap()
    }

    pub fn add_member(&mut self, member_id: UserId, list_id: ListId) -> JoinResult {
        let a = self.db.execute(
            "INSERT INTO memberships (user_id, list_id) VALUES (?1, ?2)",
            params![member_id.get(), list_id],
        );
        match a {
            Err(Error::SqliteFailure(
                rusqlite::ffi::Error {
                    code: _,
                    extended_code: 2067, // Constraint violation, already got this membership
                },
                _,
            )) => JoinResult::AlreadyMember, // Already in list
            Err(Error::SqliteFailure(
                rusqlite::ffi::Error {
                    code: _,
                    extended_code: 787,
                },
                _,
            )) => JoinResult::ListDoesNotExist, // list not found
            Err(b) => Err(b).expect("Unexpected sql error"),
            Ok(_) => JoinResult::Succes,
        }
    }

    pub fn remove_member(&mut self, member_id: UserId, list_id: ListId) -> Result<bool, Error> {
        Ok(self.db.execute(
            "DELETE FROM memberships WHERE user_id = ?1 AND list_id = ?2",
            params![member_id.get(), list_id],
        )? > 0)
    }
    //ANCHOR role functions

    pub fn get_role_permissions(
        &mut self,
        guild_id: GuildId,
        role_id: RoleId,
    ) -> (PERMISSION, PERMISSION, bool) {
        self.ensure_role_present(guild_id, role_id).unwrap();
        self.db
                .query_row(
                    "SELECT propose_permission, ping_permission, ignore_gbcooldown FROM role_settings WHERE role_id=?1 AND guild_id=?2",
                    params![role_id.get(), guild_id.get()],
                    |row| {
                        Ok((
                            PERMISSION::fromint(row.get::<usize, u64>(0)?),
                            PERMISSION::fromint(row.get::<usize, u64>(1)?),
                            row.get::<usize, bool>(2)?,
                        ))
                    },
                )
                .unwrap()
    }

    fn ensure_role_present(&mut self, guild_id: GuildId, role_id: RoleId) -> Result<(), Error> {
        self.db.execute(
            "INSERT OR IGNORE INTO role_settings (guild_id, role_id) VALUES (?1, ?2)",
            [guild_id.get(), role_id.get()],
        )?;
        Ok(())
    }

    pub fn set_role_canpropose(
        &mut self,
        guild_id: GuildId,
        role_id: RoleId,
        perm: PERMISSION,
    ) -> Result<(), Error> {
        self.ensure_role_present(guild_id, role_id)?;
        self.db.execute(
            "UPDATE role_settings SET propose_permission = ?1 WHERE role_id=?2 AND guild_id=?3",
            params![perm as u64, role_id.get(), guild_id.get()],
        )?;
        Ok(())
    }

    pub fn set_role_canping(
        &mut self,
        guild_id: GuildId,
        role_id: RoleId,
        perm: PERMISSION,
    ) -> Result<(), Error> {
        self.ensure_role_present(guild_id, role_id)?;
        println!("got here, perm: {}", perm as u64);
        self.db.execute(
            "UPDATE role_settings SET ping_permission = ?1 WHERE role_id=?2 AND guild_id=?3",
            params![perm as u64, role_id.get(), guild_id.get()],
        )?;
        Ok(())
    }

    pub fn set_role_ignore_cooldown(
        &mut self,
        guild_id: GuildId,
        role_id: RoleId,
        deny: bool,
    ) -> Result<(), Error> {
        self.ensure_role_present(guild_id, role_id)?;
        self.db.execute(
            "UPDATE role_settings SET ignore_gbcooldown = ?1 WHERE role_id=?2 AND guild_id=?3",
            params![deny, role_id.get(), guild_id.get()],
        )?;
        Ok(())
    }

    //ANCHOR user functions

    pub fn get_user_permissions(
        &mut self,
        guild_id: GuildId,
        user_id: UserId,
    ) -> (PERMISSION, PERMISSION, bool) {
        self.ensure_user_present(guild_id, user_id);
        self.db
                .query_row(
                    "SELECT propose_permission, ping_permission, ignore_gbcooldown FROM user_settings WHERE user_id=?1 AND guild_id=?2",
                    params![user_id.get(), guild_id.get()],
                    |row| {
                        Ok((
                            PERMISSION::fromint(row.get::<usize, u64>(0)?),
                            PERMISSION::fromint(row.get::<usize, u64>(1)?),
                            row.get::<usize, bool>(2)?,
                        ))
                    },
                )
                .unwrap()
    }

    fn ensure_user_present(&mut self, guild_id: GuildId, user_id: UserId) -> () {
        self.db
            .execute(
                "INSERT OR IGNORE INTO user_settings (guild_id, user_id) VALUES (?1, ?2)",
                [guild_id.get(), user_id.get()],
            )
            .expect("malformed Sql");
    }

    pub fn set_user_propose(&mut self, guild_id: GuildId, user_id: UserId, perm: PERMISSION) -> () {
        self.ensure_user_present(guild_id, user_id);
        self.db
            .execute(
                "UPDATE user_settings SET propose_permission = ?1 WHERE user_id=?2 AND guild_id=?3",
                params![perm as u64, user_id.get(), guild_id.get()],
            )
            .expect("SQL statement malformed or SQL error");
    }

    pub fn set_user_canping(&mut self, guild_id: GuildId, user_id: UserId, perm: PERMISSION) -> () {
        self.ensure_user_present(guild_id, user_id);
        self.db
            .execute(
                "UPDATE user_settings SET ping_permission = ?1 WHERE user_id=?2 AND guild_id=?3",
                params![perm as u64, user_id.get(), guild_id.get()],
            )
            .expect("SQL statement malformed or SQL error");
    }

    pub fn set_user_cooldown(&mut self, guild_id: GuildId, user_id: UserId, deny: bool) -> () {
        self.ensure_user_present(guild_id, user_id);
        self.db
            .execute(
                "UPDATE user_settings SET ignore_gbcooldown = ?1 WHERE user_id=?2 AND guild_id=?3",
                params![deny, user_id.get(), guild_id.get()],
            )
            .expect("SQL statement malformed or SQL error");
    }

    //ANCHOR channel functions

    fn ensure_channel_present(&mut self, channel_id: ChannelId) -> () {
        self.db
            .execute(
                "INSERT OR IGNORE INTO channel_settings (channel_id) VALUES (?1)",
                [channel_id.get()],
            )
            .expect("Malformed SQL or sql error: ");
    }

    pub fn set_channel_mentioning(&mut self, channel_id: ChannelId, value: PERMISSION) -> () {
        self.ensure_channel_present(channel_id);
        self.db
            .execute(
                "UPDATE channel_settings SET override_mentioning = ?1 WHERE channel_id = ?2",
                params![value as u64, channel_id.get()],
            )
            .expect("Malformed SQL or sql error: ");
    }

    pub fn set_channel_proposing(&mut self, channel_id: ChannelId, value: PERMISSION) -> () {
        self.ensure_channel_present(channel_id);
        self.db
            .execute(
                "UPDATE channel_settings SET propose_permission = ?1 WHERE channel_id = ?2",
                params![value as u64, channel_id.get()],
            )
            .expect("Malformed SQL or sql error: ");
    }

    pub fn set_channel_public_visible(&mut self, channel_id: ChannelId, value: bool) -> () {
        self.ensure_channel_present(channel_id);
        self.db
            .execute(
                "UPDATE channel_settings SET public_commands = ?1 WHERE channel_id = ?2",
                params![value, channel_id.get()],
            )
            .expect("Malformed SQL or sql error: ");
    }

    pub fn get_channel_permissions(
        &mut self,
        _guild_id: GuildId,
        channel_id: ChannelId,
    ) -> (bool, PERMISSION, PERMISSION) {
        self.ensure_channel_present(channel_id);
        self.db
                .query_row(
                    "SELECT public_commands, override_mentioning, propose_permission FROM channel_settings WHERE channel_id=?1",
                    params![channel_id.get()],
                    |row| {
                        Ok((
                            row.get::<usize, bool>(0)?,
                            PERMISSION::fromint(row.get::<usize, u64>(1)?),
                            PERMISSION::fromint(row.get::<usize, u64>(2)?),
                        ))
                    },
                )
                .unwrap()
    }

    //ANCHOR proposal functions

    // Configuration

    pub fn set_guild_canpropose(&mut self, guild_id: GuildId, value: bool) -> () {
        self.db
            .execute(
                "UPDATE guilds SET general_propose = ?1 WHERE id = ?2",
                params![value, guild_id.get()],
            )
            .unwrap();
    }

    pub fn set_propose_timeout(&mut self, guild_id: GuildId, value: u64) -> () {
        self.db
            .execute(
                "UPDATE guilds SET propose_timeout = ?1 WHERE id = ?2",
                params![value, guild_id.get()],
            )
            .unwrap();
    }

    pub fn set_propose_threshold(&mut self, guild_id: GuildId, value: u64) -> () {
        self.db
            .execute(
                "UPDATE guilds SET propose_threshold = ?1 WHERE id = ?2",
                params![value, guild_id.get()],
            )
            .unwrap();
    }

    pub fn get_propose_settings(&self, guild_id: GuildId) -> (bool, u64, usize) {
        self.db.query_row(
            "SELECT general_propose, propose_timeout, propose_threshold FROM guilds WHERE id = ?1",
            params![guild_id.get()],
            |row| {
                Ok((
                    row.get::<usize, bool>(0)?,
                    row.get::<usize, u64>(1)?,
                    row.get::<usize, usize>(2)?,
                ))
            },
        ).unwrap()
    }

    // Usage functions

    pub fn start_proposal(
        &mut self,
        guild_id: GuildId,
        name: &str,
        timestamp: i64,
        channel_id: ChannelId,
    ) -> Option<ListId> {
        // let transaction = self.db.transaction().unwrap();
        if let Some(list_id) = self.add_list(guild_id, name) {
            self.set_pingable(list_id, PERMISSION::DENY);
            self.set_joinable(list_id, PERMISSION::DENY);
            self.set_visible(list_id, false);
            self.db
                .execute(
                    "INSERT INTO proposals (list_id, timestamp, channel_id) VALUES (?1, ?2, ?3)",
                    params![list_id, timestamp, channel_id.get()],
                )
                .unwrap();
            return Some(list_id);
        }
        None
        // transaction.commit();
    }
    pub fn complete_proposal(&mut self, list_id: ListId, message_id: MessageId) -> () {
        println!("Completing proposal with message id {}", message_id);
        self.db
            .execute(
                "UPDATE proposals SET message_id = ?1 WHERE list_id=?2",
                params![message_id.get(), list_id],
            )
            .unwrap();
    }

    // pub fn proposal_refe

    pub fn accept_proposal(&mut self, list_id: ListId) -> bool {
        // let transaction = self.db.transaction().unwrap();
        if self.remove_proposal(list_id).is_ok() {
            self.set_pingable(list_id, PERMISSION::NEUTRAL);
            self.set_joinable(list_id, PERMISSION::NEUTRAL);
            self.set_visible(list_id, true);
            true
        } else {
            false
        }
        // transaction.commit();
    }

    pub fn vote_proposal(&mut self, list_id: ListId, member_id: UserId) -> () {
        self.add_member(member_id, list_id);
    }

    pub fn get_proposal_data(&mut self, list_id: ListId) -> ProposalStatus {
        let votes = self.get_members_in_list(list_id).len();
        let timestamp = self
            .db
            .query_row(
                "SELECT timestamp, channel_id, message_id FROM proposals WHERE list_id = ?1",
                params![list_id],
                |row| {
                    Ok((
                        row.get::<usize, u64>(0)?,
                        ChannelId::new(row.get::<usize, u64>(1)?),
                        MessageId::new(row.get::<usize, u64>(2)?),
                    ))
                },
            )
            .optional()
            .unwrap();
        if let Some((timestamp, channel_id, message_id)) = timestamp {
            return ProposalStatus::ACTIVE(list_id, votes, timestamp, channel_id, message_id);
        }
        ProposalStatus::REMOVED
    }

    pub fn get_list_guild(&mut self, list_id: ListId) -> Result<GuildId, Error> {
        Ok(GuildId::new(self.db.query_row(
            "SELECT guild_id FROM lists WHERE id=?1",
            params![list_id],
            |row| row.get::<usize, u64>(0),
        )?))
    }

    pub fn remove_proposal(&mut self, list_id: ListId) -> Result<bool, Error> {
        Ok(self
            .db
            .execute("DELETE FROM proposals WHERE list_id = ?1", params![list_id])?
            > 0)
    }

    pub fn get_proposals(&mut self, guild_id: GuildId) -> Vec<(String, ProposalStatus)> {
        let lists_query = "SELECT alias.name, lists.id, proposals.timestamp, proposals.channel_id, proposals.message_id, ( \
                        SELECT COUNT(memberships.user_id) \
                        FROM memberships \
                        WHERE memberships.list_id = lists.id \
                    )\
                    FROM proposals \
                    INNER JOIN lists ON proposals.list_id=lists.id \
                    INNER JOIN alias ON alias.list_id=lists.id \
                    WHERE lists.guild_id = ?1
                    GROUP BY lists.id";
        let mut stmt = self.db.prepare(lists_query).unwrap();
        stmt.query_map(params![guild_id.get()], |mf| {
            Ok((
                mf.get(0)?,
                ProposalStatus::ACTIVE(
                    mf.get(1)?,                 // name
                    mf.get(5)?,                 // votes
                    mf.get(2)?,                 // timestamp
                    ChannelId::new(mf.get(3)?), // channel id
                    MessageId::new(mf.get(4)?), // message id
                ),
            ))
        })
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap()
    }

    pub fn get_bot_proposals(&mut self) -> Vec<(GuildId, ProposalStatus)> {
        let lists_query = "SELECT lists.guild_id, lists.id, proposals.timestamp, proposals.channel_id, proposals.message_id, ( \
                        SELECT COUNT(memberships.user_id) \
                        FROM memberships \
                        WHERE memberships.list_id = lists.id \
                    )\
                    FROM proposals \
                    INNER JOIN lists ON proposals.list_id=lists.id";
        let mut stmt = self.db.prepare(lists_query).unwrap();
        stmt.query_map(params![], |mf| {
            Ok((
                GuildId::new(mf.get(0)?),
                ProposalStatus::ACTIVE(
                    mf.get(1)?,                 // name
                    mf.get(5)?,                 // votes
                    mf.get(2)?,                 // timestamp
                    ChannelId::new(mf.get(3)?), // channel id
                    MessageId::new(mf.get(4)?), // message id
                ),
            ))
        })
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap()
    }

    //ANCHOR: responding functions

    // CREATE TABLE IF NOT EXISTS action_response ( \
    //     id                  INTEGER PRIMARY KEY ASC, \
    //     guild_id            INTEGER REFERENCES guilds(id), \
    //     trigger             INTEGER NOT NULL, \
    //     trigger_id          INTEGER NOT NULL DEFAULT 0, \
    //     response_channel    INTEGER NOT NULL, \
    //     response_message    TEXT NOT NULL);\n\
    // CREATE TABLE IF NOT EXISTS action_response_condition ( \
    //     id                  INTEGER PRIMARY KEY ASC, \
    //     rolelogID           INTEGER NOT NULL REFERENCES action_response(id), \
    //     type                INTEGER DEFAULT 0, \
    //     acomp_id            INTEGER, \
    //     invert              INTEGER DEFAULT 0);\n\

    pub fn add_response(
        &mut self,
        guild_id: GuildId,
        log_type: LOGTRIGGER,
        response_channel: ChannelId,
        response_message: &str,
    ) -> Result<u64, Error> {
        // Check has response first
        match log_type {
            LOGTRIGGER::JoinServer() => {
                self.db.execute(
                        "INSERT INTO action_response (guild_id, trigger, response_channel, response_message) VALUES (?1, ?2, ?3, ?4)",
                        params![
                            guild_id.get(),
                            log_type.toint(),
                            response_channel.get(),
                            response_message
                        ],
                    )?;
            }
            LOGTRIGGER::RoleAdd(role_id) | LOGTRIGGER::RoleRemove(role_id) => {
                self.db.execute(
                        "INSERT INTO action_response (guild_id, trigger, trigger_id, response_channel, response_message) VALUES (?1, ?2, ?3, ?4, ?5)",
                        params![
                            guild_id.get(),
                            log_type.toint(),
                            role_id.get(),
                            response_channel.get(),
                            response_message
                        ],
                    )?;
            }
        };
        Ok(self.db.last_insert_rowid() as u64)
    }

    pub fn has_response(&mut self, guild_id: GuildId, log_type: LOGTRIGGER) -> Option<u64> {
        match log_type {
                LOGTRIGGER::RoleAdd(role_id) | LOGTRIGGER::RoleRemove(role_id) => {
                    match self.db
                        .query_row(
                            "SELECT id FROM action_response WHERE guild_id = ?1 AND trigger = ?2 AND trigger_id = ?3",
                            params![guild_id.get(), log_type.toint(), role_id.get()],
                            |row| row.get(0)
                        )
                    {
                        Ok(id) => Some(id),
                        Result::Err(Error::QueryReturnedNoRows) => None,
                        Result::Err(er) => Err(er).expect("SQL malformed or runtime error: ")
                    }
                },
                LOGTRIGGER::JoinServer() => {
                    match self.db
                    .query_row(
                        "SELECT id FROM action_response WHERE guild_id = ?1 AND trigger = ?2",
                        params![guild_id.get(), log_type.toint()],
                        |row| row.get(0)
                    )
                {
                    Ok(id) => Some(id),
                    Result::Err(Error::QueryReturnedNoRows) => None,
                    Result::Err(er) => Err(er).expect("SQL malformed or runtime error: ")
                }
                }
            }
    }

    pub fn get_response(
        &self,
        guild_id: GuildId,
        log_id: u64,
    ) -> Result<(ChannelId, String), Error> {
        self.db.query_row(
                "SELECT response_channel, response_message FROM action_response WHERE guild_id = ?1 AND id = ?2",
                params![guild_id.get(), log_id],
                |row| Ok(
                    (ChannelId::new( row.get::<usize, u64>(0)?),
                    row.get::<usize, String>(1)?)
            ))
    }

    pub fn get_all_responses(
        &self,
        guild_id: GuildId,
    ) -> Result<Vec<(ChannelId, String, LOGTRIGGER)>, Error> {
        let responses_query = "SELECT response_channel, response_message, trigger, trigger_id \
                FROM action_response \
                WHERE guild_id = ?1";
        let mut stmt = self.db.prepare(responses_query)?;
        let mut rows = stmt.query(params![guild_id.get()])?;

        let mut responses = Vec::new();
        while let Some(row) = rows.next()? {
            responses.push((
                ChannelId::new(row.get::<usize, u64>(0)?),
                row.get::<usize, String>(1)?,
                LOGTRIGGER::fromint(row.get::<usize, u64>(2)?, row.get::<usize, u64>(3)?),
            ));
        }
        Ok(responses)
    }

    pub fn remove_response(
        &mut self,
        guild_id: GuildId,
        log_type: LOGTRIGGER,
    ) -> Result<bool, Error> {
        match log_type {
            LOGTRIGGER::RoleAdd(role_id) | LOGTRIGGER::RoleRemove(role_id) => {
                Ok(self.db.execute(
                    "DELETE FROM action_response WHERE guild_id = ?1 AND trigger = ?2 AND trigger_id = ?3",
                    params![guild_id.get(), log_type.toint(), role_id.get()],
                )? > 0)
            }
            LOGTRIGGER::JoinServer() => {
                Ok(self.db.execute(
                    "DELETE FROM action_response WHERE guild_id = ?1 AND trigger = ?2",
                    params![guild_id.get(), log_type.toint()],
                )? > 0)
            }
        }
    }

    // conditions
    pub fn add_response_condition(
        &mut self,
        log_id: u64,
        log_type: LOGCONDITION,
        invert: bool,
    ) -> () {
        match log_type {
            LOGCONDITION::HasRole(role_id) => {
                self.db.execute(
                        "INSERT INTO action_response_condition (rolelogID, type, acomp_id, invert) VALUES (?1, ?2, ?3, ?4)",
                        params![
                            log_id,
                            log_type.toint(),
                            role_id.get(),
                            invert
                        ],
                    ).expect("Invalid SQL or sql error: ");
            }
        }
    }

    pub fn remove_response_condition(&mut self, condition_id: u64) -> () {
        self.db
            .execute(
                "DELETE FROM action_response_condition WHERE id = ?1",
                params![condition_id,],
            )
            .expect("Invalid SQL or sql error: ");
    }

    pub fn get_response_conditions(&self, log_id: u64) -> Vec<(LOGCONDITION, bool, u64)> {
        let mut stmt = self.db.prepare(
            "SELECT type, acomp_id, invert, id FROM action_response_condition WHERE rolelogID = ?1",
        ).expect("Malformed SQL");
        let mut rows = stmt
            .query(params![log_id])
            .expect("Failed to bind parameters");
        let mut conditions = Vec::new();
        while let Some(row) = rows.next().expect("Error aquiring next row") {
            let (logtype, id) = (
                row.get::<usize, u64>(0).unwrap(),
                row.get::<usize, u64>(1).unwrap(),
            );
            let cond = LOGCONDITION::fromint(logtype, id);
            conditions.push((
                cond,
                row.get::<usize, bool>(2).unwrap(),
                row.get::<usize, u64>(3).unwrap(),
            ));
        }
        conditions
    }
    //ANCHOR: log purge functions

    pub fn set_log_channel(
        &mut self,
        guild_id: GuildId,
        channel_id: Option<ChannelId>,
    ) -> Result<(), Error> {
        if let Some(cid) = channel_id {
            self.db.execute(
                "UPDATE guilds SET log_channel = ?1 WHERE id = ?2",
                params![cid.get(), guild_id.get()],
            )?;
        } else {
            self.db.execute(
                "UPDATE guilds SET log_channel = -1 WHERE id = ?2",
                params![guild_id.get()],
            )?;
        }
        Ok(())
    }

    pub fn get_log_channel(&self, guild_id: GuildId) -> Result<Option<ChannelId>, Error> {
        let cid = self.db.query_row(
            "SELECT log_channel FROM guilds WHERE id = ?1",
            params![guild_id.get()],
            |row| Ok(row.get::<usize, u64>(0)?),
        );
        match cid {
            Ok(0) => Ok(None),
            Ok(a) => Ok(Some(ChannelId::from(a as u64))),
            Err(a) => Err(a),
        }
    }
}
