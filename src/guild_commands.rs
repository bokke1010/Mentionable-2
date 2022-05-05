
pub mod guild_commands {

    use serenity::{
        model::{
            id::GuildId,
            interactions::{
                application_command::{
                    ApplicationCommand,
                    ApplicationCommandOptionType,
                    // ApplicationCommandType,
                },
            },
        },
        prelude::*,
    };


    pub async fn add_all_application_commands(gid: &mut GuildId, ctx: Context) -> Vec<ApplicationCommand> {
        match gid.set_application_commands(&ctx.http, |commands| {
            commands
                .create_application_command(|command| { //ANCHOR ping command
                    command
                        .name("ping")
                        .description("Pings all given lists")
                        .create_option(|option| {
                            option
                                .name("list")
                                .description("A single pinglist")
                                .kind(ApplicationCommandOptionType::String)
                                .required(true)
                        })
                        .create_option(|option| {
                            option
                                .name("l2")
                                .description("A single pinglist")
                                .kind(ApplicationCommandOptionType::String)
                                .required(false)
                        })
                        .create_option(|option| {
                            option
                                .name("l3")
                                .description("A single pinglist")
                                .kind(ApplicationCommandOptionType::String)
                                .required(false)
                        })
                        .create_option(|option| {
                            option
                                .name("l4")
                                .description("A single pinglist")
                                .kind(ApplicationCommandOptionType::String)
                                .required(false)
                        })
                        .create_option(|option| {
                            option
                                .name("l5")
                                .description("A single pinglist")
                                .kind(ApplicationCommandOptionType::String)
                                .required(false)
                        })
                })
                // .create_application_command(|command| { //ANCHOR ping context
                //     command
                //         .name("ping")
                //         .kind(ApplicationCommandType::Message)
                // })
                .create_application_command(|command| {
                    command
                        .name("create")
                        .description("Adds a list")
                        .create_option(|option| {
                            option
                                .name("name")
                                .description("the name to give this new list")
                                .kind(ApplicationCommandOptionType::String)
                                .required(true)
                        })
                })
                .create_application_command(|command| {
                    command
                        .name("join")
                        .description("Join all given lists")
                        .create_option(|option| {
                            option
                                .name("list")
                                .description("A single pinglist")
                                .kind(ApplicationCommandOptionType::String)
                                .required(true)
                        })
                        .create_option(|option| {
                            option
                                .name("l2")
                                .description("A single pinglist")
                                .kind(ApplicationCommandOptionType::String)
                                .required(false)
                        })
                        .create_option(|option| {
                            option
                                .name("l3")
                                .description("A single pinglist")
                                .kind(ApplicationCommandOptionType::String)
                                .required(false)
                        })
                        .create_option(|option| {
                            option
                                .name("l4")
                                .description("A single pinglist")
                                .kind(ApplicationCommandOptionType::String)
                                .required(false)
                        })
                        .create_option(|option| {
                            option
                                .name("l5")
                                .description("A single pinglist")
                                .kind(ApplicationCommandOptionType::String)
                                .required(false)
                        })
                })
                .create_application_command(|command| {
                    command
                        .name("leave")
                        .description("Leave all given lists")
                        .create_option(|option| {
                            option
                                .name("list")
                                .description("A single pinglist")
                                .kind(ApplicationCommandOptionType::String)
                                .required(true)
                        })
                        .create_option(|option| {
                            option
                                .name("l2")
                                .description("A single pinglist")
                                .kind(ApplicationCommandOptionType::String)
                                .required(false)
                        })
                        .create_option(|option| {
                            option
                                .name("l3")
                                .description("A single pinglist")
                                .kind(ApplicationCommandOptionType::String)
                                .required(false)
                        })
                        .create_option(|option| {
                            option
                                .name("l4")
                                .description("A single pinglist")
                                .kind(ApplicationCommandOptionType::String)
                                .required(false)
                        })
                        .create_option(|option| {
                            option
                                .name("l5")
                                .description("A single pinglist")
                                .kind(ApplicationCommandOptionType::String)
                                .required(false)
                        })
                })
                .create_application_command(|command| {
                    command
                        .name("get")
                        .description("Get all lists you're subscribed to")
                })
                .create_application_command(|command| {
                    command
                        .name("list")
                        .description("List all lists")
                        .create_option(|option| {
                            option
                                .name("page")
                                .description("What part of the list you want to see")
                                .kind(ApplicationCommandOptionType::Integer)
                                .required(false)
                        }).create_option(|option| {
                            option
                                .name("filter")
                                .description("Require this to be present in the name or description of a list.")
                                .kind(ApplicationCommandOptionType::String)
                                .required(false)
                        })
                })
                .create_application_command(|command| { // Only if proposals are enabled
                    command
                        .name("propose")
                        .description("Propose a lists")
                        .create_option(|option| {
                            option
                                .name("name")
                                .description("The proposed name for this ping list")
                                .kind(ApplicationCommandOptionType::String)
                                .required(true)
                        })
                })
                .create_application_command(|command| {
                    command
                        .name("list_proposals")
                        .description("See proposed lists")
                })
                .create_application_command(|command| { // mod only
                    command
                        .name("add")
                        .description("Add a user to lists")
                        .create_option(|option| {
                            option
                                .name("user")
                                .description("The user to add")
                                .kind(ApplicationCommandOptionType::User)
                                .required(true)
                        })
                        .create_option(|option| {
                            option
                                .name("list")
                                .description("A single pinglist")
                                .kind(ApplicationCommandOptionType::String)
                                .required(true)
                        })
                        .create_option(|option| {
                            option
                                .name("l2")
                                .description("A single pinglist")
                                .kind(ApplicationCommandOptionType::String)
                                .required(false)
                        })
                        .create_option(|option| {
                            option
                                .name("l3")
                                .description("A single pinglist")
                                .kind(ApplicationCommandOptionType::String)
                                .required(false)
                        })
                        .create_option(|option| {
                            option
                                .name("l4")
                                .description("A single pinglist")
                                .kind(ApplicationCommandOptionType::String)
                                .required(false)
                        })
                        .create_option(|option| {
                            option
                                .name("l5")
                                .description("A single pinglist")
                                .kind(ApplicationCommandOptionType::String)
                                .required(false)
                        })
                })
                .create_application_command(|command| { // mod only
                    command
                        .name("kick")
                        .description("Kick a user from lists")
                        .create_option(|option| {
                            option
                                .name("user")
                                .description("The user to remove")
                                .kind(ApplicationCommandOptionType::User)
                                .required(true)
                        })
                        .create_option(|option| {
                            option
                                .name("list")
                                .description("A single pinglist")
                                .kind(ApplicationCommandOptionType::String)
                                .required(true)
                        })
                        .create_option(|option| {
                            option
                                .name("l2")
                                .description("A single pinglist")
                                .kind(ApplicationCommandOptionType::String)
                                .required(false)
                        })
                        .create_option(|option| {
                            option
                                .name("l3")
                                .description("A single pinglist")
                                .kind(ApplicationCommandOptionType::String)
                                .required(false)
                        })
                        .create_option(|option| {
                            option
                                .name("l4")
                                .description("A single pinglist")
                                .kind(ApplicationCommandOptionType::String)
                                .required(false)
                        })
                        .create_option(|option| {
                            option
                                .name("l5")
                                .description("A single pinglist")
                                .kind(ApplicationCommandOptionType::String)
                                .required(false)
                        })
                })
                .create_application_command(|command| { // mod only
                    command
                        .name("rename")
                        .description("Rename a list")
                        .create_option(|option| {
                            option
                                .name("current")
                                .description("The current name")
                                .kind(ApplicationCommandOptionType::String)
                                .required(true)
                        })
                        .create_option(|option| {
                            option
                                .name("new")
                                .description("The new name for this list")
                                .kind(ApplicationCommandOptionType::String)
                                .required(true)
                        })
                })
                .create_application_command(|command| { // mod only
                    command
                        .name("configure")
                        .description("Houses various configuration subcommands")
                        .create_option(|catagory| {
                            catagory
                                .name("show_configuration").description("Shows all the current settings in a neat embed.")
                                .kind(ApplicationCommandOptionType::SubCommand)
                        })
                        .create_option(|catagory| {
                            catagory
                            .name("globalcooldown").description("a")
                            .kind(ApplicationCommandOptionType::SubCommand)
                            .create_sub_option(|option| {
                                option
                                    .name("enable")
                                    .description("Enable or disable the cooldown.")
                                    .kind(ApplicationCommandOptionType::Boolean)
                                    .required(false)
                            })
                            .create_sub_option(|option| {
                                option
                                    .name("exclude")
                                    .description("Toggle whether or not people with this role can ignore this cooldown.")
                                    .kind(ApplicationCommandOptionType::Role)
                                    .required(false)
                            })
                        })
                        .create_option(|catagory| {
                            catagory.name("pingcooldown").description("c")
                            .kind(ApplicationCommandOptionType::SubCommand)
                            .create_sub_option(|option| {
                                option
                                    .name("cooldown")
                                    .description("The cooldown duration in seconds. Enter a negative number to reset the cooldown.")
                                    .kind(ApplicationCommandOptionType::Integer)
                                    .required(true)
                            })
                            .create_sub_option(|option| {
                                option
                                    .name("list")
                                    .description("The list to which this cooldown applies. Leave empty to set the default cooldown")
                                    .kind(ApplicationCommandOptionType::String)
                                    .required(false)
                            })
                        })
                        .create_option(|catagory| {
                            catagory.name("pingrestrictions").description("b")
                            .kind(ApplicationCommandOptionType::SubCommand)
                            .create_sub_option(|option| {
                                option
                                    .name("enable")
                                    .description("Enable or disable the restriction.")
                                    .kind(ApplicationCommandOptionType::Boolean)
                                    .required(false)
                            })
                            .create_sub_option(|option| {
                                option
                                    .name("exclude")
                                    .description("Toggle whether or not people with this role can use /ping.")
                                    .kind(ApplicationCommandOptionType::Role)
                                    .required(false)
                            })
                        })
                        .create_option(|catagory| {
                            catagory.name("listconfigure").description("d")
                            .kind(ApplicationCommandOptionType::SubCommand)
                            .create_sub_option(|option| {
                                option
                                    .name("allow_join")
                                    .description("Allow anyone to join or leave this list.")
                                    .kind(ApplicationCommandOptionType::Boolean)
                                    .required(false)
                            })
                            .create_sub_option(|option| {
                                option
                                    .name("allow_ping")
                                    .description("Allow anyone to mention this list.")
                                    .kind(ApplicationCommandOptionType::Boolean)
                                    .required(false)
                            })
                            .create_sub_option(|option| {
                                option
                                    .name("description")
                                    .description("Set the list description.")
                                    .kind(ApplicationCommandOptionType::String)
                                    .required(false)
                            })
                        })
                        .create_option(|catagory| {
                            catagory.name("channelblacklist").description("Allows you to restrict usage of certain commands in certain channels.")
                            .kind(ApplicationCommandOptionType::SubCommand)
                            .create_sub_option(|option| {
                                option
                                    .name("channel")
                                    .description("Toggles whether or not a channel is on the blacklist")
                                    .kind(ApplicationCommandOptionType::String)
                                    .required(true)
                            })
                            .create_sub_option(|option| {
                                option
                                    .name("command_type")
                                    .description("What kind of commands should this apply to")
                                    .kind(ApplicationCommandOptionType::String)
                                    .required(true)
                                    .add_string_choice("membership", "membership")
                                    .add_string_choice("mentioning", "mentioning")
                                    .add_string_choice("proposals", "proposals")
                                    .add_string_choice("information", "information")
                            })
                        })
                        .create_option(|catagory| {
                            catagory.name("list_proposals").description("Configure the values used to manage list proposals.")
                            .kind(ApplicationCommandOptionType::SubCommand)
                            .create_sub_option(|option| {
                                option
                                    .name("enabled")
                                    .description("Enable or disable proposals")
                                    .kind(ApplicationCommandOptionType::Boolean)
                                    .required(false)
                            })
                            .create_sub_option(|option| {
                                option
                                    .name("timeout")
                                    .description("Set the timeout for a proposal in seconds")
                                    .kind(ApplicationCommandOptionType::Integer)
                                    .required(false)
                            })
                            .create_sub_option(|option| {
                                option
                                    .name("threshold")
                                    .description("Set the amount of votes required to accept a proposal")
                                    .kind(ApplicationCommandOptionType::Integer)
                                    .required(false)
                            })
                        })
                })
        }).await {
            Ok(result) => result,
            Err(error) => panic!("Problem assembling commands: {:?}", error),
        }
    }
}
