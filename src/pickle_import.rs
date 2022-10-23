use serde::Deserialize;
use serde_pickle as pickle;
use serde_pickle::value::{HashableValue, Value};
use serenity::model::id::{ChannelId, GuildId, MessageId, RoleId, UserId};
use std::collections::BTreeMap;
use std::fs::File;

use crate::{database::Database, LOGCONDITION, LOGTRIGGER, PERMISSION};

/* Data structure
 * guild data (tuple - data, roles)
 * > data (dict)
 *   > roleLogAdd (dict) - Role detection only - sends messages when someone gains a role
 *     > roleID (tuple - channelID, message, restrictions)
 *       > channelID (int)
 *       > message (formattable string)
 *       > restrictions (dict)
 *         > hasRole (set)
 *           > roleID
 *           > ...
 *         > notHasRole (set)
 *           > roleID
 *           > ...
 *     > roleID
 *       > ...
 *   > roleLogRemove (dict) - Role detection only - sends messages when someone loses a role
 *     > ... - Identical to roleLogAdd
 *   > fastping (set) - allow only these roles to bypass the cooldown
 *     > discord role ID
 *     > ...
 *   > restrictping (set) - allows only these roles to ping
 *     > discord role ID
 *     > ...
 *   > restrictproposal (set) - allows only these roles to propose lists
 *     > discord role ID
 *     > ...
 *   > channelRestrictions (dict)
 *     > membership (set) - blacklist; join, leave
 *     > mentioning (set) - blacklist; ping
 *     > information (set) - blacklist; get, list
 *     > proposals (set) - blacklist; propose, listProposals
 *   > pingdelay (int) - cooldown used for per list and global ping timeout
 *   > proposals (dict)
 *     > messageID (tuple)
 *       > name (string)
 *       > channelID (channel ID in which the proposal is happening)
 *       > timestamp (int)
 *       > transferData (dict)
 *         > See roles.groupName.roleData
 *   > proposalTimeout
 *   > proposalThreshold
 * > roles (dict)
 *   > groupName (tuple - roleData, members)
 *     > roleData (dict)
 *       > restricted (bool)
 *       > noping (bool)
 *       > pingdelay (float)
 *       > description (string)
 *     > members (set)
 *       > userID
 *       > ...
*/

pub fn import_pickled(ipath: &str, gid: GuildId, database: &mut Database) {
    //-> Result<Database, Error> {
    let file = File::open(ipath).unwrap();

    let deserialized = pickle::value_from_reader(&file, Default::default()).unwrap();

    // None => (),
    // Bool(b) => (b),
    // I64(int) => (),
    // Int(bint) => (),
    // F64(fp) => (),
    // Bytes(bytes) => (),
    // String(string) => (),
    // List(list) => (),
    // Tuple(tuple) => (),
    // Set(set) => (),
    // FrozenSet(fset) => (),
    // Dict(pairslist) => (),

    fn db_parse_roleLogSingle(
        db: &mut Database,
        guild_id: GuildId,
        trigger: LOGTRIGGER,
        content_val: &Vec<Value>,
    ) {
        match &content_val[..] {
            [Value::I64(channel_id), Value::String(message), Value::Dict(restrictions_map)] => {
                let log_id = db
                    .add_response(
                        GuildId::from(guild_id),
                        trigger,
                        ChannelId::from(*channel_id as u64),
                        message,
                    )
                    .unwrap();
                for (restr_key_val, restr_ids_val) in restrictions_map {
                    if let HashableValue::String(restr_key) = restr_key_val {
                        match (restr_key.as_str(), restr_ids_val) {
                            (hr, Value::Set(restr_ids)) => {
                                for restr_role_id_val in restr_ids {
                                    if let HashableValue::I64(restr_role_id) = restr_role_id_val {
                                        db.add_response_condition(
                                            log_id,
                                            LOGCONDITION::HasRole(RoleId(*restr_role_id as u64)),
                                            hr == "notHasRole",
                                        )
                                        .unwrap();
                                    } else {
                                        panic!("err");
                                    }
                                }
                            }
                            _ => panic!("err"),
                        }
                    } else {
                        panic!("err");
                    }
                }
            }
            _ => panic!("err"),
        }
    }

    fn db_parse_roleLogAdd(db: &mut Database, guild_id: GuildId, content_val: &Value) {
        if let Value::Dict(outer_dict) = content_val {
            for (role_id_val, info_val) in outer_dict {
                match (role_id_val, info_val) {
                    (HashableValue::I64(role_id), Value::Tuple(info)) => db_parse_roleLogSingle(
                        db,
                        guild_id,
                        LOGTRIGGER::RoleAdd(RoleId(*role_id as u64)),
                        &info,
                    ),
                    _ => panic!("err"),
                }
            }
        } else {
            panic!("err");
        }
    }

    fn db_parse_roleLogRemove(db: &mut Database, guild_id: GuildId, content_val: &Value) {
        if let Value::Dict(outer_dict) = content_val {
            for (role_id_val, info_val) in outer_dict {
                match (role_id_val, info_val) {
                    (HashableValue::I64(role_id), Value::Tuple(info)) => db_parse_roleLogSingle(
                        db,
                        guild_id,
                        LOGTRIGGER::RoleRemove(RoleId(*role_id as u64)),
                        &info,
                    ),
                    _ => panic!("err"),
                }
            }
        } else {
            panic!("err");
        }
    }

    fn db_parse_fastping(db: &mut Database, guild_id: GuildId, content_val: &Value) {
        if let Value::Set(roleids) = content_val {
            for role_id_val in roleids {
                if let HashableValue::I64(role_id) = role_id_val {
                    //TODO: fix guildids, allow importing over db
                    db.set_role_ignore_cooldown(guild_id, RoleId(*role_id as u64), true)
                        .unwrap();
                } else {
                    panic!("err");
                }
            }
        } else {
            panic!("err");
        }
    }

    fn db_parse_restrictping(db: &mut Database, guild_id: GuildId, content_val: &Value) {
        db.set_guild_canping(guild_id, false);
        if let Value::Set(roleids) = content_val {
            for role_id_val in roleids {
                if let HashableValue::I64(role_id) = role_id_val {
                    //TODO: fix guildids, allow importing over db
                    db.set_role_canping(guild_id, RoleId(*role_id as u64), PERMISSION::ALLOW)
                        .unwrap();
                } else {
                    panic!("err");
                }
            }
        } else {
            panic!("err");
        }
    }

    fn db_parse_restrictproposal(db: &mut Database, guild_id: GuildId, content_val: &Value) {
        db.set_guild_canpropose(guild_id, false);
        if let Value::Set(roleids) = content_val {
            for role_id_val in roleids {
                if let HashableValue::I64(role_id) = role_id_val {
                    db.set_role_canpropose(guild_id, RoleId(*role_id as u64), PERMISSION::ALLOW)
                        .unwrap();
                } else {
                    panic!("err");
                }
            }
        } else {
            panic!("err");
        }
    }

    fn db_parse_channelrestrictions(db: &mut Database, guild_id: GuildId, content_val: &Value) {
        if let Value::Dict(tags) = content_val {
            for (key_val, set_val) in tags {
                if let HashableValue::String(key) = key_val {
                    match (key.as_str(), set_val) {
                        ("mentioning", Value::Set(cids)) => {
                            for cid_val in cids {
                                if let HashableValue::I64(cid) = cid_val {
                                    db.set_channel_mentioning(
                                        ChannelId(*cid as u64),
                                        PERMISSION::DENY,
                                    )
                                    .unwrap();
                                } else {
                                    panic!("err");
                                }
                            }
                        }
                        ("membership", _) => (), // These are ephemeral, no longer needed
                        ("information", _) => (),
                        ("proposals", Value::Set(cids)) => {
                            for cid_val in cids {
                                if let HashableValue::I64(cid) = cid_val {
                                    db.set_channel_proposing(
                                        ChannelId(*cid as u64),
                                        PERMISSION::DENY,
                                    )
                                    .unwrap();
                                } else {
                                    panic!("err");
                                }
                            }
                        }
                        _ => panic!("err"),
                    }
                } else {
                    panic!("err");
                }
            }
        } else {
            panic!("err");
        }
    }

    database.add_guild(gid);

    let (data, lists): (
        &BTreeMap<HashableValue, Value>,
        &BTreeMap<HashableValue, Value>,
    );
    if let Value::Tuple(base) = deserialized {
        if let [dval, lval] = &base[..] {
            if let Value::Dict(dres) = dval {
                for (dkey, dvalue) in dres.into_iter() {
                    if let HashableValue::String(key) = dkey {
                        match key.as_str() {
                            "roleLogAdd" => db_parse_roleLogAdd(database, gid, dvalue),
                            "roleLogRemove" => db_parse_roleLogRemove(database, gid, dvalue),
                            "fastping" => db_parse_fastping(database, gid, dvalue),
                            "restrictping" => db_parse_restrictping(database, gid, dvalue),
                            "restrictproposal" => db_parse_restrictproposal(database, gid, dvalue),
                            "channelRestrictions" => {
                                db_parse_channelrestrictions(database, gid, dvalue)
                            }
                            "pingdelay" => println!("found pingdelay {:?}", dvalue),
                            "proposals" => println!("found proposals {:?}", dvalue),
                            "proposalTimeout" => println!("found proposalTimeout {:?}", dvalue),
                            "proposalThreshold" => println!("found proposalThreshold {:?}", dvalue),
                            _ => println!("Unmatched key {}", dkey),
                        }
                    }
                }
            }
            if let Value::Dict(lres) = lval {
                for (dkey, dvalue) in lres.into_iter() {
                    if let HashableValue::String(s) = dkey {
                        // println!("found list {} with val {:?}", s, dvalue);
                    }
                }
            }
        }
    }

    // Ok(database)
}
