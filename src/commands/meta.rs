use crate::keys::ShardManagerContainer;
use crate::Uptime;
use log::error;
use serenity::client::bridge::gateway::ShardId;
use serenity::framework::standard::macros::command;
use serenity::framework::standard::{Args, CommandError, CommandResult};
use serenity::model::prelude::*;
use serenity::prelude::*;
use std::process::id;
use chrono::DateTime;
use chrono::Utc;
use timeago;

use tokio::process::Command;

struct Timer {
    start: DateTime<Utc>,
}

impl Timer {
    pub fn new() -> Self {
        Timer { start: Utc::now() }
    }

    pub fn elapsed_ms(&self) -> i64 {
        Utc::now()
            .signed_duration_since(self.start)
            .num_milliseconds()
    }
}

#[command]
#[aliases(presence, a)]
#[description("Edit the bot's presence. Use the `listen`, `play`, or `reset` subcommands to set the respective activity.")]
#[owners_only]
#[sub_commands(activity_listen, activity_play, activity_stream, activity_reset)]
async fn activity(ctx: &mut Context, msg: &Message) -> CommandResult {
    // Send error message if no subcommands were matched.
    msg.channel_id.say(&ctx.http, "Invalid activity!").await?;

    Ok(())
}

#[command("listen")]
async fn activity_listen(ctx: &mut Context, msg: &Message, args: Args) -> CommandResult {
    let activity = Activity::listening(args.rest());
    ctx.set_activity(activity).await;

    msg.channel_id
        .say(&ctx.http, format!("Now listening to `{:#?}`", args.rest()))
        .await?;

    Ok(())
}

#[command("play")]
async fn activity_play(ctx: &mut Context, msg: &Message, args: Args) -> CommandResult {
    let activity = Activity::playing(args.rest());
    ctx.set_activity(activity).await;

    msg.channel_id
        .say(&ctx.http, format!("Now playing `{:#?}`", args.rest()))
        .await?;

    Ok(())
}

#[command("stream")]
async fn activity_stream(ctx: &mut Context, msg: &Message, args: Args) -> CommandResult {
    let stream_url: &str = "https://twitch.tv/smallant1"; 
    // random streamer i like i guess^^^^?
    let activity = Activity::streaming(args.rest(), stream_url);
    ctx.set_activity(activity).await;

    msg.channel_id
        .say(&ctx.http, format!("Now streaming `{:#?}`", args.rest()))
        .await?;

    Ok(())
}

#[command("reset")]
async fn activity_reset(ctx: &mut Context, msg: &Message) -> CommandResult {
    ctx.reset_presence().await;

    msg.channel_id
        .say(&ctx.http, "Reset presence successully!")
        .await?;

    Ok(())
}

#[command]
#[description("Invite the bot to a server.")]
async fn invite(ctx: &mut Context, msg: &Message) -> CommandResult {
    // Create invite URL using the bot's user ID.
    let url = format!("Invite URL: <https://discordapp.com/oauth2/authorize?&client_id={}&scope=bot&permissions=0>", ctx.cache.read().await.user.id);

    msg.channel_id.say(&ctx.http, url).await?;

    Ok(())
}

#[command]
#[aliases(nick)]
#[description("Edit the bot's nickname on a server. Pass no arguments to reset nickname.")]
#[only_in(guilds)]
#[owners_only]
async fn nickname(ctx: &mut Context, msg: &Message, args: Args) -> CommandResult {
    if let Some(guild) = msg.guild_id {
        // Reset nickname if no args given.
        let name = if args.is_empty() {
            None
        } else {
            Some(args.message())
        };

        if let Err(why) = guild.edit_nickname(&ctx.http, name).await {
            error!("Error changing nickname: {:?}", why);
        }
        let fmt = format!("Changed nickname to `{:#?}`", args.message());
        let _ = match msg.channel_id.say(&ctx.http, fmt).await {
            Ok(_) => return Ok(()),
            Err(_) => return Ok(()),
        };
    }

    Ok(())
}

#[command]
#[description("Pings Discord and shows ping time.")]
async fn ping(ctx: &mut Context, msg: &Message) -> CommandResult {
    let timer = Timer::new();

    let sent_msg = match msg.channel_id.say(&ctx.http, "Ping!").await {
        Ok(m) => m,
        Err(_) => return Ok(()),
    };

    let msg_ms = timer.elapsed_ms();


    let data = ctx.data.read().await;
    let shard_manager = match data.get::<ShardManagerContainer>() {
        Some(v) => v,
        None => {
            msg.reply(&ctx, "There was a problem getting the shard manager").await?;

            return Ok(());
        },
    };

    let manager = shard_manager.lock().await;
    let runners = manager.runners.lock().await;

    // Shards are backed by a "shard runner" responsible for processing events
    // over the shard, so we'll get the information about the shard runner for
    // the shard this command was sent over.
    let runner = match runners.get(&ShardId(ctx.shard_id)) {
        Some(runner) => runner,
        None => {
            msg.reply(&ctx,  "No shard found").await?;

            return Ok(());
        },
    };

    let runner_latency_ms = runner.latency.map(|x| {
        format!(
            "{:.3}",
            x.as_secs() as f64 / 1000.0 + f64::from(x.subsec_nanos()) * 1e-6
        )
    });

    let _ = sent_msg.clone().edit(&ctx, |m| {
        m.content(&format!(
            "Pong! \n\
            API latency: `{} ms`\n\
            Shard latency: `{} ms`\n",
            msg_ms,
            runner_latency_ms.clone().unwrap_or("(shard not found)".into()),
        ))
    }).await?;

    Ok(())
}

#[command]
#[description("Bot stats")]
async fn stats(ctx: &mut Context, msg: &Message) -> CommandResult {
    let pid = id().to_string();
    let cache = &ctx.cache.read().await;

    let bot_version = env!("CARGO_PKG_VERSION");
    let build_number = option_env!("BUILD_BUILDNUMBER");
    let agent_name = option_env!("AGENT_MACHINENAME");
    let agent_id = option_env!("AGENT_ID");

    let full_stdout = Command::new("sh")
        .arg("-c")
        .arg(format!("./full_memory.sh {}", &pid).as_str())
        .output()
        .await
        .expect("failed to execute process");
    let reasonable_stdout = Command::new("sh")
        .arg("-c")
        .arg(format!("./reasonable_memory.sh {}", &pid).as_str())
        .output()
        .await
        .expect("failed to execute process");

    let mut full_mem = String::from_utf8(full_stdout.stdout).unwrap();
    let mut reasonable_mem = String::from_utf8(reasonable_stdout.stdout).unwrap();

    full_mem.pop();
    full_mem.pop();
    reasonable_mem.pop();
    reasonable_mem.pop();

    let (name, discriminator) = match ctx.http.get_current_application_info().await {
        Ok(info) => (info.owner.name, info.owner.discriminator),
        Err(why) => panic!("Could not access application info: {:?}", why),
    };

    let owner_tag = name.to_string() + "#" + &discriminator.to_string();

    let guilds_count = &cache.guilds.len();
    let channels_count = &cache.channels.len();
    let users_count = &cache.users.len();
    let users_count_unique = &cache.users.len();

    let current_time = Utc::now();
    let start_time = {
        let data = ctx.data.read().await;
        match data.get::<Uptime>() {
            Some(val) => *val,
            None => {
                return Err(CommandError::from(
                    "There was a problem getting the shard manager",
                ))
            }
        }
    };

    let mut f = timeago::Formatter::new();
    f.num_items(4);
    f.ago("");

    let uptime_humanized = f.convert_chrono(start_time, current_time);

    let _ = msg.channel_id.send_message(&ctx.http, |m| {
        m.embed(|e| {
            e.color(0x3498db)
                .title(&format!(
                    "maki v{} - build #{} ({} #{})",
                    bot_version,
                    build_number.unwrap_or("N/A"),
                    agent_name.unwrap_or("N/A"),
                    agent_id.unwrap_or("N/A")
                ))
                .url("https://maki.kanbaru.me")
                .field("Author", &owner_tag, true)
                .field("Guilds", &guilds_count.to_string(), true)
                .field("Channels", &channels_count.to_string(), true)
                .field(
                    "Users",
                    &format!(
                        "{} Total\n{} Unique (cached)",
                        users_count, users_count_unique
                    ),
                    true,
                )
                .field("Memory usage", format!("Complete:\n`{} KB`\nBase:\n`{} KB`",
                    &full_mem.parse::<u32>().expect("NaN").to_string(), &reasonable_mem.parse::<u32>().expect("NaN").to_string()), true)

                .field("Bot Uptime", &uptime_humanized, false);
            e
        });
        m
    }).await; 

    Ok(())
}

#[command]
#[aliases(shutdown)]
#[description("Shut down the bot.")]
#[owners_only]
async fn quit(ctx: &mut Context, msg: &Message) -> CommandResult {

    let data = ctx.data.read().await;

    if let Some(manager) = data.get::<ShardManagerContainer>() {
        msg.channel_id.say(&ctx.http, "Shutting down!").await?;

        // Shut down all shards.
        manager.lock().await.shutdown_all().await;
    } else {
        error!("There was a problem getting the shard manager.");
    }

    Ok(())
}
