pub mod data_access {
    use crate::structures::structures::{ListId, PingList, LOGCONDITION, LOGTRIGGER, PERMISSION};
    use rusqlite::{named_params, params, Connection, Error, Result};
    use serenity::model::id::*;

    pub struct Database {
        db: Connection,
    }

    impl Database {
        // ANCHOR Initialization
        pub fn new(database_path: String) -> Result<Database, Error> {
            let conn = Connection::open(database_path)?;

            let mut database = Database { db: conn };
            database.init_tables()?;

            Ok(database)
        }

        fn init_tables(&mut self) -> Result<(), Error> {
            let statement = "PRAGMA foreign_keys = ON;\n\
            CREATE TABLE IF NOT EXISTS guilds ( \
                id                  INTEGER PRIMARY KEY NOT NULL, \
                general_cooldown    INTEGER DEFAULT 0 CHECK( general_cooldown = 0 OR general_cooldown = 1 ), \
                general_canping     INTEGER DEFAULT 1 CHECK( general_canping = 0 OR general_canping = 1 ) , \
                pingcooldown        INTEGER DEFAULT 1800 CHECK( pingcooldown > 0 ), \
                general_propose     INTEGER DEFAULT 1 CHECK( general_propose = 0 OR general_propose = 1 ), \
                propose_threshold   INTEGER DEFAULT 8 CHECK( propose_threshold > 0 ), \
                propose_timeout     INTEGER DEFAULT 86400 CHECK( propose_timeout > 2 ), \
                log_channel         INTEGER DEFAULT -1 );\n\
            CREATE TABLE IF NOT EXISTS alias ( \
                id                  INTEGER PRIMARY KEY ASC, \
                list_id             INTEGER REFERENCES lists(id), \
                name                TEXT NOT NULL, \
                UNIQUE(list_id, name) );\n\
            CREATE TABLE IF NOT EXISTS memberships ( \
                id                  INTEGER PRIMARY KEY ASC, \
                user_id             INTEGER NOT NULL, \
                list_id             INTEGER NOT NULL REFERENCES lists(id));\n\
            CREATE TABLE IF NOT EXISTS lists ( \
                id                  INTEGER PRIMARY KEY ASC, \
                guild_id            INTEGER REFERENCES guilds(id), \
                description         TEXT, \
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
                timestamp           INTEGER NOT NULL );";
            self.db.execute_batch(statement)
        }

        //ANCHOR Guild setup
        fn has_guild(&self, guild_id: GuildId) -> bool {
            self.db
                .query_row(
                    "SELECT EXISTS (SELECT id FROM guilds WHERE id=?1)",
                    params![guild_id.as_u64()],
                    |row| match row.get(0).expect("No value in row from guild exist query") {
                        1 => Ok(true),
                        _ => Ok(false),
                    },
                )
                .expect("Unexpected database error when checking guild existance")
        }

        pub fn add_guild(&mut self, id: GuildId) -> Result<(), Error> {
            if self.has_guild(id) {
                return Ok(());
            }
            self.db
                .execute("INSERT INTO guilds (id) VALUES (?)", [id.as_u64()])?;
            Ok(())
        }

        pub fn get_guild_ping_data(&self, guild_id: GuildId) -> (bool, bool, u64) {
            self.db
                .query_row(
                    "SELECT general_cooldown, general_canping, pingcooldown FROM guilds WHERE id=?1",
                    params![guild_id.as_u64()],
                    |row| Ok(
                    (row.get::<usize, bool>(0)?,
                    row.get::<usize, bool>(1)?,
                    row.get::<usize, u64>(2)?))
                )
                .unwrap()
        }

        pub fn set_guild_canping(&mut self, guild_id: GuildId, value: bool) -> Result<(), Error> {
            self.db.execute(
                "UPDATE guilds SET general_canping = ?1 WHERE id = ?2",
                params![value, guild_id.as_u64()],
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
                params![value, guild_id.as_u64()],
            )?;
            Ok(())
        }

        pub fn set_guild_ping_cooldown(
            &mut self,
            guild_id: GuildId,
            value: u64,
        ) -> Result<(), Error> {
            self.db.execute(
                "UPDATE guilds SET pingcooldown = ?1 WHERE id = ?2",
                params![value, guild_id.as_u64()],
            )?;
            Ok(())
        }

        //ANCHOR List functions
        pub fn add_list(
            &mut self,
            guild_id: GuildId,
            name: &String,
            description: String,
        ) -> Result<ListId, Error> {
            self.db.execute(
                "INSERT INTO lists (guild_id, description) VALUES (?1, ?2)",
                params![guild_id.as_u64(), description],
            )?;
            let list_id = self.db.last_insert_rowid() as u64;
            self.add_alias(list_id, name.as_str())?;
            Ok(list_id)
        }

        //List config
        pub fn set_pingable(&mut self, list_id: ListId, pingable: bool) -> Result<(), Error> {
            self.db.execute(
                "UPDATE lists SET ping_permission = ?1 WHERE id = ?2",
                params![pingable, list_id],
            )?;
            Ok(())
        }

        pub fn set_joinable(&mut self, list_id: ListId, joinable: bool) -> Result<(), Error> {
            self.db.execute(
                "UPDATE lists SET join_permission = ?1 WHERE id = ?2",
                params![joinable, list_id],
            )?;
            Ok(())
        }

        pub fn set_visible(&mut self, list_id: ListId, visible: bool) -> Result<(), Error> {
            self.db.execute(
                "UPDATE lists SET visible = ?1 WHERE id = ?2",
                params![visible, list_id],
            )?;
            Ok(())
        }

        pub fn set_description(&mut self, list_id: ListId, value: &String) -> Result<(), Error> {
            self.db.execute(
                "UPDATE lists SET description = ?1 WHERE id = ?2",
                params![value, list_id],
            )?;
            Ok(())
        }
        pub fn set_cooldown(&mut self, list_id: ListId, value: i64) -> Result<(), Error> {
            self.db.execute(
                "UPDATE lists SET cooldown = ?1 WHERE id = ?2",
                params![value, list_id],
            )?;
            Ok(())
        }

        pub fn add_alias(&mut self, list_id: ListId, name: &str) -> Result<(), Error> {
            self.db.execute(
                "INSERT INTO alias (list_id, name) VALUES (?1, ?2)",
                params![list_id, name],
            )?;
            Ok(())
        }

        pub fn remove_alias(&mut self, list_id: ListId, name: &str) -> Result<(), Error> {
            self.db.execute(
                "DELETE FROM alias WHERE list_id = ?1 AND name = ?2",
                params![list_id, name],
            )?;
            Ok(())
        }

        //Getters

        pub fn get_list_permissions(&self, list_id: ListId) -> (i64, bool, bool) {
            self.db
                .query_row(
                    "SELECT cooldown, join_permission, ping_permission FROM lists WHERE id=?1",
                    params![list_id],
                    |row| {
                        Ok((
                            row.get::<usize, i64>(0)?,
                            row.get::<usize, bool>(1)?,
                            row.get::<usize, bool>(2)?,
                        ))
                    },
                )
                .unwrap()
        }

        pub fn get_list_id_by_name(
            &mut self,
            list_name: &str,
            guild_id: GuildId,
        ) -> Result<ListId, Error> {
            let id = self.db.query_row(
                "SELECT lists.id FROM lists, alias WHERE alias.name=?1 AND alias.list_id = lists.id AND lists.guild_id=?2",
                params![list_name, guild_id.as_u64()], |row| row.get::<usize, u64>(0)
            )?;
            Ok(id)
        }

        pub fn get_list_names(
            &mut self,
            list_id: ListId,
            guild_id: GuildId,
        ) -> Result<Vec<String>, Error> {
            let mut stmt = self.db.prepare("SELECT alias.name FROM lists, alias WHERE lists.id=?1 AND lists.guild_id=?2 AND alias.list_id=lists.id")?;
            let mut rows = stmt.query(params![list_id, guild_id.as_u64()])?;
            let mut names = Vec::new();
            while let Some(row) = rows.next()? {
                names.push(row.get::<usize, String>(0)?);
            }
            Ok(names)
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
                named_params! { ":guid": guild_id.as_u64(), ":filter": filter, ":show_hidden": show_hidden },
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
                    named_params! { ":guid": guild_id.as_u64(), ":filter": filter, ":show_hidden": show_hidden},
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
        ) -> Result<Vec<String>, Error> {
            let lists_query = "SELECT alias.name \
                FROM lists, alias \
                WHERE lists.guild_id=:guid \
                AND alias.name LIKE '%' || :filter || '%' \
                AND alias.list_id = lists.id \
                AND (NOT lists.ping_permission = :permissiondeny OR :show_all)
                ORDER BY alias.name ASC \
                LIMIT :start, :amt";
            let mut stmt = self.db.prepare(lists_query)?;
            let mut rows = stmt.query(
                named_params! { ":guid": guild_id.as_u64(), ":filter": filter, ":amt": amount, ":start": start, ":show_all": show_all, ":permissiondeny": PERMISSION::DENY as u64 }
            )?;

            let mut lists = Vec::new();
            while let Some(row) = rows.next()? {
                lists.push(row.get::<usize, String>(0)?);
            }
            Ok(lists)
        }

        pub fn get_list_membership_by_search(
            &mut self,
            guild_id: GuildId,
            user_id: UserId,
            amount: usize,
            filter: &str,
            show_all: bool,
        ) -> Result<Vec<String>, Error> {
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
            let mut stmt = self.db.prepare(lists_query)?;
            let mut rows = stmt.query(
                named_params! { ":guid": guild_id.as_u64(), ":filter": filter, ":amt": amount, ":show_all": show_all, ":permissiondeny": PERMISSION::DENY as u64, ":user": user_id.as_u64() }
            )?;

            let mut lists = Vec::new();
            while let Some(row) = rows.next()? {
                lists.push(row.get::<usize, String>(0)?);
            }
            Ok(lists)
        }

        pub fn get_list_joinable_by_search(
            &mut self,
            guild_id: GuildId,
            user_id: UserId,
            amount: usize,
            filter: &str,
            show_all: bool,
        ) -> Result<Vec<String>, Error> {
            let lists_query = "SELECT alias.name \
                FROM lists, alias \
                WHERE lists.guild_id=:guid \
                AND alias.name LIKE '%' || :filter || '%' \
                AND alias.list_id = lists.id \
                AND (NOT lists.ping_permission = :permissiondeny OR :show_all)
                AND NOT EXISTS ( \
                    SELECT id FROM memberships WHERE \
                    memberships.user_id = :user \
                    AND memberships.list_id = lists.id) \
                ORDER BY alias.name ASC \
                LIMIT 0, :amt";
            let mut stmt = self.db.prepare(lists_query)?;
            let mut rows = stmt.query(
                named_params! { ":guid": guild_id.as_u64(), ":filter": filter, ":amt": amount, ":show_all": show_all, ":permissiondeny": PERMISSION::DENY as u64, ":user": user_id.as_u64() }
            )?;

            let mut lists = Vec::new();
            while let Some(row) = rows.next()? {
                lists.push(row.get::<usize, String>(0)?);
            }
            Ok(lists)
        }
        // List memberships

        pub fn has_member(&mut self, member_id: UserId, list_id: ListId) -> bool {
            self.db
                .query_row(
                    "SELECT EXISTS (SELECT id FROM memberships WHERE user_id=?1 AND list_id=?2)",
                    params![member_id.as_u64(), list_id],
                    |row| match row
                        .get(0)
                        .expect("No value in row from membership exist query")
                    {
                        1 => Ok(true),
                        _ => Ok(false),
                    },
                )
                .expect("Unexpected database error when checking membership existance")
        }

        pub fn get_lists_with_member(
            &mut self,
            guild_id: GuildId,
            member_id: UserId,
        ) -> Result<Vec<u64>, Error> {
            let mut stmt = self.db.prepare("SELECT lists.id FROM lists, memberships WHERE lists.id=memberships.list_id AND memberships.user_id=? AND lists.guild_id=?")?;
            let mut rows = stmt.query(params![member_id.as_u64(), guild_id.as_u64()])?;
            let mut lists = Vec::new();
            while let Some(row) = rows.next()? {
                lists.push(row.get(0)?);
            }
            Ok(lists)
        }

        pub fn get_members_in_list(&mut self, list_id: ListId) -> Result<Vec<u64>, Error> {
            let mut stmt = self.db.prepare("SELECT memberships.user_id FROM lists, memberships WHERE lists.id=memberships.list_id AND memberships.list_id=?")?;
            let mut rows = stmt.query(params![list_id])?;
            let mut lists = Vec::new();
            while let Some(row) = rows.next()? {
                lists.push(row.get(0)?);
            }
            Ok(lists)
        }

        pub fn add_member(&mut self, member_id: UserId, list_id: ListId) -> Result<(), Error> {
            self.db.execute(
                "INSERT INTO memberships (user_id, list_id) VALUES (?1, ?2)",
                params![member_id.as_u64(), list_id],
            )?;
            Ok(())
        }

        pub fn remove_member(&mut self, member_id: UserId, list_id: ListId) -> Result<(), Error> {
            self.db.execute(
                "DELETE FROM memberships WHERE user_id = ?1 AND list_id = ?2",
                params![member_id.as_u64(), list_id],
            )?;
            Ok(())
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
                    params![role_id.as_u64(), guild_id.as_u64()],
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
                [guild_id.as_u64(), role_id.as_u64()],
            )?;
            Ok(())
        }

        pub fn set_role_propose(
            &mut self,
            guild_id: GuildId,
            role_id: RoleId,
            perm: PERMISSION,
        ) -> Result<(), Error> {
            self.ensure_role_present(guild_id, role_id)?;
            self.db.execute(
                "UPDATE role_settings SET propose_permission = ?1 WHERE role_id=?2 AND guild_id=?3",
                params![perm as u64, guild_id.as_u64(), role_id.as_u64()],
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
            self.db.execute(
                "UPDATE role_settings SET ping_permission = ?1 WHERE role_id=?2 AND guild_id=?3",
                params![perm as u64, guild_id.as_u64(), role_id.as_u64()],
            )?;
            Ok(())
        }

        pub fn set_role_cooldown(
            &mut self,
            guild_id: GuildId,
            role_id: RoleId,
            deny: bool,
        ) -> Result<(), Error> {
            self.ensure_role_present(guild_id, role_id)?;
            self.db.execute(
                "UPDATE role_settings SET ignore_gbcooldown = ?1 WHERE role_id=?2 AND guild_id=?3",
                params![deny, guild_id.as_u64(), role_id.as_u64()],
            )?;
            Ok(())
        }

        //ANCHOR user functions

        pub fn get_user_permissions(
            &mut self,
            guild_id: GuildId,
            user_id: UserId,
        ) -> (PERMISSION, PERMISSION, bool) {
            self.ensure_user_present(guild_id, user_id).unwrap();
            self.db
                .query_row(
                    "SELECT propose_permission, ping_permission, ignore_gbcooldown FROM user_settings WHERE user_id=?1 AND guild_id=?2",
                    params![user_id.as_u64(), guild_id.as_u64()],
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

        fn ensure_user_present(&mut self, guild_id: GuildId, user_id: UserId) -> Result<(), Error> {
            self.db.execute(
                "INSERT OR IGNORE INTO user_settings (guild_id, user_id) VALUES (?1, ?2)",
                [guild_id.as_u64(), user_id.as_u64()],
            )?;
            Ok(())
        }

        pub fn set_user_propose(
            &mut self,
            guild_id: GuildId,
            user_id: UserId,
            perm: PERMISSION,
        ) -> Result<(), Error> {
            self.ensure_user_present(guild_id, user_id)?;
            self.db.execute(
                "UPDATE user_settings SET propose_permission = ?1 WHERE user_id=?2 AND guild_id=?3",
                params![perm as u64, guild_id.as_u64(), user_id.as_u64()],
            )?;
            Ok(())
        }

        pub fn set_user_canping(
            &mut self,
            guild_id: GuildId,
            user_id: UserId,
            perm: PERMISSION,
        ) -> Result<(), Error> {
            self.ensure_user_present(guild_id, user_id)?;
            self.db.execute(
                "UPDATE user_settings SET ping_permission = ?1 WHERE user_id=?2 AND guild_id=?3",
                params![perm as u64, guild_id.as_u64(), user_id.as_u64()],
            )?;
            Ok(())
        }

        pub fn set_user_cooldown(
            &mut self,
            guild_id: GuildId,
            user_id: UserId,
            deny: bool,
        ) -> Result<(), Error> {
            self.ensure_user_present(guild_id, user_id)?;
            self.db.execute(
                "UPDATE user_settings SET ignore_gbcooldown = ?1 WHERE user_id=?2 AND guild_id=?3",
                params![deny, guild_id.as_u64(), user_id.as_u64()],
            )?;
            Ok(())
        }

        //ANCHOR channel functions

        fn ensure_channel_present(&mut self, channel_id: ChannelId) -> Result<(), Error> {
            self.db.execute(
                "INSERT OR IGNORE INTO channel_settings (channel_id) VALUES (?1)",
                [channel_id.as_u64()],
            )?;
            Ok(())
        }

        pub fn set_channel_mentioning(
            &mut self,
            channel_id: ChannelId,
            value: PERMISSION,
        ) -> Result<(), Error> {
            self.ensure_channel_present(channel_id)?;
            self.db.execute(
                "UPDATE channel_settings SET override_mentioning = ?1 WHERE channel_id = ?2",
                params![value as u64, channel_id.as_u64()],
            )?;
            Ok(())
        }

        pub fn set_channel_proposing(
            &mut self,
            channel_id: ChannelId,
            value: PERMISSION,
        ) -> Result<(), Error> {
            self.ensure_channel_present(channel_id)?;
            self.db.execute(
                "UPDATE channel_settings SET propose_permission = ?1 WHERE channel_id = ?2",
                params![value as u64, channel_id.as_u64()],
            )?;
            Ok(())
        }

        pub fn set_channel_public_visible(
            &mut self,
            channel_id: ChannelId,
            value: bool,
        ) -> Result<(), Error> {
            self.ensure_channel_present(channel_id)?;
            self.db.execute(
                "UPDATE channel_settings SET public_commands = ?1 WHERE channel_id = ?2",
                params![value, channel_id.as_u64()],
            )?;
            Ok(())
        }

        pub fn get_channel_permissions(
            &mut self,
            _guild_id: GuildId,
            channel_id: ChannelId,
        ) -> (bool, PERMISSION, PERMISSION) {
            self.ensure_channel_present(channel_id).unwrap();
            self.db
                .query_row(
                    "SELECT public_commands, override_mentioning, propose_permission FROM channel_settings WHERE channel_id=?1",
                    params![channel_id.as_u64()],
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

        pub fn set_propose_enabled(&mut self, guild_id: GuildId, value: bool) -> Result<(), Error> {
            self.db.execute(
                "UPDATE guilds SET general_propose = ?1 WHERE id = ?2",
                params![value, guild_id.as_u64()],
            )?;
            Ok(())
        }

        pub fn set_propose_timeout(&mut self, guild_id: GuildId, value: u64) -> Result<(), Error> {
            self.db.execute(
                "UPDATE guilds SET propose_timeout = ?1 WHERE id = ?2",
                params![value, guild_id.as_u64()],
            )?;
            Ok(())
        }

        pub fn set_propose_threshold(
            &mut self,
            guild_id: GuildId,
            value: u64,
        ) -> Result<(), Error> {
            self.db.execute(
                "UPDATE guilds SET propose_threshold = ?1 WHERE id = ?2",
                params![value, guild_id.as_u64()],
            )?;
            Ok(())
        }

        pub fn get_propose_settings(&self, guild_id: GuildId) -> Result<(bool, u64, u64), Error> {
            self.db
                .query_row(
                    "SELECT general_propose, propose_timeout, propose_threshold FROM guilds WHERE id = ?1",
                    params![guild_id.as_u64()],
                    |row| {
                        Ok((
                            row.get::<usize, bool>(0)?,
                            row.get::<usize, u64>(1)?,
                            row.get::<usize, u64>(2)?,
                        ))
                    },
                )
        }

        // Usage functions

        pub fn start_proposal(
            &mut self,
            guild_id: GuildId,
            name: &String,
            description: String,
            timestamp: i64,
        ) -> Result<ListId, Error> {
            // let transaction = self.db.transaction().unwrap();
            let list_id = self.add_list(guild_id, name, description).unwrap();
            self.set_pingable(list_id, false).unwrap();
            self.set_joinable(list_id, false).unwrap();
            self.set_visible(list_id, false).unwrap();
            self.db.execute(
                "INSERT INTO proposals (list_id, timestamp) VALUES (?1, ?2)",
                params![list_id, timestamp],
            )?;
            // transaction.commit();
            Ok(list_id)
        }

        pub fn vote_proposal(&mut self, list_id: ListId, member_id: UserId) -> Result<(), Error> {
            self.add_member(member_id, list_id).unwrap();
            Ok(())
        }

        pub fn get_proposal_votes(&mut self, list_id: ListId) -> usize {
            self.get_members_in_list(list_id).unwrap().len()
        }

        pub fn get_list_guild(&mut self, list_id: ListId) -> Result<GuildId, Error> {
            Ok(GuildId(self.db.query_row(
                "SELECT guild_id FROM lists WHERE id=?1",
                params![list_id],
                |row| row.get::<usize, u64>(0),
            )?))
        }

        pub fn get_vote_threshold(&mut self, guild_id: GuildId) -> Result<usize, Error> {
            self.db.query_row(
                "SELECT propose_threshold FROM guilds WHERE id=?1",
                params![guild_id.as_u64()],
                |row| row.get::<usize, usize>(0),
            )
        }

        pub fn remove_proposal(&mut self, list_id: ListId) -> Result<(), Error> {
            self.db
                .execute("DELETE FROM proposals WHERE list_id = ?1", params![list_id])?;
            Ok(())
        }

        pub fn get_proposals_by_search(
            &mut self,
            guild_id: GuildId,
            start: usize,
            amount: usize,
            filter: &str,
        ) -> Result<Vec<String>, Error> {
            let lists_query = "SELECT alias.name \
                FROM lists, alias, proposals \
                WHERE lists.guild_id=:guid \
                AND alias.name LIKE '%' || :filter || '%' \
                AND alias.list_id = lists.id \
                AND lists.id = proposals.list_id \
                ORDER BY alias.name ASC \
                LIMIT :start, :amt";
            let mut stmt = self.db.prepare(lists_query)?;
            let mut rows = stmt.query(
                named_params! { ":guid": guild_id.as_u64(), ":filter": filter, ":amt": amount, ":start": start }
            )?;

            let mut lists = Vec::new();
            while let Some(row) = rows.next()? {
                lists.push(row.get::<usize, String>(0)?);
            }
            Ok(lists)
        }

        pub fn get_proposals(
            &mut self,
            guild_id: GuildId,
        ) -> Result<Vec<(String, u64, u64)>, Error> {
            let lists_query = "SELECT alias.name, proposals.timestamp, lists.id \
                FROM lists, alias, proposals \
                WHERE lists.guild_id = :guid \
                AND alias.list_id = lists.id \
                AND lists.id = proposals.list_id \
                ORDER BY alias.name ASC";
            let mut stmt = self.db.prepare(lists_query)?;
            let mut rows = stmt.query(named_params! { ":guid": guild_id.as_u64() })?;

            let mut lists = Vec::new();
            while let Some(row) = rows.next()? {
                lists.push((
                    row.get::<usize, String>(0)?,
                    row.get::<usize, u64>(1)?,
                    row.get::<usize, u64>(2)?,
                ));
            }
            Ok(lists)
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
            response_message: String,
        ) -> Result<u64, Error> {
            // Check has response first
            match log_type {
                LOGTRIGGER::JoinServer() => {
                    self.db.execute(
                        "INSERT INTO action_response (guild_id, trigger, response_channel, response_message) VALUES (?1, ?2, ?3, ?4)",
                        params![
                            guild_id.as_u64(),
                            log_type.toint(),
                            response_channel.as_u64(),
                            response_message
                        ],
                    )?;
                }
                LOGTRIGGER::LeaveServer() => {
                    self.db.execute(
                        "INSERT INTO action_response (guild_id, trigger, response_channel, response_message) VALUES (?1, ?2, ?3, ?4)",
                        params![
                            guild_id.as_u64(),
                            log_type.toint(),
                            response_channel.as_u64(),
                            response_message
                        ],
                    )?;
                }
                LOGTRIGGER::RoleAdd(role_id) => {
                    self.db.execute(
                        "INSERT INTO action_response (guild_id, trigger, trigger_id, response_channel, response_message) VALUES (?1, ?2, ?3, ?4, ?5)",
                        params![
                            guild_id.as_u64(),
                            log_type.toint(),
                            role_id.as_u64(),
                            response_channel.as_u64(),
                            response_message
                        ],
                    )?;
                }
                LOGTRIGGER::RoleRemove(role_id) => {
                    self.db.execute(
                        "INSERT INTO action_response (guild_id, trigger, trigger_id, response_channel, response_message) VALUES (?1, ?2, ?3, ?4, ?5)",
                        params![
                            guild_id.as_u64(),
                            log_type.toint(),
                            role_id.as_u64(),
                            response_channel.as_u64(),
                            response_message
                        ],
                    )?;
                }
            };
            Ok(self.db.last_insert_rowid() as u64)
        }

        pub fn has_response(
            &mut self,
            guild_id: GuildId,
            log_type: LOGTRIGGER,
        ) -> Result<Option<u64>, Error> {
            match log_type {
                LOGTRIGGER::RoleAdd(role_id) | LOGTRIGGER::RoleRemove(role_id) => {
                    match self.db
                        .query_row(
                            "SELECT id FROM action_response WHERE guild_id = ?1 AND trigger = ?2 AND trigger_id = ?3",
                            params![guild_id.as_u64(), log_type.toint(), role_id.as_u64()],
                            |row| row.get(0)
                        )
                    {
                        Ok(id) => Ok(Some(id)),
                        Result::Err(Error::QueryReturnedNoRows) => Ok(None),
                        Result::Err(er) => Err(er)
                    }
                },
                LOGTRIGGER::JoinServer() | LOGTRIGGER::LeaveServer() => {
                    match self.db
                    .query_row(
                        "SELECT id FROM action_response WHERE guild_id = ?1 AND trigger = ?2",
                        params![guild_id.as_u64(), log_type.toint()],
                        |row| row.get(0)
                    )
                {
                    Ok(id) => Ok(Some(id)),
                    Result::Err(Error::QueryReturnedNoRows) => Ok(None),
                    Result::Err(er) => Err(er)
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
                params![guild_id.as_u64(), log_id],
                |row| Ok(
                    (ChannelId( row.get::<usize, u64>(0)?),
                    row.get::<usize, String>(0)?,)
            ))
        }

        pub fn delete_response(
            &mut self,
            guild_id: GuildId,
            log_type: LOGTRIGGER,
            response_channel: ChannelId,
        ) -> Result<(), Error> {
            self.db.execute(
                "DELETE FROM action_response WHERE guild_id = ?1 AND trigger = ?2 AND response_channel = ?3",
                params![
                    guild_id.as_u64(),
                    log_type.toint(),
                    response_channel.as_u64(),
                ],
            )?;
            Ok(())
        }

        // conditions
        pub fn add_response_condition(
            &mut self,
            log_id: u64,
            log_type: LOGCONDITION,
            invert: bool,
        ) -> Result<(), Error> {
            match log_type {
                LOGCONDITION::HasRole(role_id) => {
                    self.db.execute(
                        "INSERT INTO action_response_condition (rolelogID, type, acomp_id, invert) VALUES (?1, ?2, ?3, ?4)",
                        params![
                            log_id,
                            log_type.toint(),
                            role_id.as_u64(),
                            invert
                        ],
                    )?;
                }
            }
            Ok(())
        }

        pub fn remove_response_condition(&mut self, condition_id: u64) -> Result<(), Error> {
            self.db.execute(
                "DELETE FROM action_response_condition WHERE id = ?1",
                params![condition_id,],
            )?;
            Ok(())
        }

        pub fn get_response_conditions(
            &self,
            log_id: u64,
        ) -> Result<Vec<(LOGCONDITION, bool, u64)>, Error> {
            let mut stmt = self.db.prepare(
                "SELECT type, acomp_id, invert, id FROM action_response_condition WHERE rolelogID = ?1",
            )?;
            let mut rows = stmt.query(params![log_id])?;
            let mut conditions = Vec::new();
            while let Some(row) = rows.next()? {
                let (logtype, id) = (row.get::<usize, u64>(0)?, row.get::<usize, u64>(1)?);
                let cond = LOGCONDITION::fromint(logtype, id);
                conditions.push((cond, row.get::<usize, bool>(2)?, row.get::<usize, u64>(3)?));
            }
            Ok(conditions)
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
                    params![cid.as_u64(), guild_id.as_u64()],
                )?;
            } else {
                self.db.execute(
                    "UPDATE guilds SET log_channel = -1 WHERE id = ?2",
                    params![guild_id.as_u64()],
                )?;
            }
            Ok(())
        }

        pub fn get_log_channel(&self, guild_id: GuildId) -> Result<Option<ChannelId>, Error> {
            let cid = self.db.query_row(
                "SELECT log_channel FROM guilds WHERE id = ?1",
                params![guild_id.as_u64()],
                |row| Ok(row.get::<usize, i64>(0)?),
            );
            match cid {
                Ok(-1) => Ok(None),
                Ok(a) => Ok(Some(ChannelId::from(a as u64))),
                Err(a) => Err(a),
            }
        }
    }
}
